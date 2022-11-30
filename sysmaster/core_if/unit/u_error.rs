use libcmdproto::proto::execute::ExecCmdErrno;

///Unit Action Error Num
///
#[derive(Debug)]
pub enum UnitActionError {
    ///
    UnitActionEAgain,
    ///
    UnitActionEAlready,
    ///
    UnitActionEComm,
    ///
    UnitActionEBadR,
    ///
    UnitActionENoExec,
    ///
    UnitActionEProto,
    ///
    UnitActionEOpNotSupp,
    ///
    UnitActionENolink,
    ///
    UnitActionEStale,
    ///
    UnitActionEFailed,
    ///
    UnitActionEInval,
    ///
    UnitActionEBusy,
    ///
    UnitActionENoent,
    ///
    UnitActionECanceled,
}

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
