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

//! encapsulate loopdev crate to provide unit test support
//!

use crate::error::*;
use basic::ResultExt;
use loopdev::*;
use std::path::PathBuf;

pub(crate) struct LoopDev {
    tmpfile: String,
    lodev: LoopDevice,
}

impl LoopDev {
    /// create a temporate file with specific size
    #[allow(dead_code)]
    pub(crate) fn new(tmpfile: &str, size: u64) -> Result<Self, Error> {
        let file = std::fs::File::create(tmpfile).context(IoSnafu {
            filename: tmpfile.to_string(),
        })?;
        file.set_len(size).context(IoSnafu {
            filename: tmpfile.to_string(),
        })?;

        let lc = loopdev::LoopControl::open().context(IoSnafu {
            filename: tmpfile.to_string(),
        })?;
        let ld = lc.next_free().context(IoSnafu {
            filename: tmpfile.to_string(),
        })?;

        ld.with()
            .part_scan(true)
            .offset(0)
            .size_limit(size)
            .attach(tmpfile)
            .context(IoSnafu {
                filename: tmpfile.to_string(),
            })?;

        Ok(LoopDev {
            tmpfile: tmpfile.to_string(),
            lodev: ld,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn get_device_path(&self) -> Option<PathBuf> {
        self.lodev.path()
    }
}

impl Drop for LoopDev {
    fn drop(&mut self) {
        let _ = self.lodev.detach();
        let _ = std::fs::remove_file(self.tmpfile.as_str());
    }
}
