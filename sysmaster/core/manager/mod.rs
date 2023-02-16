pub use manager::{Action, Manager, Mode, MANAGER_ARGS_SIZE_MAX};

#[cfg(test)]
pub(crate) use rentry::RELI_HISTORY_MAX_DBS;

pub(crate) mod commands;
pub(crate) mod config;
pub(crate) mod manager;
pub(crate) mod pre_install;
pub(crate) mod rentry;
pub(crate) mod signals;
