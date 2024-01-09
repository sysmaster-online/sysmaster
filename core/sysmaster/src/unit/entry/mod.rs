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

//! #  General description
//!  Unit is an abstraction of sysmaster management objects. All objects can be mapped to a unit. Unit is divided into two major stages in sysmaster
//!  1. Load stage: convert the configuration file into a specific unit object and load it into sysmaster.
//!  2. Execution stage: create unit instance and execute specific actions defined by unit.
//! #  Overall abstraction
//!  Unit is the basic unit abstraction of sysmaster management. Systemd originally contains 9 types. sysmaster supports the expansion of Unit into multiple types. The overall architecture is as follows:
//! ! [avatar][../../../../docs/assets/unit_c_diagram.jpg]
//!  It contains two core objects: SubUnit, Unit and the implementation of a sub Unit.
//!  SubUnit is the interface abstraction of subclasses, including the interfaces that subclasses must implement. It is represented by trait in trust. See ['SubUnit'] for specific definitions
//! #  Configuration Item Description
//!  The unit configuration consists of three parts, which are described as follows
//! ``` toml
//!  [Unit]: Configuration items that can be configured for all units. See uu for details_ config::UeConfigUnit
//!  [SelfDefSection]
//!  [Install] Configuration items during unit installation (see subsequent remarks for installation concept)
//! ```
//! #  Load stage design
//!  In the load stage, the unit is loaded from the configuration file into sysmaster, including the creation of the configuration unit object, the resolution of the configuration file, and the filling of the unit object attributes.
//! ##  Unit object creation
//!   sysmaster refers to systemd. The preliminary plan includes 9 types of units. The naming rule of each type of configuration file is *. XXX. XXX refers to the specific unit type, such as service, slice, target, etc.
//!  The following modules are included.
//!  u_entry: The interface abstract entity of unit, which is the parent class of all units, can implement the SubUnit trait object
//!  unitx: The interface is an internally managed entity object that encapsulates the Unit. Only UnitX objects can be seen in sysmaster, but the Unit cannot be seen. The Unit is isolated
//!  uu_load: Encapsulates Unitload Status
//!  uu_child: The child maintains the parent and child processes associated with the unit. The child services associated with the unit may start the child processes. Therefore, it is necessary to maintain the processes associated with the unit.
//!  uu_cgroup: cgroup related configurations
//!  uu_config is the configuration of unit
//!
pub(crate) use config::UnitEmergencyAction;
pub(crate) use ratelimit::StartLimitResult;
pub use uentry::Unit;
pub(crate) use unitx::UnitX;
// pub(super) use uu_config::UnitConfigItem;

// dependency:
// condition ->
// base -> {config | cgroup} -> {load | child | ratelimit} ->
// {bus -> uentry} -> {unitx}
mod base;
mod bus;
mod cgroup;
mod child;
mod condition;
mod config;
mod load;
mod ratelimit;
mod uentry;
mod unitx;
