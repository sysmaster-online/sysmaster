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
#[cfg(feature = "capability")]
pub mod capability;
#[cfg(feature = "cargo")]
pub mod cargo;
#[cfg(feature = "condition")]
pub mod condition;
#[cfg(feature = "config")]
pub mod config;
pub mod error;
pub use error::*;
pub mod fd_util;
pub mod file_util;
pub mod fs_util;
pub mod initrd_util;
pub mod io_util;
pub mod logger;
pub mod macros;
pub mod mount_util;
pub mod naming_scheme;
#[cfg(feature = "network")]
pub mod network;
pub mod os_release;
#[cfg(feature = "parse")]
pub mod parse;
pub mod path_lookup;
pub mod path_util;
pub mod proc_cmdline;
pub mod process_util;
pub mod rlimit_util;
pub mod security;
pub mod show_table;
pub mod signal_util;
pub mod socket_util;
pub mod stat_util;
pub mod string;
#[cfg(feature = "sysfs")]
pub mod sysfs;
#[cfg(feature = "unistd")]
pub mod unistd;
#[cfg(feature = "uuid")]
pub mod uuid;
#[cfg(feature = "virt")]
pub mod virt;

/// default startup target
pub const DEFAULT_TARGET: &str = "default.target";
/// the shutdown target
pub const SHUTDOWN_TARGET: &str = "shutdown.target";
/// the socketc target
pub const SOCKETS_TARGET: &str = "sockets.target";

/// early boot targets
pub const SYSINIT_TARGET: &str = "sysinit.target";
/// the basic start target
pub const BASIC_TARGET: &str = "basic.target";

/// Special user boot targets */
pub const MULTI_USER_TARGET: &str = "multi-user.target";

/// the init scope
pub const INIT_SCOPE: &str = "init.scope";
/// sysmaster service slice
pub const SYSMASTER_SLICE: &str = "system.slice";

/// the unit store sysmaster itself
pub const CGROUP_SYSMASTER: &str = "sysmaster";

/// the default running time directory of sysmaster
pub const EXEC_RUNTIME_PREFIX: &str = "/run";
