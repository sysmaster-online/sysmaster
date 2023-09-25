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

//! subcommand for devctl trigger
//!
use hwdb::HwdbUtil;

/// subcommand for hwdb a fake device action, then the kernel will report an uevent
pub fn subcommand_hwdb(
    update: bool,
    test: Option<String>,
    path: Option<String>,
    usr: bool,
    strict: Option<bool>,
    root: Option<String>,
) {
    if !update && test.is_none() {
        eprintln!("Either --update or --test must be used.");
        return;
    }

    if update {
        let s = strict.unwrap_or(false);
        if usr {
            let _ = HwdbUtil::update(path, root, Some("/usr/lib/devmaster/".to_string()), s, true);
        } else {
            let _ = HwdbUtil::update(path, root, None, s, true);
        }
    }

    if let Some(modalias) = test {
        let _ = HwdbUtil::query(modalias, None);
    }
}
