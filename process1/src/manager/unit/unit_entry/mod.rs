//! #  General description
//!  Unit is an abstraction of process1 management objects. All objects can be mapped to a unit. Unit is divided into two major stages in process1
//!  1. Load stage: convert the configuration file into a specific unit object and load it into process1.
//!  2. Execution stage: create unit instance and execute specific actions defined by unit.
//! #  Overall abstraction
//!  Unit is the basic unit abstraction of process1 management. Systemd originally contains 9 types. Process1 supports the expansion of Unit into multiple types. The overall architecture is as follows:
//! ! [avatar][../../../../docs/res/unit_c_diagram.jpg]
//!  It contains two core objects: UnitObj, Unit and the implementation of a sub Unit.
//!  UnitObj is the interface abstraction of subclasses, including the interfaces that subclasses must implement. It is represented by trait in trust. See ['UnitObj '] for specific definitions
//! #  Configuration Item Description
//!  The unit configuration consists of three parts, which are described as follows
//! ``` toml
//!  [Unit]: Configuration items that can be configured for all units. See uu for details_ config::UeConfigUnit
//!  [SelfDefSection]
//!  [Install] Configuration items during unit installation (see subsequent remarks for installation concept)
//! ```
//! #  Load stage design
//!  In the load stage, the unit is loaded from the configuration file into process1, including the creation of the configuration unit object, the resolution of the configuration file, and the filling of the unit object attributes.
//! ##  Unit object creation
//!   Process1 refers to systemd. The preliminary plan includes 9 types of units. The naming rule of each type of configuration file is *. XXX. XXX refers to the specific unit type, such as service, slice, target, etc.
//!  The following modules are included.
//!  u_entry: The interface abstract entity of unit, which is the parent class of all units, can implement the unitObj trait object
//!  uf_interface: The interface is an internally managed entity object that encapsulates the Unit. Only UnitX objects can be seen in process1, but the Unit cannot be seen. The Unit is isolated
//!  uu_load: Encapsulates Unitload Status
//!  uu_child: The child maintains the parent and child processes associated with the unit. The child services associated with the unit may start the child processes. Therefore, it is necessary to maintain the processes associated with the unit.
//!  uu_cgroup: cgroup related configurations
//!  uu_config is the configuration of unit
//!
pub use u_entry::{Unit, UnitObj};
pub(in crate::manager) use uf_interface::UnitX;

// dependency:
// uu_condition ->
// uu_base -> {uu_config | uu_cgroup} -> {uu_load | uu_child} ->
// u_entry -> uf_interface
mod u_entry;
mod uf_interface;
mod uu_base;
mod uu_cgroup;
mod uu_child;
mod uu_condition;
mod uu_config;
mod uu_load;
