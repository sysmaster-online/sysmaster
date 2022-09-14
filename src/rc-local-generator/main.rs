//! # rc-local-generator
//!
//! 将会在/etc/rc.local 存在并可执行的情况下，将其封装为rc-local.service 服务，并加入到系统启动流程的末尾阶段

pub mod rc_local_generator;
use rc_local_generator::*;
use utils::logger;

fn main() {
    logger::init_log_with_console("rc_local_generator", 4);

    /*判断rc.local是否存在并且可执行*/
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
