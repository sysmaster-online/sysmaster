//! test unit signal
use process1::manager::{Action, ManagerX, Mode};
use std::env;

use utils::logger;

fn main() {
    logger::init_log_with_console("test_unit_signal", 4);
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

    const MODE: Mode = Mode::SYSTEM;
    const ACTION: Action = Action::RUN;
    let manager = ManagerX::new(MODE, ACTION);

    let unit_name = String::from("config.service");

    {
        match manager.start_unit(&unit_name) {
            Ok(_) => {}
            Err(e) => {
                println!("start error: {:?}", e)
            }
        }
    }
    log::debug!("event running");
}
