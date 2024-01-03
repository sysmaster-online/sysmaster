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
use std::path::PathBuf;

use basic::fs::parse_absolute_path;
use serde::{
    de::{self, Unexpected},
    Deserialize, Deserializer,
};

///
pub trait DeserializeWith: Sized {
    /// Item which  deserialize_with return
    type Item;
    ///
    fn deserialize_with<'de, D>(de: D) -> Result<Self::Item, D::Error>
    where
        D: Deserializer<'de>;
}

impl DeserializeWith for Vec<String> {
    type Item = Self;
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;
        let mut vec = Vec::new();

        for l in s.split_terminator(';') {
            vec.push(l.trim().to_string());
        }

        Ok(vec)
    }
}

impl DeserializeWith for Vec<PathBuf> {
    type Item = Self;

    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;
        let mut res: Vec<PathBuf> = Vec::new();
        if s.is_empty() {
            return Ok(res);
        }
        for p in s.split_terminator(';') {
            if parse_absolute_path(p).is_err() {
                return Err(de::Error::invalid_value(Unexpected::Str(p), &""));
            }
            res.push(PathBuf::from(p));
        }
        Ok(res)
    }
}
