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

#[derive(Debug)]
struct Signals {}
impl Signals {
    fn new() -> Signals {
        Signals {}
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
        (libc::EPOLLIN | libc::EPOLLONESHOT) as u32
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn dispatch(&self, _: &mut Events) {
        println!("Dispatching signal!");
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

pub struct Manager {
    mode: Mode,
    action: Action,
    event: Events,
    signal: Rc<RefCell<Signals>>,
}

type JobId = i32;



impl Manager {
    pub fn new(mode: Mode, action: Action) -> Manager {
        Manager {
            mode,
            action,
            event: Events::new().unwrap(),
            signal: Rc::new(RefCell::new(Signals::new())),
        }
    }

    pub fn startup(&mut self) -> Result<(), Error> {
        self.event.add_source(self.signal.clone());
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
        self.event.rloop();
        Ok(Stats::OK)
    }

    pub fn reload(&mut self) -> Result<(), Error> {
        todo!()
    }

    pub fn reboot(&mut self) -> Result<(), Error> {
        todo!()
    }

    pub fn reexec(&mut self) -> Result<(), Error> {
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

    pub fn state(&self) -> Result<(), Error> {
        todo!()
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


