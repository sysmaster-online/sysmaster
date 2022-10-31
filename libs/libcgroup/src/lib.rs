//! the library of operation on the cgroup
//!

use bitflags::bitflags;
use nix::errno::Errno;
use std::io::Error;

mod cgroup;

bitflags! {
    /// the flag that operate on the cgroup controller
    pub struct CgFlags: u8 {
        /// send SIGCONT to the process after kill it
        const SIGCONT = 1 << 0;
        /// ignore the process which call the kill operation
        const IGNORE_SELF = 1 << 1;
        /// remove the cgroup dir agter kill it
        const REMOVE = 1 << 2;
    }
}

/// the cgroup version of the mounted
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum CgType {
    /// cgroup is not mounted
    None,
    /// cgroup v1 mounted to /sys/fs/cgroup/sysmaster
    Legacy,
    /// cgroup v2 mounted to /sys/fs/cgroup/unifed
    UnifiedV1,
    /// cgroup v2 mounted to /sys/fs/cgroup/
    UnifiedV2,
}

/// the error returned of the cgroup operation
#[derive(Debug)]
pub enum CgroupErr {
    /// error from IoError
    IoError(Error),
    /// kill process error contain Errno
    KillError(Errno),
    /// cgroup is not supported
    NotSupported,
}

impl std::fmt::Display for CgroupErr {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let info = match self {
            CgroupErr::IoError(e) => format!("Io error: {}", e),
            CgroupErr::KillError(e) => format!("kill error no: {}", e),
            CgroupErr::NotSupported => "cgroup is not supported".to_string(),
        };
        fmt.write_str(info.as_str())
    }
}

pub use crate::cgroup::cg_attach;
pub use crate::cgroup::cg_controllers;
pub use crate::cgroup::cg_create;
pub use crate::cgroup::cg_escape;
pub use crate::cgroup::cg_get_pids;
pub use crate::cgroup::cg_is_empty_recursive;
pub use crate::cgroup::cg_kill_recursive;
pub use crate::cgroup::cg_type;
