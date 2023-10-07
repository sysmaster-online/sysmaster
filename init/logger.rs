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

use kernlog::KernelLog;

pub struct Logger;

impl Logger {
    pub fn init(loglevel: log::LevelFilter) {
        match KernelLog::with_level(loglevel) {
            Ok(klog) => {
                log::set_boxed_logger(Box::new(klog)).expect("Failed to set logger!");
                log::set_max_level(loglevel);
            }
            Err(e) => {
                env_logger::builder().filter_level(loglevel).init();
                log::error!("Unsupported log into /dev/kmsg: {}, log into console!", e);
            }
        }
    }
}
