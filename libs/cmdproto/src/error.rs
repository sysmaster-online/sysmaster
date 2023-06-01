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
#[allow(unused_imports)]
pub use snafu::ResultExt;

/// cmdproto Error
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

    #[snafu(display("SendStreamFailed"))]
    SendStream { msg: String },

    #[snafu(display("ManagerStartFailed"))]
    ManagerStart { msg: String },
}

/// new Result
#[allow(dead_code)]
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Set this byte of your error code if you want to print it to stdout
pub const ERROR_CODE_MASK_PRINT_STDOUT: u32 = 1 << 10;
