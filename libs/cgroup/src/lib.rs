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

//! the library of operation on the cgroup
//!

#[cfg(all(feature = "linux", feature = "hongmeng"))]
compile_error!("feature linux and hongmeng cannot be enabled at the same time");

use bitflags::bitflags;
mod cgroup;
pub mod error;
pub use crate::cgroup::cg_attach;
pub use crate::cgroup::cg_controllers;
pub use crate::cgroup::cg_create;
pub use crate::cgroup::cg_create_and_attach;
pub use crate::cgroup::cg_escape;
pub use crate::cgroup::cg_get_pids;
pub use crate::cgroup::cg_is_empty_recursive;
pub use crate::cgroup::cg_kill_recursive;
pub use crate::cgroup::cg_type;
pub use crate::cgroup::CgController;
pub use crate::cgroup::CG_BASE_DIR;

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
    /// cgroup v1 mounted to /sys/fs/cgroup/systemd
    LegacySystemd,
}
