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

//!

use crate::uuid::Uuid;
use bitflags::bitflags;

bitflags! {
    /// GPT attribute
    pub struct GptAttribute: u64 {
        /// growfs
        const GPT_FLAG_GROWFS = 1 << 59;
        /// read only
        const GPT_FLAG_READ_ONLY = 1 << 60;
        /// no auto mount
        const GPT_FLAG_NO_AUTO = 1 << 63;
    }
}

/// GPT_ESP UUID type
pub const GPT_ESP: Uuid = Uuid([
    0xc1, 0x2a, 0x73, 0x28, 0xf8, 0x1f, 0x11, 0xd2, 0xba, 0x4b, 0x00, 0xa0, 0xc9, 0x3e, 0xc9, 0x3b,
]);
/// GPT_XBOOTLDR UUID type
pub const GPT_XBOOTLDR: Uuid = Uuid([
    0xbc, 0x13, 0xc2, 0xff, 0x59, 0xe6, 0x42, 0x62, 0xa3, 0x52, 0xb2, 0x75, 0xfd, 0x6f, 0x71, 0x72,
]);
#[cfg(target_arch = "aarch64")]
/// GPT_ROOT_NATIVE UUID type
pub const GPT_ROOT_NATIVE: Uuid = Uuid([
    0xb9, 0x21, 0xb0, 0x45, 0x1d, 0xf0, 0x41, 0xc3, 0xaf, 0x44, 0x4c, 0x6f, 0x28, 0x0d, 0x3f, 0xae,
]);
#[cfg(target_arch = "x86_64")]
/// GPT_ROOT_NATIVE UUID type
pub const GPT_ROOT_NATIVE: Uuid = Uuid([
    0x69, 0xda, 0xd7, 0x10, 0x2c, 0xe4, 0x4e, 0x3c, 0xb1, 0x6c, 0x21, 0xa1, 0xd4, 0x9a, 0xbe, 0xd3,
]);
