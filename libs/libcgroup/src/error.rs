//! Error define
use snafu::prelude::*;
#[allow(unused_imports)]
pub use snafu::ResultExt;

/// Libcmdproto Error
#[allow(missing_docs)]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum Error {
    #[snafu(display("IoError"))]
    Io { source: std::io::Error },

    #[snafu(display("NixErrno"))]
    Nix { source: nix::errno::Errno },

    #[snafu(display("ManagerStartFailed"))]
    ManagerStart { msg: String },

    #[snafu(display("NotSupported"))]
    NotSupported,

    #[snafu(display("NotFound:{}", what))]
    NotFound { what: String },

    #[snafu(display("ReadLineError:{}", line))]
    ReadLine { line: String },

    #[snafu(display("DataFormatError:{}", data))]
    DataFormat { data: String },

    #[snafu(display("KillControlService:{}", what))]
    KillControlService { what: String },

    #[snafu(display("NotADirectory:{}", path))]
    NotADirectory { path: String },

    #[snafu(display("ParseError"))]
    ParseInt { source: std::num::ParseIntError },
}

/// new Result
#[allow(dead_code)]
pub type Result<T, E = Error> = std::result::Result<T, E>;
