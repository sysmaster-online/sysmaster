use std::process;
use std::thread::sleep;
use std::time::Duration;
use std::{cell::RefCell, rc::Rc};

use crate::manager::{Unit, UnitManager};

use super::{exec_child, CmdError, CommandLine, ExecParameters};
use cgroup;
use nix::unistd::Pid;

pub struct ExecSpawn {
    unit: Rc<Unit>,
    um: Rc<UnitManager>,
    cmdline: Rc<RefCell<CommandLine>>,
    exec_params: Rc<ExecParameters>,
}

impl ExecSpawn {
    pub fn new(
        unit: Rc<Unit>,
        um: Rc<UnitManager>,
        cmd: Rc<RefCell<CommandLine>>,
        params: Rc<ExecParameters>,
    ) -> ExecSpawn {
        ExecSpawn {
            unit,
            um,
            cmdline: cmd,
            exec_params: params,
        }
    }

    pub fn start(&self) -> Result<Pid, CmdError> {
        unsafe {
            match nix::unistd::fork() {
                Ok(nix::unistd::ForkResult::Parent { child }) => {
                    self.um.child_watch_pid(child, self.unit.get_id());
                    log::debug!("child pid is :{}", child);
                    cgroup::cg_attach(child, &self.unit.cg_path())
                        .map_err(|_e| CmdError::SpawnError)?;
                    return Ok(child);
                }
                Ok(nix::unistd::ForkResult::Child) => {
                    sleep(Duration::from_secs(2));
                    exec_child::exec_child(
                        self.unit.clone(),
                        &self.cmdline.borrow(),
                        self.exec_params.clone(),
                    );
                    process::exit(0);
                }
                Err(_e) => return Err(CmdError::SpawnError),
            }
        };
    }
}
