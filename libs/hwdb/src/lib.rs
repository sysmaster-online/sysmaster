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

//! lib of hwdb

/// Process the raw data of hwdb.bin
pub mod sd_hwdb;

/// Generic devmaster properties, key-value database based on modalias strings.
/// Uses a Patricia/radix trie to index all matches for efficient lookup.
pub mod hwdb_util;

pub use crate::hwdb_util::*;
pub use crate::sd_hwdb::*;
