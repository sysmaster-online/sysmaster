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

//! Path is one of the unit types supported in sysmaster. Path units use the inotify(7) API to monitor file systems, and pull up the corresponding service when the conditions are met
//! The Path configuration file contains three sections: Unit,Path,and Install.
//!
//! # Example:
//! ``` toml
//! [Unit]
//! Description=test path
//!
//! [Path]
//! PathExists=/tmp/PathExists
//! PathExistsGlob=/tmp/PathExistsGlo*
//! PathChanged=/tmp/PathChanged
//! PathModified=/tmp/PathModified
//! DirectoryNotEmpty=/tmp/DirectoryNotEmpty
//! Unit=test.service
//! MakeDirectory=yes
//! DirectoryMode=0644
//!
//! [Install]
//! WantedBy="paths.target"
//! ```
//! `[Path]` section related configuration
//!

#[cfg(all(feature = "plugin", feature = "noplugin"))]
compile_error!("feature plugin and noplugin cannot be enabled at the same time");

pub use {manager::__um_obj_create, unit::__subunit_create_with_params};

// dependency:
// base -> rentry -> {comm | config}
// mng -> unit -> manager
mod base;
mod bus;
mod comm;
mod config;
mod manager;
mod mng;
mod rentry;
mod unit;
