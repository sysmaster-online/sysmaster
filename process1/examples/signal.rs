use std::rc::Rc;

use event::EventState;
use event::Events;
use process1::manager::{signals::Signals, DataManager, UnitManagerX};

use utils::logger;

fn main() {
    logger::init_log_with_console("test_unit_signal", 4);
    let event1 = Rc::new(Events::new().unwrap());

    let _dm = Rc::new(DataManager::new());
    let um = Rc::new(UnitManagerX::new(Rc::clone(&_dm), event1.clone()));
    let signal = Rc::new(Signals::new(um.clone()));

    event1.add_source(signal.clone()).unwrap();
    event1.set_enabled(signal.clone(), EventState::On).unwrap();
    let unit_name = String::from("config.service");

    {
        um.start_unit(&unit_name).unwrap();
    }
    log::debug!("event runing");
    println!("event: {:?}", event1);
    event1.run(0).unwrap();
}
