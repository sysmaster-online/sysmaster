use event::{Events, Source};
use log::info;
use core::manager::manager::{Mode, Action, Manager, Stats, Signals};
use std::{io::Error, cell::RefCell, rc::Rc};
use utils::logger;

fn main() -> Result<(), Error>{
    logger::init_log_with_console("process1", 4);
    info!("process1 running in system mode.");

    const MODE: Mode = Mode::SYSTEM;
    const ACTION: Action = Action::RUN;
    let manager = Rc::new(RefCell::new(Manager::new(MODE, ACTION)));
    let mut m = manager.try_borrow_mut().unwrap();

    m.startup().unwrap();

    m.add_job(0).unwrap();

    let mut event = Events::new().unwrap();
    let source: Rc<RefCell<dyn Source>> = Rc::new(RefCell::new(Signals::new(manager.clone())));

    event.add_source(source.clone());

    loop {
        event.run(0);
        match m.state() {
            Ok(Stats::REEXECUTE) => m.reexec(),
            Ok(_) => todo!(),
            Err(_) => todo!(),
        };
    };

    info!("process1 shutdown.");
    Ok(())
}