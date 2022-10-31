#[allow(dead_code)]
enum ErrKind {
    Unit,
    Syscall,
    Http,
    Proc,
    ParseInt,
    ParseFloat,
    FromUTF8,
    ParseBoolean,
    Other,
}

impl std::fmt::Display for ErrKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err_kind = match self {
            ErrKind::Unit => "unit",
            ErrKind::Syscall => "syscall",
            ErrKind::Http => "http",
            ErrKind::Proc => "procfs",
            ErrKind::ParseInt => "parseint",
            ErrKind::FromUTF8 => "fromutf8",
            ErrKind::Other => "other",
            ErrKind::ParseBoolean => "parse_boolean",
            ErrKind::ParseFloat => "parsefloat",
        };
        write!(f, "{}", err_kind)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error from syscall
    #[error(
        "{}: Got an error: (ret={}, errno={}) for syscall: {}",
        ErrKind::Syscall,
        ret,
        errno,
        syscall
    )]
    Syscall {
        syscall: &'static str,
        ret: i32,
        errno: i32,
    },

    /// An error writing the cargo instructions to stdout
    #[error("{}: There was an error writing the cargo instructions to stdout: {}", ErrKind::Unit, .0)]
    Io(#[from] std::io::Error),

    /// An error from procfs
    #[error("{}: Got an error from: {}", ErrKind::Proc, .0)]
    Proc(#[from] procfs::ProcError),

    /// An error from parse int
    #[error("{}: Got an error from: {}", ErrKind::ParseInt, .0)]
    ParseInt(#[from] std::num::ParseIntError),

    /// An error from parse float
    #[error("{}: Got an error from: {}", ErrKind::ParseFloat, .0)]
    ParseFloat(#[from] std::num::ParseFloatError),

    /// An error from parse string to boolean
    #[error("{}: Got an error from: {}", ErrKind::ParseBoolean, .0)]
    ParseBoolError(String),

    /// An error from utf8
    #[error("{}: Got an error from: {}", ErrKind::FromUTF8, .0)]
    FromUTF8(#[from] std::string::FromUtf8Error),

    /// An error getting the current pid
    #[error("{}: Got an error: {} for unit: {}", ErrKind::Unit, msg, unit)]
    Unit {
        msg: &'static str,
        unit: &'static str,
    },

    /// An error getting the current pid
    #[error(
        "{}: Unable to determine the current process pid: {}",
        ErrKind::Other,
        msg
    )]
    Other { msg: &'static str },
}

pub type Result<T, E = Error> = anyhow::Result<T, E>;

#[cfg(test)]
mod test {
    use std::io::{self, ErrorKind};

    use super::Error;

    #[test]
    fn io_error() {
        let err: Error = io::Error::new(ErrorKind::Other, "testing").into();
        assert_eq!(
            "unit: There was an error writing the cargo instructions to stdout: testing",
            format!("{}", err)
        );
    }
}
