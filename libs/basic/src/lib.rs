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
#[cfg(feature = "cpu")]
pub mod cpu;
#[cfg(feature = "disk")]
pub mod disk;
#[cfg(feature = "fd")]
pub mod fd_util;
#[cfg(feature = "fs")]
pub mod fs_util;
#[cfg(feature = "host")]
pub mod host;
#[cfg(feature = "io")]
pub mod io_util;
#[cfg(feature = "logger")]
pub mod logger;
#[cfg(feature = "machine")]
pub mod machine;
pub mod macros;
#[cfg(feature = "memory")]
pub mod memory;
#[cfg(feature = "mount")]
pub mod mount_util;
#[cfg(feature = "naming_scheme")]
pub mod naming_scheme;
#[cfg(feature = "network")]
pub mod network;
#[cfg(feature = "os_release")]
pub mod os_release;
#[cfg(feature = "parse")]
pub mod parse;
#[cfg(feature = "cmdline")]
pub mod proc_cmdline;
#[cfg(feature = "process")]
pub mod process;
#[cfg(feature = "rlimit")]
pub mod rlimit;
#[cfg(feature = "security")]
pub mod security;
#[cfg(feature = "sensors")]
pub mod sensors;
#[cfg(feature = "show_table")]
pub mod show_table;
#[cfg(feature = "signal")]
pub mod signal_util;
#[cfg(feature = "socket")]
pub mod socket_util;
#[cfg(feature = "stat")]
pub mod stat_util;
#[cfg(feature = "string")]
pub mod string;
#[cfg(feature = "sysfs")]
pub mod sysfs;
#[cfg(feature = "unistd")]
pub mod unistd;
#[cfg(feature = "uuid")]
pub mod uuid;

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
