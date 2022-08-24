pub mod rc_local_generator;
use rc_local_generator::*;
use utils::logger;

fn main() {
    logger::init_log_with_console("rc_local_generator", 4);
    /*解析命令行参数 命令个数为1或者是4*/
    let args: Vec<String> = std::env::args().collect();

    let args_size = args.len();
    let mut str_to = String::new();

    if 1 == args_size {
        str_to.push_str("/tmp");
    } else if 1 < args_size && 4 == args_size {
        str_to.push_str(&args[1]);
    } else {
        log::debug!("This program takes zero or three arguments.");
        return;
    }

    /*判断rc.local是否存在并且可执行*/
    let e = check_executable(RC_LOCAL_PATH);
    match e {
        Ok(_) => {
            str_to = str_to + "/" + "multi-user.target";

            let f = add_symlink("rc-local.service", &str_to);
            match f {
                Ok(()) => {}
                Err(_) => log::debug!("failed to create symlink!"),
            }
        }
        _ => log::debug!("{} no exist", RC_LOCAL_PATH),
    }
}
