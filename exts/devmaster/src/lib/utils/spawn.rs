// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! utilities for spawning a child process
//!

use crate::{error::*, rules::exec_unit::ExecuteUnit};
use lazy_static::lazy_static;
use shell_words::split;
use std::{
    io::Read,
    process::{Command, Stdio},
    time::Duration,
};
use wait_timeout::ChildExt;

/// if the command is not absolute path, try to find it under lib directory first.
pub(crate) fn spawn(cmd_str: &str, timeout: Duration, unit: &ExecuteUnit) -> Result<(String, i32)> {
    lazy_static! {
        static ref LIB_DIRS: Vec<String> =
            vec!["/lib/udev/".to_string(), "/lib/devmaster/".to_string()];
    }

    let dev = unit.get_device();

    let cmd_tokens = split(cmd_str).map_err(|e| Error::Other {
        msg: format!(
            "failed to split command '{}' into shell tokens: ({})",
            cmd_str, e
        ),
        errno: nix::errno::Errno::EINVAL,
    })?;

    if cmd_tokens.is_empty() {
        return Err(Error::Other {
            msg: "failed to spawn empty command.".to_string(),
            errno: nix::errno::Errno::EINVAL,
        });
    }

    let mut cmd = if !cmd_tokens[0].starts_with('/') {
        LIB_DIRS
            .iter()
            .map(|lib| lib.clone() + cmd_tokens[0].as_str())
            .find(|path| std::fs::metadata(path).is_ok())
            .map_or_else(|| Command::new(&cmd_tokens[0]), Command::new)
    } else {
        Command::new(&cmd_tokens[0])
    };

    for (key, val) in &dev.property_iter() {
        cmd.env(key, val);
    }

    let cmd = cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    for arg in &cmd_tokens[1..] {
        cmd.arg(arg);
    }

    let mut child = cmd.spawn().map_err(|e| Error::Other {
        msg: format!("failed to spawn command '{:?}': {}", cmd, e),
        errno: nix::errno::Errno::EINVAL,
    })?;
    let pid = child.id();

    match child.wait_timeout(timeout).map_err(|e| Error::Other {
        msg: format!("failed to kill child process {} '{:?}': ({})", pid, cmd, e),
        errno: nix::errno::Errno::EINVAL,
    })? {
        Some(status) => {
            log::debug!("Process {} exited with status {:?}", pid, status);
            // status.code()
            let mut stdout = child.stdout.take().unwrap();
            let mut stderr = child.stderr.take().unwrap();
            let retno = status.code().unwrap();

            let mut stdout_buf = String::new();
            stdout
                .read_to_string(&mut stdout_buf)
                .map_err(|e| Error::Other {
                    msg: format!(
                        "failed to read stdout for child process {} '{:?}': ({})",
                        pid, cmd, e
                    ),
                    errno: nix::errno::Errno::EINVAL,
                })?;

            let mut stderr_buf = String::new();
            stderr
                .read_to_string(&mut stderr_buf)
                .map_err(|e| Error::Other {
                    msg: format!(
                        "failed to read stderr for child process {} '{:?}': ({})",
                        pid, cmd, e
                    ),
                    errno: nix::errno::Errno::EINVAL,
                })?;

            if !stderr_buf.is_empty() {
                log::debug!(
                    "stderr from child process {} {:?}: {}",
                    pid,
                    cmd,
                    stderr_buf
                );
            }

            Ok((stdout_buf, retno))
        }
        None => {
            child.kill().map_err(|e| Error::Other {
                msg: format!("failed to kill child process {} '{:?}': ({})", pid, cmd, e),
                errno: nix::errno::Errno::EINVAL,
            })?;
            Err(Error::Other {
                msg: format!("child process {} '{:?}' timed out", pid, cmd),
                errno: nix::errno::Errno::EINVAL,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::exec_unit::ExecuteUnit;
    use device::Device;
    use log::init_log;
    use log::Level;
    use std::rc::Rc;

    #[test]
    fn test_spawn() {
        init_log("test_spawn", Level::Debug, vec!["console"], "", 0, 0, false);

        let dev = Device::from_subsystem_sysname("net", "lo").unwrap();
        let unit = ExecuteUnit::new(Rc::new(dev));

        println!(
            "{}",
            spawn("echo hello world", Duration::from_secs(1), &unit)
                .unwrap()
                .0
        );

        println!(
            "{}",
            spawn("/bin/echo hello world", Duration::from_secs(1), &unit)
                .unwrap()
                .0
        );

        println!(
            "{}",
            spawn("sleep 2", Duration::from_secs(1), &unit).unwrap_err()
        );

        println!(
            "{}",
            spawn("sleep 1", Duration::from_secs(10), &unit).unwrap().0
        );

        println!(
            "{}",
            spawn(
                "sh -c '/bin/echo test shell'",
                Duration::from_secs(1),
                &unit
            )
            .unwrap()
            .0
        );
    }
}
