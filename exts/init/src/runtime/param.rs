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

const TIME_OUT: i64 = 5;
const TIME_WAIT: i64 = 10;

pub struct Param {
    pub init_args: Vec<String>,
    pub manager_args: Vec<String>,

    pub time_out: i64,
    pub time_wait: i64,
}

impl Param {
    pub fn new() -> Self {
        Param {
            init_args: Vec::<String>::with_capacity(0),
            manager_args: Vec::<String>::with_capacity(0),
            time_out: TIME_OUT,
            time_wait: TIME_WAIT,
        }
    }

    pub fn get_opt(&mut self) {
        for param in &self.init_args {
            if let Some(size) = param.find("--timeout=") {
                let len = size + str::len("--timeout=");
                let str1 = &param[len..];
                match str1.parse::<i64>() {
                    Ok(time) => self.time_out = time,
                    Err(_) => {
                        println!(
                            "Failed to parse timeout, using the default timeout:{:?}",
                            TIME_OUT
                        );
                        self.time_out = TIME_OUT;
                    }
                }
            }

            if let Some(size) = param.find("--timewait=") {
                let len = size + str::len("--timewait=");
                let str1 = &param[len..];
                match str1.parse::<i64>() {
                    Ok(time) => self.time_wait = time,
                    Err(_) => {
                        println!(
                            "Failed to parse timewait, using the default timewait:{:?}",
                            TIME_WAIT
                        );
                        self.time_wait = TIME_WAIT;
                    }
                }
            }
        }
    }
}
