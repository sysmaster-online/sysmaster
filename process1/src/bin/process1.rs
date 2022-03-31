use event::{Events, Source};
use log::info;
use process1::manager::commands::Commands;
use process1::manager::manager::{Action, Manager, Mode, Stats};
use process1::manager::signals::Signals;
use std::{cell::RefCell, io::Error, rc::Rc};
use utils::logger;

fn main() -> Result<(), Error> {
    logger::init_log_with_console("process1", 4);
    info!("process1 running in system mode.");

    const MODE: Mode = Mode::SYSTEM;
    const ACTION: Action = Action::RUN;
    let manager = Rc::new(RefCell::new(Manager::new(MODE, ACTION)));
    let mut m = manager.try_borrow_mut().unwrap();

    m.startup().unwrap();

    m.add_job(0).unwrap();

    let mut event = Events::new().unwrap();
    let signal: Rc<RefCell<dyn Source>> = Rc::new(RefCell::new(Signals::new(manager.clone())));
    let command: Rc<RefCell<dyn Source>> = Rc::new(RefCell::new(Commands::new(manager.clone())));

    event.add_source(signal.clone());
    event.add_source(command.clone());

    loop {
        event.run(-1);
        match m.state() {
            Ok(Stats::REEXECUTE) => m.reexec()?,
            Ok(_) => todo!(),
            Err(_) => todo!(),
        };
    }

    #[allow(unreachable_code)]
    Ok(())
}
