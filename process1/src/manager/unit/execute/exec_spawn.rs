use super::exec_base::{ExecCmdError, ExecCommand, ExecParameters};
use crate::manager::unit::Unit;
use cgroup;
use log;
use nix::unistd::{self, ForkResult, Pid};
use regex::Regex;
use std::process;
use std::thread;
use std::time::Duration;

pub(in crate::manager::unit) struct ExecSpawn;

impl ExecSpawn {
    pub(in crate::manager::unit) fn new() -> ExecSpawn {
        ExecSpawn
    }

    pub(in crate::manager::unit) fn spawn(
        &self,
        unit: &Unit,
        cmdline: &ExecCommand,
        params: &ExecParameters,
    ) -> Result<Pid, ExecCmdError> {
        unsafe {
            match unistd::fork() {
                Ok(ForkResult::Parent { child }) => {
                    log::debug!("child pid is :{}", child);
                    cgroup::cg_attach(child, &unit.cg_path())
                        .map_err(|_e| ExecCmdError::SpawnError)?;
                    return Ok(child);
                }
                Ok(ForkResult::Child) => {
                    thread::sleep(Duration::from_secs(2));
                    exec_child(unit, cmdline, params);
                    process::exit(0);
                }
                Err(_e) => return Err(ExecCmdError::SpawnError),
            }
        };
    }
}

fn exec_child(unit: &Unit, cmdline: &ExecCommand, params: &ExecParameters) {
    let (cmd, args) = build_run_environment(unit, cmdline, params);
    let cstr_args = args
        .iter()
        .map(|cstring| cstring.as_c_str())
        .collect::<Vec<_>>();

    log::debug!(
        "exec child command is: {}, args is: {:?}",
        cmd.to_str().unwrap(),
        args
    );
    match unistd::execv(&cmd, &cstr_args) {
        Ok(_) => {
            log::debug!("execv returned Ok()");
        }
        Err(e) => {
            log::error!("exec child failed: {:?}", e);
            std::process::exit(1);
        }
    }
}

// contrast: build_environment
fn build_run_environment(
    _unit: &Unit,
    cmdline: &ExecCommand,
    env: &ExecParameters,
) -> (std::ffi::CString, Vec<std::ffi::CString>) {
    // let command = cmdline.borrow();
    let cmd = std::ffi::CString::new(cmdline.path().clone()).unwrap();

    let exec_name = std::path::PathBuf::from(cmdline.path());
    let exec_name = exec_name.file_name().unwrap().to_str().unwrap();
    let exec_name = std::ffi::CString::new::<Vec<u8>>(exec_name.bytes().collect()).unwrap();

    let mut args = Vec::new();
    args.push(exec_name);

    let var_regex = Regex::new(r"(\$[A-Z_]+)|(\$\{[A-Z_]+\})").unwrap();
    for arg in cmdline.argv() {
        let cap = var_regex.captures(arg);
        if let Some(cap) = cap {
            let match_result = {
                if let Some(mat) = cap.get(1) {
                    Some(mat.as_str())
                } else if let Some(mat) = cap.get(2) {
                    Some(mat.as_str())
                } else {
                    None
                }
            };

            if let Some(val) = match_result {
                let v = val.trim_matches('$').trim_matches('{').trim_matches('}');
                if let Some(target) = env.get_env(v) {
                    args.push(
                        std::ffi::CString::new(var_regex.replace(arg, target).to_string()).unwrap(),
                    );
                };
            }
            continue;
        }

        args.push(std::ffi::CString::new(arg.as_str()).unwrap())
    }

    (cmd, args)
}
