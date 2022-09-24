use super::manager::Manager;

use epoll;
use event::{EventType, Events, Source};
use libmount::mountinfo;
use std::cell::RefCell;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::rc::Rc;
use utils::{Error, Result};

pub struct MountMonitorEntry {
    file: File,
    epfd: i32,
    epevent: epoll::Event,
    events: Vec<epoll::Event>,
}

impl MountMonitorEntry {
    pub fn new(filename: String) -> Self {
        let me_file = match File::open(&filename) {
            Ok(me_file) => me_file,
            Err(why) => {
                panic!("Failed to open {}: {}", &filename, why);
            }
        };

        let me_fd = me_file.as_raw_fd();
        let me_epfd = epoll::create(false).unwrap();
        let me_epevent = epoll::Event::new(
            epoll::Events::EPOLLET | epoll::Events::EPOLLIN,
            me_fd as u64,
        );
        let mut me_events: Vec<epoll::Event> = vec![epoll::Event::new(epoll::Events::empty(), 0)];

        epoll::ctl(
            me_epfd,
            epoll::ControlOptions::EPOLL_CTL_ADD,
            me_fd,
            me_epevent,
        )
        .unwrap();
        // drain out all events.
        while epoll::wait(me_epfd, 0, &mut me_events).unwrap() > 0 {}

        log::debug!("Monitoring {} by epool fd {}", filename, me_epfd);
        MountMonitorEntry {
            file: me_file,
            epfd: me_epfd,
            epevent: me_epevent,
            events: me_events,
        }
    }
}

pub(super) struct MountMonitor {
    manager: Rc<Manager>,
    mount_entry_monitor: RefCell<MountMonitorEntry>,
}

impl MountMonitor {
    pub(super) fn new(mr: &Rc<Manager>) -> MountMonitor {
        let mount_entry_monitor = MountMonitorEntry::new("/proc/self/mountinfo".to_string());
        MountMonitor {
            manager: Rc::clone(mr),
            mount_entry_monitor: RefCell::new(mount_entry_monitor),
        }
    }
    pub(super) fn parser(&self) -> Result<i32, Error> {
        let mut mount_data = String::new();
        File::open("/proc/self/mountinfo")
            .unwrap()
            .read_to_string(&mut mount_data)
            .unwrap();
        let parser = mountinfo::Parser::new(mount_data.as_bytes());
        for mount_result in parser {
            match mount_result {
                Ok(mount) => {
                    log::debug!(
                        "[{}] mounted on [{}]",
                        mount.mount_source.to_str().unwrap(),
                        mount.mount_point.to_str().unwrap()
                    );
                }
                Err(err) => {
                    println!("parse error: {}", err);
                }
            }
        }
        Ok(0)
    }
}

impl Source for MountMonitor {
    fn event_type(&self) -> EventType {
        EventType::Io
    }

    fn signals(&self) -> Vec<libc::c_int> {
        vec![]
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn priority(&self) -> i8 {
        0
    }

    fn dispatch(&self, _e: &Events) -> Result<i32, Error> {
        let epfd = self.mount_entry_monitor.borrow().epfd;
        // drain out all events.
        while epoll::wait(epfd, 0, &mut self.mount_entry_monitor.borrow_mut().events).unwrap() > 0 {
        }
        self.manager.dispatch_mountinfo();
        Ok(0)
    }

    fn token(&self) -> u64 {
        0
    }

    fn fd(&self) -> std::os::unix::prelude::RawFd {
        self.mount_entry_monitor.borrow().epfd
    }

    fn pid(&self) -> libc::pid_t {
        0
    }
}

#[cfg(test)]
mod test {
    use crate::manager::manager_config::ManagerConfig;

    use super::super::manager::{Action, Manager, Mode};
    use event::EventState;
    use event::Events;
    use std::rc::Rc;

    use super::super::mount_monitor::MountMonitor;

    #[test]
    fn test_monitor() {
        let e = Rc::new(Events::new().unwrap());
        const MODE: Mode = Mode::SYSTEM;
        const ACTION: Action = Action::RUN;
        let configm = Rc::new(ManagerConfig::new());
        let manager = Rc::new(Manager::new(MODE, ACTION, &e, &configm));
        let mount_source = Rc::new(MountMonitor::new(&manager));

        e.add_source(mount_source.clone()).unwrap();
        e.set_enabled(mount_source.clone(), EventState::On).unwrap();

        e.run(10).unwrap();
    }
}
