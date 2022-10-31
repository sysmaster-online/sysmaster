//! socket_load mod parse the field of section Socket and add the extra dependency。
//!

use crate::socket_base::PortType;
use crate::socket_comm::SocketUnitComm;
use crate::socket_config::SocketConfig;
use libsysmaster::manager::{UnitRelations, UnitType};
use std::path::Path;
use std::{error::Error, rc::Rc};

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

    pub(super) fn socket_add_extras(&self) -> Result<(), Box<dyn Error>> {
        log::debug!("socket add extras");
        if self.can_accept() {
            if self.config.unit_ref_target().is_none() {
                self.load_related_unit(UnitType::UnitService)?;
            }

            self.comm.unit().insert_two_deps(
                UnitRelations::UnitBefore,
                UnitRelations::UnitTriggers,
                self.config.unit_ref_target().unwrap(),
            );
        }

        Ok(())
    }

    pub(super) fn socket_verify(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn load_related_unit(&self, related_type: UnitType) -> Result<(), Box<dyn Error>> {
        let unit_name = self.comm.unit().id().to_string();
        let suffix = String::from(related_type);
        if suffix.is_empty() {
            return Err(format!("failed to load related unit {}", suffix).into());
        }

        let stem_name = Path::new(&unit_name).file_stem().unwrap().to_str().unwrap();
        let relate_name = format!("{}.{}", stem_name, suffix);
        self.config.set_unit_ref(relate_name)?;

        Ok(())
    }

    fn can_accept(&self) -> bool {
        if !self.config.config_data().borrow().Socket.Accept {
            return true;
        };

        self.no_accept_socket()
    }

    fn no_accept_socket(&self) -> bool {
        for port in self.config.ports().iter() {
            if port.p_type() != PortType::Socket {
                return true;
            }

            if !port.sa().can_accept() {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use crate::{socket_comm::SocketUnitComm, socket_config::SocketConfig};
    use std::rc::Rc;
    use tests::get_project_root;

    #[test]
    fn test_socket_load_parse() {
        let comm = Rc::new(SocketUnitComm::new());
        let mut file_path = get_project_root().unwrap();
        file_path.push("test_units/test.socket.toml");

        let paths = vec![file_path];

        let config = SocketConfig::new(&comm);
        assert!(config.load(paths, false).is_ok());
    }
}
