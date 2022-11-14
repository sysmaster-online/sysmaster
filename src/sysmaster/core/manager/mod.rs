//!
pub use manager::{Action, Manager, Mode, MANAGER_ARGS_SIZE_MAX};
pub use rentry::{ReliLastFrame,ReliLastQue};
pub use rentry::RELI_HISTORY_MAX_DBS;

/// error number of manager
#[derive(Debug)]
pub enum MngErrno {
    /// invalid input
    Input,
    /// not existed
    NotExisted,
    /// Internal error
    Internal,
    /// not supported
    NotSupported,
}

pub (in crate::core) mod commands;
pub (in crate::core) mod config;
pub (in crate::core) mod manager;
pub (in crate::core) mod pre_install;
pub (in crate::core) mod rentry;
pub (in crate::core) mod signals;
