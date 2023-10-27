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

//! socket_load mod parse the field of section Socket and add the extra dependency
//!
use crate::comm::SocketUnitComm;
use crate::config::SocketConfig;
use crate::rentry::PortType;
use basic::{SHUTDOWN_TARGET, SOCKETS_TARGET, SYSINIT_TARGET};
use core::error::*;
use core::unit::{UnitDependencyMask, UnitRelations, UnitType};
use std::path::Path;
use std::rc::Rc;

pub(super) struct SocketLoad {
    config: Rc<SocketConfig>,
    comm: Rc<SocketUnitComm>,
}

impl SocketLoad {
    pub(super) fn new(configr: &Rc<SocketConfig>, commr: &Rc<SocketUnitComm>) -> Self {
        SocketLoad {
            config: configr.clone(),
            comm: commr.clone(),
        }
    }

    pub(super) fn socket_add_extras(&self) -> Result<()> {
        log::debug!("socket add extras");
        if self.have_non_accept_socket() {
            if self.config.unit_ref_target().is_none() {
                self.load_related_unit(UnitType::UnitService)?;
            }
            if let Some(owner) = self.comm.owner() {
                let um = self.comm.um();
                um.unit_add_two_dependency(
                    &owner.id(),
                    UnitRelations::UnitBefore,
                    UnitRelations::UnitTriggers,
                    &self.config.unit_ref_target().unwrap(),
                    true,
                    UnitDependencyMask::Implicit,
                )?;
            }
        }

        self.add_default_dependencies()?;

        Ok(())
    }

    fn load_related_unit(&self, related_type: UnitType) -> Result<()> {
        let unit_name = self.comm.owner().map(|u| u.id());
        let suffix = String::from(related_type);
        if suffix.is_empty() {
            return Err(format!("failed to load related unit {}", suffix).into());
        }
        if unit_name.is_none() {
            return Err(format!("failed to load related unit {} unit name is none", suffix).into());
        }
        let u_name = unit_name.unwrap();
        let stem_name = Path::new(&u_name).file_stem().unwrap().to_str().unwrap();
        let relate_name = format!("{}.{}", stem_name, suffix);
        self.config.set_unit_ref(relate_name);
        Ok(())
    }

    fn have_non_accept_socket(&self) -> bool {
        if !self.config.config_data().borrow().Socket.Accept {
            return true;
        }

        for port in self.config.ports().iter() {
            if port.p_type() != PortType::Socket {
                return true;
            }

            if !port.can_accept() {
                return true;
            }
        }

        false
    }

    pub(self) fn add_default_dependencies(&self) -> Result<()> {
        let u = match self.comm.owner() {
            None => {
                return Ok(());
            }
            Some(v) => v,
        };

        if !u.default_dependencies() {
            return Ok(());
        }

        log::debug!("Adding default dependencies for socket: {}", u.id());
        let um = self.comm.um();
        um.unit_add_dependency(
            &u.id(),
            UnitRelations::UnitAfter,
            SOCKETS_TARGET,
            true,
            UnitDependencyMask::Default,
        )?;
        um.unit_add_two_dependency(
            &u.id(),
            UnitRelations::UnitAfter,
            UnitRelations::UnitRequires,
            SYSINIT_TARGET,
            true,
            UnitDependencyMask::Default,
        )?;
        um.unit_add_two_dependency(
            &u.id(),
            UnitRelations::UnitBefore,
            UnitRelations::UnitConflicts,
            SHUTDOWN_TARGET,
            true,
            UnitDependencyMask::Default,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{comm::SocketUnitComm, config::SocketConfig};
    use libtests::get_project_root;
    use std::rc::Rc;

    #[test]
    fn test_socket_load_parse() {
        let comm = Rc::new(SocketUnitComm::new());
        let mut file_path = get_project_root().unwrap();
        file_path.push("tests/test_units/uttest.socket");

        let paths = vec![file_path];

        let config = SocketConfig::new(&comm);
        assert!(config.load(paths, false).is_ok());
    }
}
