pub use manager::{Action, Manager, Mode, MANAGER_ARGS_SIZE_MAX};

#[cfg(test)]
pub(in crate::core) use rentry::RELI_HISTORY_MAX_DBS;

pub(in crate::core) mod commands;
pub(in crate::core) mod config;
pub(in crate::core) mod manager;
pub(in crate::core) mod pre_install;
pub(in crate::core) mod rentry;
pub(in crate::core) mod signals;
