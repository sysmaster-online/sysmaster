use std::error::Error as Err;
use std::io::Error;
use std::rc::Rc;

use super::data::DataManager;
use super::unit::UnitManager;

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

pub struct Manager {
    mode: Mode,
    action: Action,
    stat: Stats,
    um: UnitManager,
    dm: Rc<DataManager>,
}

type JobId = i32;

impl Manager {
    pub fn new(mode: Mode, action: Action) -> Manager {
        let _dm = Rc::new(DataManager::new());
        Manager {
            dm: Rc::clone(&_dm),
            mode,
            action,
            stat: Stats::INIT,
            um: UnitManager::new(Rc::clone(&_dm)),
        }
    }

    pub fn startup(&mut self) -> Result<(), Error> {
        Ok(())
    }

    pub fn get_job(&self, id: JobId) -> Result<(), Error> {
        todo!()
    }

    pub fn get_unit(&self, name: &str) -> Result<(), Error> {
        todo!()
    }

    pub fn load_unit(&self, name: &str) -> Result<(), Error> {
        todo!()
    }

    pub fn add_job(&mut self, job: JobId) -> Result<(), Error> {
        Ok(())
    }

    pub fn clear_jobs(&self) -> Result<(), Error> {
        todo!()
    }

    pub fn rloop(&mut self) -> Result<Stats, Error> {
        Ok(Stats::REEXECUTE)
    }

    pub fn reload(&mut self) -> Result<(), Error> {
        todo!()
    }

    pub fn reboot(&mut self) -> Result<(), Error> {
        todo!()
    }

    pub fn reexec(&mut self) -> Result<(), Error> {
        Ok(())
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
        Ok(Stats::REEXECUTE)
    }

    pub fn dispatch_sigchld(&mut self) -> Result<(), Box<dyn Err>> {
        self.um.dispatch_sigchld()
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

    use super::*;

    #[test]
    fn test_mangerplugin() {}
}
