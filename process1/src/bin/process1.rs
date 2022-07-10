use std::error::Error;

use log::info;
use process1::manager::{Action, ManagerX, Mode, Stats};
use process1::mount::mount_setup;
use utils::logger;

fn main() -> Result<(), Box<dyn Error>> {
    logger::init_log_with_console("process1", 4);
    info!("process1 running in system mode.");

    const MODE: Mode = Mode::SYSTEM;
    const ACTION: Action = Action::RUN;

    // temporary annotation for repeat mount

    // mount_setup::mount_setup_early().map_err(|e| {
    //     log::error!("failed to mount early mount point, errno: {}", e);
    //     format!("failed to mount early mount point, errno: {}", e)
    // })?;

    // mount_setup::mount_setup().map_err(|e| {
    //     log::error!("failed to mount mount point, errno: {}", e);
    //     format!("failed to mount mount point, errno: {}", e)
    // })?;

    // initialize_runtime()?;

    let manager = ManagerX::new(MODE, ACTION);
    manager.startup().unwrap();
    manager.add_job(0).unwrap();

    match manager.rloop() {
        Ok(Stats::REEXECUTE) => manager.reexec()?,
        Ok(_) => todo!(),
        Err(_) => todo!(),
    };

    Ok(())
}

fn initialize_runtime() -> Result<(), Box<dyn Error>> {
    mount_setup::mount_cgroup_controllers()?;

    Ok(())
}
