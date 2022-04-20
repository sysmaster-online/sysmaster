use super::commands::Commands;
use super::data::DataManager;
use super::signals::Signals;
use super::unit::UnitManagerX;
use super::MngErrno;
use event::Events;
use std::cell::RefCell;
use std::error::Error as Err;
use std::io::Error;
use std::rc::Rc;
use utils::Result;

pub enum Mode {
    SYSTEM,
    USER,
}

pub enum Action {
    RUN,
    HELP,
    TEST,
}

pub enum Stats {
    INIT,
    OK,
    EXIT,
    RELOAD,
    REEXECUTE,
    REBOOT,
    POWEROFF,
    HALT,
    KEXEC,
    SWITCHROOT,
}

pub struct ManagerX {
    event: Rc<RefCell<Events>>,
    commands: Rc<RefCell<Commands>>,
    data: Rc<Manager>,
}

impl ManagerX {
    pub fn new(mode: Mode, action: Action) -> ManagerX {
        let _event = Rc::new(RefCell::new(Events::new().unwrap()));
        let _data = Rc::new(Manager::new(mode, action, Rc::clone(&_event)));
        let m = ManagerX {
            event: Rc::clone(&_event),
            commands: Rc::new(RefCell::new(Commands::new(Rc::clone(&_data)))),
            data: Rc::clone(&_data),
        };
        m.register(Rc::clone(&m.event));
        m
    }

    pub fn startup(&self) -> Result<i32> {
        self.data.startup()
    }

    pub fn add_job(&self, job: JobId) -> Result<(), Error> {
        self.data.add_job(job)
    }

    pub fn rloop(&self) -> Result<Stats> {
        self.data.rloop()
    }

    pub fn reexec(&self) -> Result<(), Error> {
        self.data.reexec()
    }

    fn register(&self, event: Rc<RefCell<Events>>) {
        let source = Rc::clone(&self.commands);
        event.borrow_mut().add_source(source).unwrap();
    }
}

pub struct Manager {
    mode: Mode,
    action: Action,
    stat: Stats,
    dm: Rc<DataManager>,
    um: Rc<UnitManagerX>,
    event: Rc<RefCell<Events>>,
    signal: Rc<RefCell<Signals>>,
}

type JobId = i32;

impl Manager {
    pub fn new(mode: Mode, action: Action, event: Rc<RefCell<Events>>) -> Manager {
        let _dm = Rc::new(DataManager::new());
        let _um = Rc::new(UnitManagerX::new(Rc::clone(&_dm), Rc::clone(&event)));
        Manager {
            mode,
            action,
            stat: Stats::INIT,
            dm: Rc::clone(&_dm),
            um: Rc::clone(&_um),
            event,
            signal: Rc::new(RefCell::new(Signals::new(Rc::clone(&_um)))),
        }
    }

    pub fn startup(&self) -> Result<i32> {
        let source = Rc::clone(&self.signal);
        self.event.borrow_mut().add_source(source)?;
        Ok(0)
    }

    pub fn get_job(&self, _id: JobId) -> Result<(), Error> {
        todo!()
    }

    pub fn get_unit(&self, _name: &str) -> Result<(), Error> {
        todo!()
    }

    pub fn load_unit(&self, _name: &str) -> Result<(), Error> {
        todo!()
    }

    pub fn add_job(&self, _job: JobId) -> Result<(), Error> {
        Ok(())
    }

    pub fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.um.start_unit(name)
    }

    pub fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.um.stop_unit(name)
    }

    pub fn clear_jobs(&self) -> Result<(), Error> {
        todo!()
    }

    pub fn rloop(&self) -> Result<Stats> {
        loop {
            self.um.dispatch_load_queue();
            self.event.borrow_mut().run(-1)?;
        }
        #[allow(unreachable_code)]
        Ok(Stats::OK)
    }

    pub fn reload(&mut self) -> Result<(), Error> {
        todo!()
    }

    pub fn reboot(&mut self) -> Result<(), Error> {
        todo!()
    }

    pub fn reexec(&self) -> Result<(), Error> {
        todo!()
    }

    pub fn switch_root(&mut self) -> Result<(), Error> {
        todo!()
    }

    pub fn check_finished(&self) -> Result<(), Error> {
        todo!()
    }

    pub fn reset_failed(&mut self) -> Result<(), Error> {
        todo!()
    }

    pub fn exit(&mut self) {
        self.stat = Stats::EXIT;
    }

    pub fn state(&self) -> Result<Stats, Error> {
        todo!()
    }

    pub fn dispatch_sigchld(&mut self) -> Result<(), Box<dyn Err>> {
        self.um.child_dispatch_sigchld()
    }
}

impl Drop for Manager {
    fn drop(&mut self) {}
}

pub trait Mangerobj {
    fn init(&self) {}

    fn load(&self);

    fn dispatch(&self) -> i32;

    fn reload(&self) -> Option<i32>;

    fn destroy(&self);

    // reserved for sd event
    fn event_dispatch(&self) -> Option<i32>;
}

pub struct MangerLoader {
    pub managers: Vec<Box<dyn Mangerobj>>,
}

impl MangerLoader {
    pub fn new() -> Self {
        MangerLoader {
            managers: Vec::new(),
        }
    }
    pub fn load_plugins(&mut self, d: Box<dyn Mangerobj>) {
        self.managers.push(d);
    }

    pub fn run(&mut self) -> i32 {
        let mut ret: i32 = 0;
        for m in self.managers.iter() {
            m.init();
            m.load();
            ret = m.dispatch();
        }
        ret
    }

    pub fn destroy(&self) {
        for m in self.managers.iter() {
            m.destroy();
        }
    }

    pub fn reload(&self) {
        for m in self.managers.iter() {
            m.reload();
        }
    }
}
#[cfg(test)]
mod tests {
    // use crate::manager::service::ServiceUnit;

    // use super::*;

    #[test]
    fn test_mangerplugin() {}
}
