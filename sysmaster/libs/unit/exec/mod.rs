//! execute module
//////#![allow(missing_docs)]
mod base;
mod cmd;
pub use base::{ExecCmdError, ExecContext, ExecFlags, ExecParameters};
pub use cmd::ExecCommand;
