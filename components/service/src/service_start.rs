use std::{collections::HashMap, process::exit};

use super::exec_child;
use super::service::{CmdError, CommandLine, ServiceUnit};
use nix::unistd::Pid;
use process1::manager::unit::unit_manager::UnitManager;

pub fn start_service(
    srvc: &mut ServiceUnit,
    manager: &mut UnitManager,
    cmdline: &CommandLine,
) -> Result<Pid, CmdError> {
    let mut env = HashMap::new();

    if let Some(pid) = srvc.main_pid {
        env.insert("MAINPID", format!("{}", pid));
    }

    unsafe {
        match nix::unistd::fork() {
            Ok(nix::unistd::ForkResult::Parent { child }) => {
                manager.add_watch_pid(child, &srvc.unit.id);
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
