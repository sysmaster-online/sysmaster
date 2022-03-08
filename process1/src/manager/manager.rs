use std::cell::RefCell;
use std::io::Error;
use std::rc::Rc;

use event::Events;
use event::EventType;
use event::Source;

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

pub struct Signals {
    manager: Rc<RefCell<Manager>>,
}
impl Signals {
    pub fn new(m: Rc<RefCell<Manager>>) -> Signals {
        Signals { manager: m.clone() }
    }
}

impl Source for Signals {
    fn event_type(&self) -> EventType {
        EventType::Signal
    }

    fn signals(&self) -> Vec<libc::c_int> {
        vec![libc::SIGCHLD, libc::SIGTERM, libc::SIGINT]
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn dispatch(&self, e: &mut Events) {
        println!("Dispatching signal!");
        loop {
            match e.read_signals() {
                Ok(Some(info)) => {
                    println!("read signo: {:?}", info.si_signo);
                    break;
                }
                Ok(None) => break,
                Err(e) => {
                    println!("{:?}", e);
                    break;
                }
            }
        }
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn fd(&self) -> std::os::unix::prelude::RawFd {
        todo!()
    }

    fn pid(&self) -> libc::pid_t {
        0
    }
}

pub struct Manager {
    mode: Mode,
    action: Action,
    stat: Stats,
}

type JobId = i32;



impl Manager {
    pub fn new(mode: Mode, action: Action) -> Manager {
        Manager {
            mode,
            action,
            stat: Stats::INIT,
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

    pub fn rloop(&mut self) -> Result<Stats, Error>  {
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
        self.stat =  Stats::EXIT;
    }

    pub fn state(&self) -> Result<Stats, Error> {
        Ok(Stats::REEXECUTE)
    }
}

impl Drop for Manager {
    fn drop(&mut self) {}
}

pub trait  Mangerobj {
    fn init(&self){

    }

    fn load(&self);

    fn dispatch(&self) -> i32;

    fn reload(&self) -> Option<i32>;

    fn destroy(&self);

    // reserved for sd event
    fn event_dispatch(&self) -> Option<i32>;
}

pub struct MangerLoader  {
    pub managers: Vec<Box <dyn Mangerobj>>,
}


impl  MangerLoader{
    pub fn new() -> Self{
        MangerLoader{
            managers: Vec::new()
        }
    }
    pub fn load_plugins(&mut self, d: Box<dyn Mangerobj>) {
            self.managers.push(d);
        }

        pub fn run(&mut self) -> i32{
            let mut ret:i32 = 0;
            for m in self.managers.iter(){
                m.init();
                m.load();
                ret =m.dispatch();
            }
            ret
        }

        pub fn destroy(&self) {
            for m in self.managers.iter(){
                m.destroy();
            }
        }

        pub fn reload(&self){
            for m in self.managers.iter(){
                m.reload();
            }
        }
}
#[cfg(test)]
mod tests {
    use crate::manager::service::ServiceUnit;

    use super::*;

#[test]
fn  test_mangerplugin(){

}
}