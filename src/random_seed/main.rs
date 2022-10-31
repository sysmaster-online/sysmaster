//! # random-seed
//!
//! random-seed.service, random_seed Load and save the system random seed at boot and shutdown

use std::{env, process};
mod random_seed;
use crate::random_seed::run;
use libutils::logger;

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
