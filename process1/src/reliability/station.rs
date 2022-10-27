#![allow(clippy::type_complexity)]
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;

pub(super) struct ReliStation {
    t_name: RefCell<HashMap<String, Rc<dyn ReStation>>>, // key: name, value: station
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
    pub(super) fn new() -> ReliStation {
        ReliStation {
            t_name: RefCell::new(HashMap::new()),
            t_kind: RefCell::new(HashMap::new()),
        }
    }

    pub(super) fn station_register(
        &self,
        name: &str,
        kind: ReStationKind,
        station: Rc<dyn ReStation>,
    ) {
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

    pub(super) fn input_rebuild(&self) {
        for (_, station) in self.t_name.borrow().iter() {
            station.input_rebuild();
        }
    }

    pub(super) fn db_compensate(
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

    pub(super) fn db_map(&self) {
        // level 1
        for station in self.get_kind(ReStationKind::Level1).iter() {
            station.db_map();
        }

        // level 2
        for station in self.get_kind(ReStationKind::Level2).iter() {
            station.db_map();
        }
    }

    pub(super) fn make_consistent(
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

    pub(super) fn clear(&self) {
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
    fn db_map(&self) {}
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
    /// release external connection, and clear all entrys optionally.
    fn entry_clear(&self) {}
}
