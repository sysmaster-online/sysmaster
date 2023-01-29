//! Error define
use snafu::prelude::*;

/// Event Error
#[derive(Debug, Snafu)]
pub enum Error {
    /// An error from syscall
    #[snafu(display(
        "Error(libevent): : Got an error: (ret={}, errno={}) for syscall: {}",
        ret,
        errno,
        syscall
    ))]
    Syscall {
        /// string representation
        syscall: &'static str,
        /// return value
        ret: i32,
        /// errno
        errno: i32,
    },

    /// Other
    #[snafu(display("Error(libevent): '{}'.", word))]
    Other {
        /// some words
        word: &'static str,
    },
}

/// new Result
pub type Result<T, E = Error> = std::result::Result<T, E>;
