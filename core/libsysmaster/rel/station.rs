// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;

pub struct ReliStation {
    t_name: RefCell<HashMap<String, Rc<dyn ReStation>>>, // key: name, value: station
    #[allow(clippy::type_complexity)]
    t_kind: RefCell<HashMap<ReStationKind, HashMap<String, Rc<dyn ReStation>>>>, // key: kind, value: stations
}

impl fmt::Debug for ReliStation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReliStation")
            .field("stations.len", &self.t_name.borrow().len())
            .finish()
    }
}

impl ReliStation {
    pub fn new() -> ReliStation {
        ReliStation {
            t_name: RefCell::new(HashMap::new()),
            t_kind: RefCell::new(HashMap::new()),
        }
    }

    pub fn station_register(&self, name: &str, kind: ReStationKind, station: Rc<dyn ReStation>) {
        let sta = Rc::clone(&station);
        if self
            .t_name
            .borrow_mut()
            .insert(name.to_string(), station)
            .is_none()
        {
            // new, update kind-table
            let mut stations = self.t_kind.borrow_mut().remove(&kind).unwrap_or_default();
            stations.insert(name.to_string(), sta);
            self.t_kind.borrow_mut().insert(kind, stations);
        }
    }

    pub fn input_rebuild(&self) {
        for (_, station) in self.t_name.borrow().iter() {
            station.input_rebuild();
        }
    }

    pub fn db_compensate(
        &self,
        lframe: Option<(u32, Option<u32>, Option<u32>)>,
        lunit: Option<String>,
    ) {
        // history first, dealing with history records.
        for (_, station) in self.t_name.borrow().iter() {
            station.db_compensate_history();
        }

        // last second, dealing with last words, which could produce history records.
        if let Some(lf) = lframe {
            let lu = lunit.as_ref();
            for (_, station) in self.t_name.borrow().iter() {
                station.db_compensate_last(lf, lu);
            }
        }
    }

    pub fn db_map(&self, reload: bool) {
        // level 1
        for station in self.get_kind(ReStationKind::Level1).iter() {
            station.db_map(reload);
        }

        // level 2
        for station in self.get_kind(ReStationKind::Level2).iter() {
            station.db_map(reload);
        }
    }

    pub fn db_insert(&self) {
        // level 1
        for station in self.get_kind(ReStationKind::Level1).iter() {
            station.db_insert();
        }

        // level 2
        for station in self.get_kind(ReStationKind::Level2).iter() {
            station.db_insert();
        }
    }

    pub fn make_consistent(
        &self,
        lframe: Option<(u32, Option<u32>, Option<u32>)>,
        lunit: Option<String>,
    ) {
        let lu = lunit.as_ref();

        // last first, dealing with more untrusted information.
        if let Some(lf) = lframe {
            for (_, station) in self.t_name.borrow().iter() {
                station.do_compensate_last(lf, lu);
            }
        }

        // other secondj, dealing with less untrusted information.
        for (_, station) in self.t_name.borrow().iter() {
            station.do_compensate_others(lu);
        }
    }

    pub fn clear(&self) {
        self.t_name.borrow_mut().clear();
        self.t_kind.borrow_mut().clear();
    }

    fn get_kind(&self, kind: ReStationKind) -> Vec<Rc<dyn ReStation>> {
        self.t_kind
            .borrow()
            .get(&kind)
            .cloned()
            .unwrap_or_default()
            .into_values()
            .collect::<_>()
    }
}

/// reliability station kind
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ReStationKind {
    /// priority 1
    Level1,
    /// priority 2
    Level2,
}

/// the behavior of reliability station
pub trait ReStation {
    // input
    /// rebuild the input
    fn input_rebuild(&self) {}

    // compensate: process reentrant
    /// compensate database for the last information
    fn db_compensate_last(
        &self,
        _lframe: (u32, Option<u32>, Option<u32>),
        _lunit: Option<&String>,
    ) {
    }
    /// compensate database for the history information
    fn db_compensate_history(&self) {}
    /// do the compensation for the last information
    fn do_compensate_last(
        &self,
        _lframe: (u32, Option<u32>, Option<u32>),
        _lunit: Option<&String>,
    ) {
    }
    /// do the compensation for the history information excluding the last
    fn do_compensate_others(&self, _lunit: Option<&String>) {}

    // data
    /// map data from database
    /// If reload is true, determine whether the configuration needs to be reloaded based on the situation.
    fn db_map(&self, _reload: bool) {}
    /// insert itself to database
    fn db_insert(&self) {}
    /// update itself to database
    fn db_update(&self) {
        self.db_insert();
    }

    // reload
    /// setup external connections
    fn register_ex(&self) {}
    /// rebuild external connections
    fn entry_coldplug(&self) {}
    /// release external connection, and clear all entries optionally.
    fn entry_clear(&self) {}
}
