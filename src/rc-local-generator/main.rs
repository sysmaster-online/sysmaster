//! # rc-local-generator
//!
//! rc-local-generator is a generator that checks whether /etc/rc.local exists and is executable,
//! and if it is, pulls the rc-local.service unit into the boot process.
//!  This unit is responsible for running this script during late boot.

mod rc_local_generator;
use rc_local_generator::*;
use utils::logger;

fn main() {
    logger::init_log_with_console("rc_local_generator", 4);

    /*judge whether /etc/rc.local can be executed*/
    let e = check_executable(RC_LOCAL_PATH);
    match e {
        Ok(_) => {
            let f = add_symlink("rc-local.service", "/etc/process1/basic.target");
            match f {
                Ok(()) => {}
                Err(_) => log::debug!("failed to create symlink!"),
            }
        }
        _ => log::debug!("{} no exist", RC_LOCAL_PATH),
    }
}
