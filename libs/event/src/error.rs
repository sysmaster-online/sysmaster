// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! Error define
use snafu::prelude::*;

/// Event Error
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Error {
    #[snafu(display("Error(event): Got an error: {:?}", source))]
    Io { source: std::io::Error },
    #[snafu(display("Error(event): Nix error: {}", source))]
    Nix { source: nix::Error },
    #[snafu(display("Error(event): '{}'.", word))]
    Other { word: &'static str },
    #[snafu(display(
        "Error(event): Got an error: (ret={}, errno={}) for syscall: {}",
        ret,
        errno,
        syscall
    ))]
    Syscall {
        syscall: &'static str,
        ret: i32,
        errno: i32,
    },
}

/// new Result
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error() {
        let error = Error::Io {
            source: std::io::Error::new(std::io::ErrorKind::Other, "test error"),
        };
        assert_eq!(
            error.to_string(),
            "Error(event): Got an error: Custom { kind: Other, error: \"test error\" }"
        );
    }

    #[test]
    fn test_nix_error() {
        let error = Error::Nix {
            source: nix::errno::Errno::EINVAL,
        };
        assert_eq!(
            error.to_string(),
            "Error(event): Nix error: EINVAL: Invalid argument"
        );
    }

    #[test]
    fn test_other_error() {
        let error = Error::Other { word: "test" };
        assert_eq!(error.to_string(), "Error(event): 'test'.");
    }

    #[test]
    fn test_syscall_error() {
        let error = Error::Syscall {
            syscall: "test_syscall",
            ret: -1,
            errno: 123,
        };
        assert_eq!(
            error.to_string(),
            "Error(event): Got an error: (ret=-1, errno=123) for syscall: test_syscall"
        );
    }
}
