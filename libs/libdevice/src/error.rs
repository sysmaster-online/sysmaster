//! Error definition of libdevice
//!
use nix::errno::Errno;
use snafu::prelude::*;

/// libdevice error
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum Error {
    /// error from syscall
    #[snafu(display(
        "Error(libdevice): Got an error for syscall {} (errno={})",
        syscall,
        errno,
    ))]
    Syscall {
        /// syscall
        syscall: String,
        /// errno
        errno: Errno,
    },

    /// other error
    #[snafu(display("Error(libdevice): Got an error {}", msg,))]
    Other {
        /// message
        msg: String,
        /// errno
        errno: Option<Errno>,
    },
}
