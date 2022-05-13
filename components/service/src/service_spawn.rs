use super::service_comm::ServiceComm;
use super::service_pid::ServicePid;
use nix::unistd::Pid;
use process1::manager::{ExecCommand, ExecParameters};
use std::error::Error;
use std::rc::Rc;

pub(super) struct ServiceSpawn {
    comm: Rc<ServiceComm>,
    pid: Rc<ServicePid>,
}

impl ServiceSpawn {
    pub(super) fn new(commr: &Rc<ServiceComm>, pidr: &Rc<ServicePid>) -> ServiceSpawn {
        ServiceSpawn {
            comm: Rc::clone(commr),
            pid: Rc::clone(pidr),
        }
    }

    pub(super) fn start_service(
        &self,
        cmdline: &ExecCommand,
        _time_out: u64,
    ) -> Result<Pid, Box<dyn Error>> {
        let params = ExecParameters::new();
        if let Some(pid) = self.pid.main() {
            params.add_env("MAINPID", format!("{}", pid));
        }

        let unit = self.comm.unit();
        let um = self.comm.um();
        unit.prepare_exec()?;
        match um.exec_spawn(&unit, cmdline, &params) {
            Ok(pid) => {
                um.child_watch_pid(pid, unit.get_id());
                Ok(pid)
            }
            Err(_e) => {
                log::error!("failed to start service: {}", unit.get_id());
                Err(format!("spawn exec return error").into())
            }
        }
    }
}
