use crate::service_base::ServiceType;
use crate::service_config::ServiceConfig;

use super::service_comm::ServiceComm;
use super::service_pid::ServicePid;
use nix::unistd::Pid;
use process1::manager::{ExecCommand, ExecContext, ExecFlags, ExecParameters};
use std::env;
use std::error::Error;
use std::rc::Rc;

pub(super) struct ServiceSpawn {
    comm: Rc<ServiceComm>,
    pid: Rc<ServicePid>,
    config: Rc<ServiceConfig>,
    exec_ctx: Rc<ExecContext>,
}

impl ServiceSpawn {
    pub(super) fn new(
        commr: &Rc<ServiceComm>,
        pidr: &Rc<ServicePid>,
        configr: &Rc<ServiceConfig>,
        exec_ctx: &Rc<ExecContext>,
    ) -> ServiceSpawn {
        ServiceSpawn {
            comm: Rc::clone(commr),
            pid: Rc::clone(pidr),
            config: configr.clone(),
            exec_ctx: exec_ctx.clone(),
        }
    }

    pub(super) fn start_service(
        &self,
        cmdline: &ExecCommand,
        _time_out: u64,
        ec_flags: ExecFlags,
    ) -> Result<Pid, Box<dyn Error>> {
        let mut params = ExecParameters::new();

        params.add_env(
            "PATH",
            env::var("PATH").unwrap_or(
                "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
            ),
        );

        if let Some(pid) = self.pid.main() {
            params.add_env("MAINPID", format!("{}", pid));
        }

        let unit = self.comm.unit();
        let um = self.comm.um();
        unit.prepare_exec()?;

        if ec_flags.contains(ExecFlags::PASS_FDS) {
            params.insert_fds(self.collect_socket_fds());
        }

        if self.config.service_type() == ServiceType::Notify {
            let notify_sock = um.notify_socket().unwrap();
            log::debug!("add NOTIFY_SOCKET env: {}", notify_sock.to_str().unwrap());
            params.add_env(
                "NOTIFY_SOCKET",
                format!("{}", notify_sock.to_str().unwrap()),
            );
            params.set_notify_sock(notify_sock);
        }

        log::debug!("begin to exec spawn");
        match um.exec_spawn(&unit, cmdline, &params, self.exec_ctx.clone()) {
            Ok(pid) => {
                um.child_watch_pid(pid, unit.get_id());
                Ok(pid)
            }
            Err(e) => {
                log::error!("failed to start service: {}, error:{:?}", unit.get_id(), e);
                Err(format!("spawn exec return error").into())
            }
        }
    }

    fn collect_socket_fds(&self) -> Vec<i32> {
        self.comm.um().collect_socket_fds(self.comm.unit().get_id())
    }
}
