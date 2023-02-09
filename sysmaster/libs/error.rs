//! Error define
use libcmdproto::proto::execute::ExecCmdErrno;
use snafu::prelude::*;

/// UnitAction Error
#[allow(missing_docs)]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum UnitActionError {
    #[snafu(display("EAgain(UnitActionError)"))]
    UnitActionEAgain,
    #[snafu(display("EAlready(UnitActionError)"))]
    UnitActionEAlready,
    #[snafu(display("EComm(UnitActionError)"))]
    UnitActionEComm,
    #[snafu(display("EBadR(UnitActionError)"))]
    UnitActionEBadR,
    #[snafu(display("ENoExec(UnitActionError)"))]
    UnitActionENoExec,
    #[snafu(display("EProto(UnitActionError)"))]
    UnitActionEProto,
    #[snafu(display("EOpNotSupp(UnitActionError)"))]
    UnitActionEOpNotSupp,
    #[snafu(display("ENolink(UnitActionError)"))]
    UnitActionENolink,
    #[snafu(display("EStale(UnitActionError)"))]
    UnitActionEStale,
    #[snafu(display("EFailed(UnitActionError)"))]
    UnitActionEFailed,
    #[snafu(display("EInval(UnitActionError)"))]
    UnitActionEInval,
    #[snafu(display("EBusy(UnitActionError)"))]
    UnitActionEBusy,
    #[snafu(display("ENoent(UnitActionError)"))]
    UnitActionENoent,
    #[snafu(display("ECanceled(UnitActionError)"))]
    UnitActionECanceled,
}

/// error number of manager
#[allow(missing_docs)]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum MngErrno {
    #[snafu(display("Input(ManagerError)"))]
    Input,
    #[snafu(display("NotExisted(ManagerError)"))]
    NotExisted,
    #[snafu(display("Internal(ManagerError)"))]
    Internal,
    #[snafu(display("NotSupported(ManagerError)"))]
    NotSupported,
}

impl From<MngErrno> for ExecCmdErrno {
    fn from(err: MngErrno) -> Self {
        match err {
            MngErrno::Input => ExecCmdErrno::Input,
            MngErrno::NotExisted => ExecCmdErrno::NotExisted,
            MngErrno::NotSupported => ExecCmdErrno::NotSupported,
            _ => ExecCmdErrno::Internal,
        }
    }
}

/// libsysmaster Error
#[allow(missing_docs)]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum Error {
    #[snafu(display("UnitActionError(libsysmaster)"))]
    UnitAction { source: UnitActionError },

    #[snafu(display("ManagerError(libsysmaster)"))]
    Manager { source: MngErrno },

    /// Other
    #[snafu(display("OtherError(libsysmaster): '{}'.", word))]
    Other {
        /// some words
        word: &'static str,
    },
}

/// new Result
pub type Result<T, E = Error> = std::result::Result<T, E>;
