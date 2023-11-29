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
use crate::Result;
use hwdb::HwdbUtil;

pub struct HwdbArgs {
    update: bool,
    test: Option<String>,
    path: Option<String>,
    usr: bool,
    strict: Option<bool>,
    root: Option<String>,
}

impl HwdbArgs {
    pub fn new(
        update: bool,
        test: Option<String>,
        path: Option<String>,
        usr: bool,
        strict: Option<bool>,
        root: Option<String>,
    ) -> Self {
        HwdbArgs {
            update,
            test,
            path,
            usr,
            strict,
            root,
        }
    }

    /// subcommand for update or query the hardware database.
    pub fn subcommand(&self) -> Result<()> {
        if !self.update && self.test.is_none() {
            log::error!("Either --update or --test must be used.");
            return Err(nix::Error::EINVAL);
        }

        log::warn!("devctl hwdb is deprecated. Use sysmaster-hwdb instead.");

        if self.update {
            let s = self.strict.unwrap_or(false);
            if self.usr {
                HwdbUtil::update(
                    self.path.clone(),
                    self.root.clone(),
                    Some("/usr/lib/devmaster/".to_string()),
                    s,
                    true,
                )?;
            } else {
                HwdbUtil::update(self.path.clone(), self.root.clone(), None, s, true)?;
            }
        }

        if let Some(modalias) = &self.test {
            HwdbUtil::query(modalias.to_string(), None)?;
        }

        Ok(())
    }
}
