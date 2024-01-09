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

//! SocketUnit is the entrance of the sub unit，implement the trait UnitObj,UnitMngUtil and UnitSubClass.
//! Trait UnitObj defines the behavior of the sub unit.
//! Trait UnitMngUtil is used to attach the Unitmanager to the sub unit.
//! Trait UnitSubClass implement the convert from sub unit to UnitObj.

use crate::bus::SocketBus;
use crate::mng::SocketMngPort;
use crate::port::SocketPort;
use crate::rentry::SocketState;
use crate::{comm::SocketUnitComm, config::SocketConfig, load::SocketLoad, mng::SocketMng};
use core::error::*;
use core::exec::ExecContext;
use core::rel::{ReStation, Reliability};
use core::unit::{SubUnit, UmIf, UnitActiveState, UnitBase, UnitMngUtil, UnitWriteFlags};
use nix::sys::wait::WaitStatus;
use std::any::Any;
use std::{path::PathBuf, rc::Rc};

// the structuer of the socket unit type
struct SocketUnit {
    comm: Rc<SocketUnitComm>,
    config: Rc<SocketConfig>,
    mng: Rc<SocketMng>,
    load: SocketLoad,
    bus: SocketBus,
}

impl ReStation for SocketUnit {
    // input: do nothing

    // compensate: do nothing

    // data
    fn db_map(&self, reload: bool) {
        self.config.db_map(reload);
        if !reload {
            self.build_ports();
        }
        self.mng.db_map(reload);
    }

    fn db_insert(&self) {
        self.config.db_insert();
        self.mng.db_insert();
    }

    // reload: entry-only
    fn entry_coldplug(&self) {
        // rebuild external connections, like: timer, ...
        self.mng.entry_coldplug();
    }

    fn entry_clear(&self) {
        // release external connection, like: timer, ...
        self.mng.entry_clear();
    }
}

impl SubUnit for SocketUnit {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn load(&self, paths: Vec<PathBuf>) -> Result<()> {
        log::debug!("socket begin to load conf file");
        self.config.load(paths, true)?;

        let ret = self.load.socket_add_extras();
        if ret.is_err() {
            self.config.reset();
            return ret;
        }

        self.build_ports();

        self.verify()
    }

    // the function entrance to start the unit
    fn start(&self) -> Result<()> {
        let starting = self.mng.start_check()?;
        if starting {
            log::debug!("socket already in start");
            return Ok(());
        }

        self.mng.start_action();

        Ok(())
    }

    fn stop(&self, force: bool) -> Result<()> {
        if !force {
            let stopping = self.mng.stop_check()?;
            if stopping {
                log::debug!("socket already in stop, return immediretly");
                return Ok(());
            }
        }

        self.mng.stop_action();

        Ok(())
    }

    fn trigger(&self, other: &str) {
        if ![SocketState::Running, SocketState::Listening].contains(&self.mng.state()) {
            return;
        }

        if self.config.config_data().borrow().Socket.Accept {
            return;
        }

        let um = self.comm.um();

        if um.has_job(other) {
            return;
        }

        let service_state = um.get_subunit_state(other);
        if [
            "dead".to_string(),
            "failed".to_string(),
            "finalsigterm".to_string(),
            "finalsigkill".to_string(),
            "autorestart".to_string(),
        ]
        .contains(&service_state)
        {
            self.mng.enter_listening();
        }
    }

    fn sigchld_events(&self, wait_status: WaitStatus) {
        self.mng.sigchld_event(wait_status)
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.mng.current_active_state()
    }

    fn get_subunit_state(&self) -> String {
        self.mng.state().to_string()
    }

    fn collect_fds(&self) -> Vec<i32> {
        self.mng.collect_fds()
    }

    fn attach_unit(&self, unit: Rc<dyn UnitBase>) {
        self.comm.attach_unit(unit);
        self.db_insert();
    }

    fn unit_set_property(&self, key: &str, value: &str, flags: UnitWriteFlags) -> Result<()> {
        self.bus.unit_set_property(key, value, flags)
    }
}

// attach the UnitManager for weak reference
impl UnitMngUtil for SocketUnit {
    fn attach_um(&self, um: Rc<dyn UmIf>) {
        self.comm.attach_um(um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

impl SocketUnit {
    fn new(_um: Rc<dyn UmIf>) -> SocketUnit {
        let context = Rc::new(ExecContext::new());
        let comm = Rc::new(SocketUnitComm::new());
        let config = Rc::new(SocketConfig::new(&comm));
        SocketUnit {
            comm: Rc::clone(&comm),
            config: Rc::clone(&config),
            mng: Rc::new(SocketMng::new(
                &Rc::clone(&comm),
                &Rc::clone(&config),
                &Rc::clone(&context),
            )),
            load: SocketLoad::new(&Rc::clone(&config), &Rc::clone(&comm)),
            bus: SocketBus::new(&comm, &config),
        }
    }

    fn find_symlink_target(&self) -> Option<String> {
        if self.config.ports().is_empty() {
            return None;
        }
        let mut res: Option<String> = None;
        for port in self.config.ports() {
            if !port.can_be_symlinked() {
                continue;
            }
            /* Already found one target, refuse if there are more. */
            if res.is_some() {
                return None;
            }
            res = Some(port.listen().to_string());
        }
        res
    }

    fn build_ports(&self) {
        for p_conf in self.config.ports().iter() {
            let port = Rc::new(SocketPort::new(&self.comm, &self.config, p_conf));
            let mport = Rc::new(SocketMngPort::new(&self.mng, port));
            self.mng.push_port(mport);
        }
    }

    fn verify(&self) -> Result<()> {
        if self.config.ports().is_empty() {
            log::error!("Unit has no Listen setting (ListenStream=, ListenDatagram=, ListenFIFO=, ...). Refusing.");
            return Err(Error::Nix {
                source: nix::Error::ENOEXEC,
            });
        }

        let config = self.config.config_data();
        if !config.borrow().Socket.Symlinks.is_empty() && self.find_symlink_target().is_none() {
            /* Set to None, so we won't create symlinks by mistake. */
            config.borrow_mut().Socket.Symlinks = Vec::new();
            log::error!("Symlinks in [Socket] is configured, but there are none or more than one listen files.");
            return Err(Error::Nix {
                source: nix::Error::ENOEXEC,
            });
        }
        Ok(())
    }
}

// define the method to create the instance of the unit
use core::declare_unitobj_plugin_with_param;
declare_unitobj_plugin_with_param!(SocketUnit, SocketUnit::new);
