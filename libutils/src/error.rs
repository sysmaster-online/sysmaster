enum ErrKind {
    Protocol,
    Env,
    Unit,
    Syscall,
}

impl std::fmt::Display for ErrKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err_kind = match self {
            ErrKind::Protocol => "protocol",
            ErrKind::Env => "env",
            ErrKind::Unit => "unit",
            ErrKind::Syscall => "syscall",
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
    #[error("{}: There was an error writing the cargo instructions to stdout: {}", ErrKind::Protocol, .0)]
    Io(#[from] std::io::Error),

    /// An error getting the 'CARGO_PKG_VERSION' environment variable
    #[error("{}: The 'CARGO_PKG_VERSION' environment variable may not be set: {}", ErrKind::Env, .0)]
    Var(#[from] std::env::VarError),

    /// An error getting the current pid
    #[error(
        "{}: Unable to determine the current process pid: {}",
        ErrKind::Protocol,
        msg
    )]
    Pid { msg: &'static str },

    /// An error getting the current pid
    #[error("{}: Got an error: {} for unit: {}", ErrKind::Unit, msg, unit)]
    Unit {
        msg: &'static str,
        unit: &'static str,
    },
}

pub type Result<T, E = Error> = anyhow::Result<T, E>;

#[cfg(test)]
mod test {
    use super::Error;
    use std::{
        env,
        io::{self, ErrorKind},
    };

    #[test]
    fn io_error() {
        let err: Error = io::Error::new(ErrorKind::Other, "testing").into();
        assert_eq!(
            "protocol: There was an error writing the cargo instructions to stdout: testing",
            format!("{}", err)
        );
    }

    #[test]
    fn pid_error() {
        let err: Error = Error::Pid { msg: "test" };
        assert_eq!(
            "protocol: Unable to determine the current process pid: test",
            format!("{}", err)
        );
    }

    #[test]
    fn var_error() {
        let res = env::var("yoda").map_err(Error::from);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert_eq!(
            "env: The \'CARGO_PKG_VERSION\' environment variable may not be set: environment variable not found",
            format!("{}", err)
        );
    }
}
