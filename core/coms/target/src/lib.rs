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

//! # Target is the entry of sysmaster's control startup mode. The earliest concept of startup mode comes from the concept of sysvint in Linux system. In sysvinit, startup mode includes 0-6 6 modes
//!  Sysmaster refers to systemd, and uses target as the entry of the startup mode. It is the unit that sysmaster loads by default during startup. Target has no actual action to execute,
//!  Target can be understood as the logical grouping of units to be started during system startup
//!  The target configuration file does not have its own private configuration item and only contains Unit/Install
//! #  Example:
//! ``` toml
//!  [Unit]
//!  Description=""
//!
//!  [Install]
//!  WantedBy=
//! ```
//! ##  Automatic dependency
//!
//! ###  Implicit dependency
//!  No implicit dependencies
//!
//! ###  Default Dependency
//!  If DefaultDependencies=true is set, the following dependencies will be added by default:
//!  Conflicts="shutdown.target", Beforet="shutdown.target"

#[cfg(all(feature = "plugin", feature = "noplugin"))]
compile_error!("feature plugin and noplugin cannot be enabled at the same time");

pub use {manager::__um_obj_create, unit::__subunit_create_with_params};

// dependency: target_base -> target_rentry -> target_comm -> {target_mng} -> target_unit -> target_manager
mod base;
mod comm;
mod manager;
mod mng;
mod rentry;
mod unit;
