//! # rc-local-generator
//!
//! When /etc/rc.local exists and is executable, it will be encapsulated as rc-local.service service
//! and added to the end of the system startup process

mod rc_local_generator;
use libutils::logger;
use rc_local_generator::*;

fn main() {
    logger::init_log_with_console("rc_local_generator", 4);

    // Determine if rc.local exists and is executable
    let e = check_executable(RC_LOCAL_PATH);
    match e {
        Ok(_) => {
            let f = add_symlink("rc-local.service", "/etc/sysmaster/basic.target");
            match f {
                Ok(()) => {}
                Err(_) => log::debug!("failed to create symlink!"),
            }
        }
        _ => log::debug!("{} no exist", RC_LOCAL_PATH),
    }
}
