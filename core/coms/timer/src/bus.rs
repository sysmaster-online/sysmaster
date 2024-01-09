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

use super::comm::TimerUnitComm;
use super::config::TimerConfig;
use core::error::*;
use core::unit::{self, UnitWriteFlags};
use std::rc::Rc;

pub struct TimerBus {
    // associated objects
    comm: Rc<TimerUnitComm>,
    config: Rc<TimerConfig>,
    // owned objects
}

impl TimerBus {
    pub(super) fn new(commr: &Rc<TimerUnitComm>, configr: &Rc<TimerConfig>) -> TimerBus {
        TimerBus {
            comm: Rc::clone(commr),
            config: Rc::clone(configr),
        }
    }

    pub(super) fn unit_set_property(
        &self,
        key: &str,
        value: &str,
        flags: UnitWriteFlags,
    ) -> Result<()> {
        let mut ret = self.cgroup_set_transient_property(key, value, flags);
        if let Err(Error::NotFound { what: _ }) = ret {
            let unit = self.comm.owner().unwrap();
            if unit.transient() && unit.is_load_stub() {
                ret = self.unit_set_transient_property(key, value, flags);

                if let Err(Error::NotFound { what: _ }) = ret {
                    ret = self.exec_set_transient_property(key, value, flags);
                }

                if let Err(Error::NotFound { what: _ }) = ret {
                    ret = self.kill_set_transient_property(key, value, flags);
                }
            }
        }

        ret
    }

    fn unit_set_transient_property(
        &self,
        key: &str,
        value: &str,
        flags: UnitWriteFlags,
    ) -> Result<()> {
        let real_flags = flags | UnitWriteFlags::PRIVATE;
        match key {
            "AccuracySec" | "RandomizedDelaySec" | "WakeSystem" | "Persistent"
            | "RemainAfterElapse" | "OnActiveSec" | "OnBootSec" | "OnStartupSec"
            | "OnUnitActiveSec" | "OnUnitInactiveSec" | "OnCalendar" => {
                self.unit_write_property(key, value, real_flags, false)
            }
            "Unit" => self.unit_write_property(key, value, real_flags, false),
            str_key => Err(Error::NotFound {
                what: format!("set transient property:{}", str_key),
            }),
        }
    }

    fn exec_set_transient_property(
        &self,
        key: &str,
        _value: &str,
        _flags: UnitWriteFlags,
    ) -> Result<()> {
        // not supported now
        Err(Error::NotFound {
            what: format!("set exec property:{}", key),
        })
    }

    fn kill_set_transient_property(
        &self,
        key: &str,
        _value: &str,
        _flags: UnitWriteFlags,
    ) -> Result<()> {
        // not supported now
        Err(Error::NotFound {
            what: format!("set kill property:{}", key),
        })
    }

    fn cgroup_set_transient_property(
        &self,
        key: &str,
        _value: &str,
        _flags: UnitWriteFlags,
    ) -> Result<()> {
        // not supported now
        Err(Error::NotFound {
            what: format!("set cgroup property:{}", key),
        })
    }

    fn unit_write_property(
        &self,
        key: &str,
        value: &str,
        flags: UnitWriteFlags,
        update: bool,
    ) -> Result<()> {
        if unit::unit_write_flags_is_noop(flags) {
            return Ok(());
        }

        let unit = self.comm.owner().unwrap();
        let um = self.comm.um();
        let ps = um.private_section(unit.unit_type());

        if update {
            self.set_property(key, value)?;
        }
        unit::unit_write_settingf(unit, &ps, flags, key, format_args!("{}={}", key, value))
    }

    fn set_property(&self, key: &str, value: &str) -> Result<()> {
        self.config.set_property(key, value)
    }
}
