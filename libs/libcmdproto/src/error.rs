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

    #[snafu(display("EncodeError"))]
    Encode { source: prost::EncodeError },

    #[snafu(display("DecodeError"))]
    Decode { source: prost::DecodeError },

    #[snafu(display("ReadStreamFailed"))]
    ReadStream { msg: String },

    #[snafu(display("ManagerStartFailed"))]
    ManagerStart { msg: String },
}

/// new Result
#[allow(dead_code)]
pub type Result<T, E = Error> = std::result::Result<T, E>;
