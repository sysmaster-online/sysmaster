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
use crate::error::*;
use std::env;

///
pub fn env_path() -> Result<String> {
    let path = env::var("OUT_DIR")
        .or_else(|_| env::var("LD_LIBRARY_PATH"))
        .context(VarSnafu)?;

    let out_dir = path.split(':').collect::<Vec<_>>()[0]
        .split("target")
        .collect::<Vec<_>>()[0]
        .to_string();
    let tmp_str: Vec<_> = out_dir.split("build").collect();
    if tmp_str.is_empty() {
        return Err(Error::Other {
            msg: "not running with cargo".to_string(),
        });
    }

    Ok(tmp_str[0].to_string())
}
