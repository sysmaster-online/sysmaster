use nix::errno::Errno;
use std::io::Error;

mod cgroup;

pub enum CgFlags {
    CgSigcont = 1 << 0,
    CgIgnoreSelf = 1 << 1,
    CgRemove = 1 << 2,
}

#[derive(Debug)]
pub enum CgroupErr {
    IoError(Error),
    KillError(Errno),
    NotSupported,
}

impl std::fmt::Display for CgroupErr {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        let info = match self {
            CgroupErr::IoError(e) => format!("Io error: {}", e),
            CgroupErr::KillError(e) => format!("kill error no: {}", e),
            CgroupErr::NotSupported => format!("cgroup is not supported"),
        };
        fmt.write_str(format!("{}", info).as_str())
    }
}

pub use crate::cgroup::cg_attach;
pub use crate::cgroup::cg_create;
pub use crate::cgroup::cg_escape;
pub use crate::cgroup::cg_get_pids;
pub use crate::cgroup::cg_kill_recursive;
