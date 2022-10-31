//! # Target is the entry of sysmaster's control startup mode. The earliest concept of startup mode comes from the concept of sysvint in Linux system. In sysvinit, startup mode includes 0-6 6 modes
//!  Sysmaster refers to systemd, and uses target as the entry of the startup mode. It is the unit that sysmaster loads by default during startup. Target has no actual action to execute,
//!  Target can be understood as the logical grouping of units to be started during system startup
//!  The target configuration file does not have its own private configuration item and only contains Unit/Install
//! #  Example:
//! ``` toml
//!  [Unit]
//!  Description = ""
//!
//!  [Install]
//!  WantedBy =
//! ```
//! ##  Automatic dependency
//!
//! ###  Implicit dependency
//!  No implicit dependencies
//!
//! ###  Default Dependency
//!  If DefaultDependencies=no is set, the following dependencies will be added by default:
//! +  Yes XXX
//! +  Conflicts = shutdown. Dependence between target and Before=shutdown.target

// dependency: target_base -> target_rentry -> target_comm -> {target_mng} -> target_unit -> target_manager
mod target_base;
mod target_comm;
mod target_manager;
mod target_mng;
mod target_rentry;
mod target_unit;
