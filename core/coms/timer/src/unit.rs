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

use crate::{
    bus::TimerBus,
    comm::TimerUnitComm,
    config::TimerConfig,
    load::TimerLoad,
    mng::{MonotonicTimer, RealtimeTimer, TimerMng},
};
use core::{
    error::*,
    rel::{ReStation, Reliability},
    unit::{SubUnit, UnitActiveState, UnitBase, UnitMngUtil},
    UmIf,
};
use std::{any::Any, path::PathBuf, rc::Rc};

struct TimerUnit {
    comm: Rc<TimerUnitComm>,
    config: Rc<TimerConfig>,
    mng: Rc<TimerMng>,
    load: TimerLoad,
    bus: TimerBus,
}

impl ReStation for TimerUnit {
    // input: do nothing

    // compensate: do nothing

    // data
    fn db_map(&self, reload: bool) {
        self.config.db_map(reload);
        self.mng.db_map(reload);
    }

    fn db_insert(&self) {
        self.config.db_insert();
        self.mng.db_insert();
    }
}

impl SubUnit for TimerUnit {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn load(&self, paths: Vec<PathBuf>) -> Result<()> {
        log::debug!("timer begin to load conf file");
        self.config.load(paths, true)?;

        let ret = self.load.timer_add_extras();
        if ret.is_err() {
            self.config.reset();
            return ret;
        }

        self.verify()
    }

    // the function entrance to start the unit
    fn start(&self) -> Result<()> {
        let starting = self.mng.start()?;
        if starting {
            log::debug!("timer already in start");
        }

        Ok(())
    }

    fn stop(&self, force: bool) -> Result<()> {
        if !force {
            let stopping = self.mng.stop()?;
            if stopping {
                log::debug!("timer already in stop, return immediretly");
                return Ok(());
            }
        }

        self.mng.stop().unwrap();

        Ok(())
    }

    fn reset_failed(&self) {
        self.mng.reset_failed()
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.mng.current_active_state()
    }

    fn get_subunit_state(&self) -> String {
        self.mng.state().to_string()
    }

    fn attach_unit(&self, unit: Rc<dyn UnitBase>) {
        self.comm.attach_unit(unit);
        self.db_insert();
    }

    fn trigger_notify(&self) {
        self.mng.trigger_notify()
    }

    fn unit_set_property(
        &self,
        key: &str,
        value: &str,
        flags: core::unit::UnitWriteFlags,
    ) -> Result<()> {
        self.bus.unit_set_property(key, value, flags)
    }
}

impl UnitMngUtil for TimerUnit {
    fn attach_um(&self, um: Rc<dyn UmIf>) {
        self.comm.attach_um(um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

impl TimerUnit {
    fn new(_um: Rc<dyn UmIf>) -> TimerUnit {
        let comm = Rc::new(TimerUnitComm::new());
        let config = Rc::new(TimerConfig::new(&comm));
        let mt = Rc::new(MonotonicTimer::new(0));
        let rt = Rc::new(RealtimeTimer::new(0));
        let mng = Rc::new(TimerMng::new(&comm, &config, &mt, &rt));
        mt.attach_mng(Rc::downgrade(&mng));
        rt.attach_mng(Rc::downgrade(&mng));
        TimerUnit {
            comm: Rc::clone(&comm),
            config: Rc::clone(&config),
            mng: Rc::clone(&mng),
            load: TimerLoad::new(&config, &comm),
            bus: TimerBus::new(&comm, &config),
        }
    }

    fn verify(&self) -> Result<()> {
        if self.config.value.borrow().is_empty()
            && !self.mng.get_on_clock_change()
            && !self.mng.get_on_timezone_change()
        {
            log::error!("Timer unit lacks value setting. Refusing.");
            return Err(Error::Nix {
                source: nix::Error::ENOEXEC,
            });
        }
        Ok(())
    }
}

use core::declare_unitobj_plugin_with_param;
declare_unitobj_plugin_with_param!(TimerUnit, TimerUnit::new);
