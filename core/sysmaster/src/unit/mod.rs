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

//!  Unit is the main module for process 1 to manage and abstract system services
//!  The module contains:
//!
//!  `[execute]`: unit Object data structure definition to be executed.
//!
//!  `[job]`: The scheduling execution entity corresponding to the unit. After each unit is started, it will be driven by the job.
//!
//!  `[uload_util]`: Attribute definitions related to each unit configuration file.
//!
//!  `[unit_base]`: Definition of basic attributes of unit related objects, such as enumeration of unit type and definition of unit dependency
//!
//!  `[unit_datastore]`: the unit object storage module is responsible for storing the unit module status.
//!
//!  `[unit_entry]`: Definition of unit related objects
//!
//!  `[unit_manager]`: Manager all Unit Instances in sysmaster
//!
//!  `[um_interface]`: Share api of unit_manager for subunit
pub(crate) use core::unit::{UnitRelations, UnitType};
pub(super) use data::DataManager;
pub(super) use datastore::UnitDb;
pub(super) use entry::UnitX;
pub(super) use manager::UnitManagerX;
pub use rentry::UeConfigInstall;
pub(super) use rentry::{unit_name_to_type, JobMode};

#[cfg(test)]
pub(super) use rentry::UnitRe;
#[cfg(test)]
pub(super) use test::test_utils;

///
#[allow(dead_code)]
#[derive(Debug)]
pub enum UnitErrno {
    ///
    InputErr,
    ///
    NotExisted,
    ///
    InternalErr,
    ///
    NotSupported,
}

// dependency:
// rentry -> data -> base -> {util} ->
// entry -> {datastore -> runtime} -> job -> submanager
// {execute | sigchld | notify} -> {bus -> manager(uload)}

mod base;
mod bus;
mod data;
mod datastore;
mod entry;
mod execute;
mod manager;
mod notify;
mod rentry;
mod runtime;
mod sigchld;
mod submanager;
#[cfg(test)]
mod test;
mod uload;
mod util;
