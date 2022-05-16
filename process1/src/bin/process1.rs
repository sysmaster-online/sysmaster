use log::info;
use process1::manager::{Action, ManagerX, Mode, Stats};
use std::io::Error;
use utils::logger;

fn main() -> Result<(), Error> {
    logger::init_log_with_console("process1", 4);
    info!("process1 running in system mode.");

    const MODE: Mode = Mode::SYSTEM;
    const ACTION: Action = Action::RUN;
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
