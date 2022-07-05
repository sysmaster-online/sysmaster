use std::{error::Error, rc::Rc};

use nix::unistd::Pid;
use process1::manager::{ExecCommand, ExecParameters};

use crate::socket_comm::SocketComm;

#[allow(dead_code)]
pub(super) struct SocketSpawn {
    comm: Rc<SocketComm>,
}

impl SocketSpawn {
    pub(super) fn new(comm: &Rc<SocketComm>) -> SocketSpawn {
        SocketSpawn { comm: comm.clone() }
    }

    #[allow(dead_code)]
    pub(super) fn start_socket(&self, cmdline: &ExecCommand) -> Result<Pid, Box<dyn Error>> {
        let params = ExecParameters::new();

        let unit = self.comm.unit();
        let um = self.comm.um();
        unit.prepare_exec()?;
        match um.exec_spawn(&unit, cmdline, &params) {
            Ok(pid) => {
                um.child_watch_pid(pid, unit.get_id());
                Ok(pid)
            }
            Err(_e) => {
                log::error!("failed to start socket: {}", unit.get_id());
                Err(format!("spawn exec return error").into())
            }
        }
    }
}
