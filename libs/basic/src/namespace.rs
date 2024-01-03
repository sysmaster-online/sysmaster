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

//!

use nix::{
    errno::Errno,
    fcntl::{open, OFlag},
    mount::{mount, MsFlags},
    sched,
    sys::stat::Mode,
    unistd::{setgroups, setresgid, setresuid, Gid, Pid, Uid},
    Result,
};
use std::{
    fs, io,
    os::unix::io::RawFd,
    path::{Path, PathBuf},
};

/// return (/proc/self/ns/${p})'s fd if $pid is 0.
/// return (/proc/${pid}/ns/${p})'s fd if $pid isn't 0.
pub fn namespace_open(pid: &Pid, p: &Path) -> Result<RawFd> {
    let proc_path: PathBuf;
    if pid.as_raw() == 0 {
        let s = format!("/proc/self/ns/{}", p.display());
        proc_path = PathBuf::from(&s);
    } else {
        let s = format!("/proc/{}/ns/{}", pid.as_raw(), p.display());
        proc_path = PathBuf::from(&s);
    }
    open(
        &proc_path,
        OFlag::O_RDONLY | OFlag::O_NOCTTY | OFlag::O_CLOEXEC,
        Mode::empty(),
    )
}

/// Detaches the mount namespace, disabling propagation from our namespace to the host
pub fn detach_mount_namespace() -> Result<()> {
    sched::unshare(sched::CloneFlags::CLONE_NEWNS)?;

    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_SLAVE | MsFlags::MS_REC,
        None::<&str>,
    )
}

/// The function `reset_uid_gid` resets the user and group IDs to root (0) in Rust.
///
/// Returns:
///
/// The function `reset_uid_gid()` returns a `Result` type. If the function executes successfully, it
/// returns `Ok(())`, indicating that there was no error. If there is an error during execution, it
/// returns `Err(Errno::EIO)`, indicating an input/output error.
fn reset_uid_gid() -> Result<()> {
    match fs::read("/proc/self/setgroups") {
        Ok(s) => {
            if s != "allow".to_string().into_bytes() {
                return Ok(());
            }
        }
        Err(e) => {
            if e.kind() != io::ErrorKind::NotFound {
                return Err(Errno::EIO);
            }
        }
    };

    setgroups(&[])?;

    setresgid(Gid::from_raw(0), Gid::from_raw(0), Gid::from_raw(0))?;

    setresuid(Uid::from_raw(0), Uid::from_raw(0), Uid::from_raw(0))
}

///
/// The function `namespace_enter` enters a namespace specified by a file descriptor and clone flags,
/// and then resets the user and group IDs.
///
/// Arguments:
///
/// * `fd`: A reference to a RawFd, which is a file descriptor.
/// * `f`: The parameter `f` is of type `sched::CloneFlags`. It is used to specify the behavior of the
/// `setns` function. `sched::CloneFlags` is an enum that represents various flags that can be passed to
/// the `setns` function.
///
/// Returns:
///
/// a `Result<()>`.
pub fn namespace_enter(fd: &RawFd, f: sched::CloneFlags) -> Result<()> {
    sched::setns(*fd, f)?;
    reset_uid_gid()
}
