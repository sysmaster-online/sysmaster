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

//! error definitions
use snafu::prelude::*;
#[allow(unused_imports)]
pub use snafu::ResultExt;

#[allow(missing_docs)]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum Error {
    #[snafu(display(
        "Got an error: (ret={}, errno={}) for syscall: {}",
        ret,
        errno,
        syscall
    ))]
    Syscall {
        syscall: &'static str,
        ret: i32,
        errno: i32,
    },

    #[snafu(display("Io"))]
    Io { source: std::io::Error },

    #[snafu(display("Errno"))]
    Nix { source: nix::Error },

    #[snafu(display("Var"))]
    Var { source: std::env::VarError },

    #[snafu(display("procfs"))]
    Proc { source: procfs::ProcError },

    #[snafu(display("Error parsing from string: {}", source))]
    Parse {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[snafu(display("Not exist): '{}'.", what))]
    NotExisted { what: String },

    #[snafu(display("Invalid: '{}'.", what))]
    Invalid { what: String },

    #[snafu(display("OtherError): '{}'.", msg))]
    Other { msg: String },
}

#[allow(unused_macros)]
macro_rules! errfrom {
    ($($st:ty),* => $variant:ident) => (
        $(
            impl From<$st> for Error {
                fn from(e: $st) -> Error {
                    Error::$variant { source: e.into() }
                }
            }
        )*
    )
}

errfrom!(std::num::ParseIntError, std::string::ParseError, std::num::ParseFloatError, std::str::ParseBoolError, std::string::FromUtf8Error => Parse);

///
pub type Result<T, E = Error> = std::result::Result<T, E>;
