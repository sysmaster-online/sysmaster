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
const DEFAULT_TIMECNT: i64 = 5;
const DEFAULT_TIMEWAIT: i64 = 10;
const INIT_PARAM: i32 = 0;
// const SYSMASTER_PARAM: i32 = 1;
type Callback = fn(arg: &str, key: &String, init_param: &mut InitParam);

struct Dispatch<'a> {
    key: &'a str,
    param_type: i32, // The parameter type of init is INIT_PARAM.
    callback: Option<Callback>,
}

const PARAM_TABLE: &[Dispatch] = &[
    Dispatch {
        key: "--timecnt=",
        param_type: INIT_PARAM,
        callback: Some(parse_timecnt),
    },
    Dispatch {
        key: "--timewait=",
        param_type: INIT_PARAM,
        callback: Some(parse_timewait),
    },
];

fn parse_timecnt(arg: &str, key: &String, init_param: &mut InitParam) {
    let str1 = &arg[key.len()..];
    if let Ok(value) = str1.parse::<i64>() {
        if value >= 2 {
            init_param.time_cnt = value;
        }
    }
}

fn parse_timewait(arg: &str, key: &String, init_param: &mut InitParam) {
    let str1 = &arg[key.len()..];
    if let Ok(value) = str1.parse::<i64>() {
        if value >= DEFAULT_TIMEWAIT {
            init_param.time_wait = value;
        }
    }
}

pub struct InitParam {
    pub time_cnt: i64,
    pub time_wait: i64,
}

pub struct Param {
    pub init_param: InitParam,
    pub manager_param: Vec<String>,
}

impl Param {
    pub fn new() -> Self {
        Param {
            init_param: InitParam {
                time_cnt: DEFAULT_TIMECNT,
                time_wait: DEFAULT_TIMEWAIT,
            },
            manager_param: Vec::<String>::with_capacity(0),
        }
    }

    pub fn get_opt(&mut self, args: Vec<String>) {
        for arg in args {
            for table in PARAM_TABLE {
                if arg.starts_with(table.key) && table.callback.is_some() {
                    if INIT_PARAM == table.param_type {
                        table.callback.unwrap()(&arg, &table.key.to_string(), &mut self.init_param);
                    } else {
                        self.manager_param.push(arg);
                    }
                    break;
                }
            }
        }
    }
}
