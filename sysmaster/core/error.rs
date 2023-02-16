//! Error define
use snafu::prelude::*;
#[allow(unused_imports)]
pub use snafu::ResultExt;

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum JobErrno {
    Input,
    Conflict,
    NotExisted,
    Internal,
    NotSupported,
    BadRequest,
}

use sysmaster::error::MngErrno;
impl From<JobErrno> for MngErrno {
    fn from(err: JobErrno) -> Self {
        match err {
            JobErrno::Input => MngErrno::Input,
            JobErrno::NotExisted => MngErrno::NotExisted,
            JobErrno::NotSupported => MngErrno::NotSupported,
            _ => MngErrno::Internal,
        }
    }
}

/// sysmaster Error
#[allow(missing_docs)]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum Error {
    #[snafu(display("utils error"))]
    Utils { source: libutils::error::Error },

    #[snafu(display("sysmaster error"))]
    Sysmaster { source: sysmaster::error::Error },

    #[snafu(display("nix errno"))]
    Nix { source: nix::errno::Errno },

    #[snafu(display("io error"))]
    Io { source: std::io::Error },

    #[snafu(display("plugin load error"))]
    PluginLoad { msg: String },

    #[snafu(display("job error:{}", source))]
    JobErrno { source: JobErrno },

    #[snafu(display("other error:'{}'", msg))]
    Other { msg: String },
}

/// new Result
#[allow(dead_code)]
pub type Result<T, E = Error> = std::result::Result<T, E>;
