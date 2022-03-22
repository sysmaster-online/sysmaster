use std::{collections::HashMap, process::exit};

use super::exec_child;
use super::service::ServiceUnit;
use super::service_base::{CmdError, CommandLine};
use nix::unistd::Pid;

pub fn start_service(srvc: &mut ServiceUnit, cmdline: &CommandLine) -> Result<Pid, CmdError> {
    let mut env = HashMap::new();

    if let Some(pid) = srvc.main_pid {
        env.insert("MAINPID", format!("{}", pid));
    }

    unsafe {
        match nix::unistd::fork() {
            Ok(nix::unistd::ForkResult::Parent { child }) => {
                srvc.um
                    .as_ref()
                    .cloned()
                    .unwrap()
                    .upgrade()
                    .as_ref()
                    .cloned()
                    .unwrap()
                    .child_watch_pid(
                        child,
                        srvc.unit
                            .as_ref()
                            .cloned()
                            .unwrap()
                            .upgrade()
                            .as_ref()
                            .cloned()
                            .unwrap()
                            .get_id(),
                    );
                return Ok(child);
            }
            Ok(nix::unistd::ForkResult::Child) => {
                exec_child::exec_child(srvc, cmdline, &env);
                exit(0);
            }
            Err(_e) => return Err(CmdError::SpawnError),
        }
    };
}
