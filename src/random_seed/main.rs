//! # random-seed
//!
//! 在系统启动的早期加载随机数种子，并在关机过程中保存系统的随机数种子，可以增加在系统启动早期的可用熵数量。随机数种子保存在/etc/process1/random-seed 文件中。

use std::{env, process};
mod random_seed;
use crate::random_seed::run;
use utils::logger;

fn main() {
    logger::init_log_with_console("random-seed", 4);
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        log::error!("{}", "This program requires one argument.");
        process::exit(1);
    }

    unsafe {
        libc::umask(0o022);
    }

    if let Err(str) = run(&args[1]) {
        log::error!("{str}");
        process::exit(1);
    }

    process::exit(0);
}
