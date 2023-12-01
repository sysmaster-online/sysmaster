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

//! Mount is the entry for the mount point monitoring of sysmaster. sysmaster refers to systemd, but it is not the same.
//! sysmaster mainly provides the monitoring function and does not actively mount/unmount the mount point, which is implemented by other processes.
//! Mount does not support configuration files.
//!
//! ## Automatic dependency
//! NA
//! ### Implicit dependency
//! NA
//! ### Default Dependency
//! NA

#[cfg(all(feature = "plugin", feature = "noplugin"))]
compile_error!("feature plugin and noplugin cannot be enabled at the same time");

pub use {manager::__um_obj_create, unit::__subunit_create_with_params};

// dependency: mount_base -> mount_rentry -> mount_comm -> {mount_mng -> mount_unit} -> mount_manager
mod base;
mod comm;
mod config;
mod manager;
mod mng;
mod rentry;
mod spawn;
mod unit;
