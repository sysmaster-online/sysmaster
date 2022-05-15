use std::thread::sleep;
use std::time::Duration;
use std::{collections::HashMap, process::exit};

use super::exec_child;
use super::service::ServiceUnit;
use super::service_base::{CmdError, CommandLine};
use cgroup;
use nix::unistd::Pid;

pub fn start_service(srvc: &mut ServiceUnit, cmdline: &CommandLine) -> Result<Pid, CmdError> {
    let mut env = HashMap::new();

    if let Some(pid) = srvc.main_pid {
        env.insert("MAINPID", format!("{}", pid));
    }

    srvc.unit()
        .prepare_exec()
        .map_err(|_e| CmdError::SpawnError)?;

    unsafe {
        match nix::unistd::fork() {
            Ok(nix::unistd::ForkResult::Parent { child }) => {
                srvc.um().child_watch_pid(child, srvc.unit().get_id());
                log::debug!("child pid is :{}", child);
                cgroup::cg_attach(child, &srvc.unit().cg_path())
                    .map_err(|_e| CmdError::SpawnError)?;
                return Ok(child);
            }
            Ok(nix::unistd::ForkResult::Child) => {
                sleep(Duration::from_secs(2));
                exec_child::exec_child(srvc, cmdline, &env);
                exit(0);
            }
            Err(_e) => return Err(CmdError::SpawnError),
        }
    };
}
