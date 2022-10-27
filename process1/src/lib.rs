//!
extern crate strum;

pub use reliability::{
    reli_debug_get_switch, reli_dir_prepare, ReDb, ReDbRoTxn, ReDbRwTxn, ReDbTable, ReStation,
    ReStationKind, Reliability,
};

#[macro_use]
extern crate lazy_static;

pub mod manager;
pub mod mount;
pub mod plugin;
pub mod proto;
mod reliability;
