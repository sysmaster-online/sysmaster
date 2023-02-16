//! execute module
mod base;
mod cmd;
pub use crate::error::ExecCmdError;
pub use base::{ExecContext, ExecFlags, ExecParameters};
pub use cmd::ExecCommand;
