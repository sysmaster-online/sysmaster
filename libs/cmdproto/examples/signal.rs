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

//! test unit signal

fn main() {
    /*log::init_log_to_console("test_unit_signal", 4);
    let out_dir = env::var("LD_LIBRARY_PATH");
    let _tmp_str = out_dir.unwrap();
    let _tmp_str_v = _tmp_str.split(':').collect::<Vec<_>>()[0];
    let _tmp_path = _tmp_str_v.split("target").collect::<Vec<_>>()[0];
    let mut r_s: String = String::new();
    r_s.push_str(_tmp_path);
    r_s.push_str("target/debug;");
    r_s.push_str(_tmp_path);
    r_s.push_str("target/release;");
    env::set_var("PROCESS_LIB_LOAD_PATH", r_s.as_str());

    const MODE: Mode = Mode::System;
    const ACTION: Action = Action::Run;
    let manager = Manager::new(MODE, ACTION);

    manager.startup().unwrap();

    log::debug!("event running");*/
}
