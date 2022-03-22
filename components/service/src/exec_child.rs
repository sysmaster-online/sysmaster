use std::collections::HashMap;

use super::service::{CommandLine, ServiceUnit};
use log;
use regex::Regex;

fn build_exec_environment(
    service: &ServiceUnit,
    cmdline: &CommandLine,
    env: &HashMap<&str, String>,
) -> (std::ffi::CString, Vec<std::ffi::CString>) {
    // let command = cmdline.borrow();
    let cmd = std::ffi::CString::new(cmdline.cmd.clone()).unwrap();

    let exec_name = std::path::PathBuf::from(&cmdline.cmd);
    let exec_name = exec_name.file_name().unwrap().to_str().unwrap();
    let exec_name = std::ffi::CString::new::<Vec<u8>>(exec_name.bytes().collect()).unwrap();

    let mut args = Vec::new();
    args.push(exec_name);

    let var_regex = Regex::new(r"(\$[A-Z_]+)|(\$\{[A-Z_]+\})").unwrap();
    for arg in &cmdline.args {
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
                if let Some(target) = env.get(v) {
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

pub fn exec_child(service: &ServiceUnit, cmdline: &CommandLine, env: &HashMap<&str, String>) {
    let (cmd, args) = build_exec_environment(service, cmdline, env);
    let cstr_args = args
        .iter()
        .map(|cstring| cstring.as_c_str())
        .collect::<Vec<_>>();

    log::debug!("command is: {}, args is: {:?}", cmd.to_str().unwrap(), args);
    match nix::unistd::execv(&cmd, &cstr_args) {
        Ok(_) => {
            log::debug!("execv returned Ok()");
        }
        Err(e) => {
            log::error!("exec child failed: {:?}", e);
            std::process::exit(1);
        }
    }
}
