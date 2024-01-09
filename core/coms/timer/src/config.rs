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
#![allow(non_snake_case)]
use crate::{
    comm::TimerUnitComm,
    rentry::{CalendarSpec, SectionTimer, TimerBase, TimerValue},
};
use basic::time::USEC_INFINITY;
use core::error::*;
use core::rel::ReStation;
use std::{cell::RefCell, path::PathBuf, rc::Rc};
use unit_parser::prelude::UnitConfig;

#[derive(Default, Clone)]
pub struct UnitRef {
    source: String,
    target: String,
}

impl UnitRef {
    ///
    pub fn new() -> Self {
        UnitRef {
            source: String::new(),
            target: String::new(),
        }
    }

    ///
    pub fn set_ref(&mut self, source: String, target: String) {
        self.source = source;
        self.target = target;
    }

    ///
    pub fn target(&self) -> String {
        self.target.clone()
    }
}

//
#[derive(UnitConfig, Default, Debug)]
pub(crate) struct TimerConfigData {
    #[section(must)]
    pub Timer: SectionTimer,
}

impl TimerConfigData {
    pub(self) fn new(Timer: SectionTimer) -> TimerConfigData {
        TimerConfigData { Timer }
    }

    pub(self) fn set_property(&mut self, key: &str, value: &str) -> Result<(), core::error::Error> {
        self.Timer.set_property(key, value)
    }
}

pub struct TimerConfig {
    // associated objects
    comm: Rc<TimerUnitComm>,

    // owned objects
    /* original */
    data: Rc<RefCell<TimerConfigData>>,
    /* processed */
    timerunitref: RefCell<UnitRef>,
    pub value: RefCell<Vec<TimerValue>>,
}

impl ReStation for TimerConfig {
    // no input, no compensate

    // data
    fn db_map(&self, reload: bool) {
        if reload {
            return;
        }
        if let Some((data, unit)) = self.comm.rentry_conf_get() {
            // TImerConfigData
            self.data.replace(TimerConfigData::new(data));

            // UnitRef
            self.set_unit_ref(unit);
        }
    }

    fn db_insert(&self) {
        self.comm
            .rentry_conf_insert(&self.data.borrow().Timer, self.unit_ref_target());
    }
}

impl TimerConfig {
    pub(super) fn new(commr: &Rc<TimerUnitComm>) -> Self {
        TimerConfig {
            comm: Rc::clone(commr),
            data: Rc::new(RefCell::new(TimerConfigData::default())),
            timerunitref: RefCell::new(UnitRef::new()),
            value: RefCell::new(Vec::new()),
        }
    }

    pub(super) fn reset(&self) {
        self.data.replace(TimerConfigData::default());
        self.timerunitref.replace(UnitRef::new());
        self.db_update();
    }

    fn load_value(&self) {
        let timerconf = self.data.borrow().Timer.clone();
        let mut timervalue = self.value.borrow_mut();
        if timerconf.OnCalendar != USEC_INFINITY {
            let v = TimerValue::new(
                TimerBase::Calendar,
                false,
                timerconf.OnCalendar,
                CalendarSpec::default(),
                0,
            );
            timervalue.push(v)
        }

        if timerconf.OnActiveSec != USEC_INFINITY {
            let v = TimerValue::new(
                TimerBase::Active,
                false,
                timerconf.OnActiveSec,
                CalendarSpec::default(),
                0,
            );
            timervalue.push(v)
        }

        if timerconf.OnBootSec != USEC_INFINITY {
            let v = TimerValue::new(
                TimerBase::Boot,
                false,
                timerconf.OnBootSec,
                CalendarSpec::default(),
                0,
            );
            timervalue.push(v)
        }

        if timerconf.OnStartupSec != USEC_INFINITY {
            let v = TimerValue::new(
                TimerBase::Startup,
                false,
                timerconf.OnStartupSec,
                CalendarSpec::default(),
                0,
            );
            timervalue.push(v)
        }

        if timerconf.OnUnitActiveSec != USEC_INFINITY {
            let v = TimerValue::new(
                TimerBase::UnitActive,
                false,
                timerconf.OnUnitActiveSec,
                CalendarSpec::default(),
                0,
            );
            timervalue.push(v)
        }

        if timerconf.OnUnitInactiveSec != USEC_INFINITY {
            let v = TimerValue::new(
                TimerBase::UnitInactive,
                false,
                timerconf.OnUnitInactiveSec,
                CalendarSpec::default(),
                0,
            );
            timervalue.push(v)
        }
    }

    pub(super) fn load(&self, paths: Vec<PathBuf>, update: bool) -> Result<()> {
        let name = paths[0].file_name().unwrap().to_string_lossy().to_string();
        let data = match TimerConfigData::load_config(paths, &name) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Invalid Configuration: {}", e);
                return Err(Error::ConfigureError {
                    msg: format!("Invalid Configuration: {}", e),
                });
            }
        };

        // record original configuration
        *self.data.borrow_mut() = data;

        // parse and record processed configuration
        let ret = self.parse_unit();
        if ret.is_err() {
            self.reset(); // fallback
            return ret;
        }

        self.load_value();

        if update {
            self.db_update();
        }

        Ok(())
    }

    pub(super) fn config_data(&self) -> Rc<RefCell<TimerConfigData>> {
        self.data.clone()
    }

    pub(super) fn set_unit_ref(&self, unit: String) {
        self.set_ref(unit);
        self.db_update();
    }

    pub(super) fn unit_ref_target(&self) -> String {
        self.timerunitref.borrow().target()
    }

    fn parse_unit(&self) -> Result<()> {
        if let Some(unit) = self.config_data().borrow().Timer.Unit.clone() {
            if unit.ends_with(".timer") {
                log::warn!("Timer Unit must not be end with .timer, ignoring:{}", unit);
                return Ok(());
            }
            self.set_unit_ref(unit);
        } else if let Some(unit) = self.comm.owner() {
            self.set_unit_ref(unit.id().replace(".timer", ".service"));
        }
        Ok(())
    }

    fn set_ref(&self, target: String) {
        if let Some(u) = self.comm.owner() {
            self.timerunitref.borrow_mut().set_ref(u.id(), target)
        };
    }

    pub(super) fn set_property(&self, key: &str, value: &str) -> Result<(), core::error::Error> {
        let ret = self.data.borrow_mut().set_property(key, value);
        self.db_update();
        ret
    }
}
