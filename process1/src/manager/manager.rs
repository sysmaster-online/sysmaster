use super::commands::Commands;
use super::data::DataManager;
use super::manager_config::ManagerConfig;
use super::mount_monitor::MountMonitor;
use super::notify::NotifyEvent;
use super::signals::Signals;
use super::unit::UnitManagerX;
use super::MngErrno;
use event::{EventState, Events};
use nix::sys::socket::UnixCredentials;
use std::collections::HashMap;
use std::error::Error as Err;
use std::io::Error;
use std::rc::Rc;
use utils::error::Error as ServiceError;
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
    event: Rc<Events>,
    commands: Rc<Commands>,
    data: Rc<Manager>,
    signal: Rc<Signals>,
    mount_monitor: Rc<MountMonitor>,
    config: Rc<ManagerConfig>,
    notify: Rc<NotifyEvent>,
}

impl ManagerX {
    pub fn new(mode: Mode, action: Action) -> ManagerX {
        let configm = Rc::new(ManagerConfig::new());
        let _event = Rc::new(Events::new().unwrap());
        let _data = Rc::new(Manager::new(mode, action, &_event, &configm));

        let m = ManagerX {
            event: Rc::clone(&_event),
            commands: Rc::new(Commands::new(&_data)),
            data: Rc::clone(&_data),
            signal: Rc::new(Signals::new(&_data)),
            mount_monitor: Rc::new(MountMonitor::new(&_data)),
            config: configm.clone(),
            notify: Rc::new(NotifyEvent::new(&_data, &configm)),
        };
        m.register(&_event);
        m.enable(&_event);
        m
    }

    pub fn startup(&self) -> Result<i32> {
        log::debug!("Adding signals source to event loop.");
        let signal_source = Rc::clone(&self.signal);
        self.event.add_source(signal_source.clone())?;
        self.event
            .set_enabled(signal_source.clone(), EventState::On)?;

        log::debug!("Adding mount source to event loop.");
        let mount_source = Rc::clone(&self.mount_monitor);
        self.event.add_source(mount_source.clone())?;
        self.event
            .set_enabled(mount_source.clone(), EventState::On)?;

        log::debug!("Setup notify socket event.");
        let notify = Rc::clone(&self.notify);
        notify.open_socket().map_err(|e| Error::from(e))?;
        self.event.add_source(notify.clone())?;
        self.event.set_enabled(notify.clone(), EventState::On)?;

        Ok(0)
    }

    pub fn add_job(&self, job: JobId) -> Result<(), Error> {
        self.data.add_job(job)
    }

    pub fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.data.start_unit(name)
    }

    pub fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.data.stop_unit(name)
    }

    pub fn rloop(&self) -> Result<Stats> {
        self.data.rloop()
    }

    pub fn reexec(&self) -> Result<(), Error> {
        self.data.reexec()
    }

    fn register(&self, event: &Rc<Events>) {
        let source = Rc::clone(&self.commands);
        event.add_source(source).unwrap();
    }

    fn enable(&self, event: &Rc<Events>) {
        let source = Rc::clone(&self.commands);
        event.set_enabled(source, EventState::On).unwrap();
    }
}

pub(crate) struct Manager {
    mode: Mode,
    action: Action,
    stat: Stats,

    dm: Rc<DataManager>,
    um: UnitManagerX,
    event: Rc<Events>,
}

type JobId = i32;

impl Manager {
    pub(crate) fn new(
        mode: Mode,
        action: Action,
        eventr: &Rc<Events>,
        configm: &Rc<ManagerConfig>,
    ) -> Manager {
        let _dm = Rc::new(DataManager::new());
        Manager {
            mode,
            action,
            stat: Stats::INIT,
            dm: Rc::clone(&_dm),
            um: UnitManagerX::new(&_dm, eventr, configm),
            event: Rc::clone(eventr),
        }
    }

    pub(crate) fn get_job(&self, _id: JobId) -> Result<(), Error> {
        todo!()
    }

    pub(crate) fn get_unit(&self, _name: &str) -> Result<(), Error> {
        todo!()
    }

    pub(crate) fn add_job(&self, _job: JobId) -> Result<(), Error> {
        Ok(())
    }

    pub(crate) fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.um.start_unit(name)
    }

    pub(crate) fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.um.stop_unit(name)
    }

    pub(crate) fn clear_jobs(&self) -> Result<(), Error> {
        todo!()
    }

    pub(crate) fn rloop(&self) -> Result<Stats> {
        loop {
            self.um.dispatch_load_queue();
            self.event.run(-1)?;
        }
        #[allow(unreachable_code)]
        Ok(Stats::OK)
    }

    pub(crate) fn reload(&mut self) -> Result<(), Error> {
        todo!()
    }

    pub(crate) fn reboot(&mut self) -> Result<(), Error> {
        todo!()
    }

    pub(crate) fn reexec(&self) -> Result<(), Error> {
        todo!()
    }

    pub(crate) fn switch_root(&mut self) -> Result<(), Error> {
        todo!()
    }

    pub(crate) fn check_finished(&self) -> Result<(), Error> {
        todo!()
    }

    pub(crate) fn reset_failed(&mut self) -> Result<(), Error> {
        todo!()
    }

    pub(crate) fn exit(&mut self) {
        self.stat = Stats::EXIT;
    }

    pub(crate) fn state(&self) -> Result<Stats, Error> {
        todo!()
    }

    pub(crate) fn dispatch_sigchld(&self) -> Result<(), Box<dyn Err>> {
        self.um.child_dispatch_sigchld()
    }

    pub(crate) fn dispatch_mountinfo(&self) -> Result<(), MngErrno> {
        self.um.dispatch_mountinfo()
    }

    pub(crate) fn notify_message(
        &self,
        ucred: &UnixCredentials,
        messages: &HashMap<&str, &str>,
        fds: &Vec<i32>,
    ) -> Result<(), ServiceError> {
        self.um.notify_message(ucred, messages, fds)
    }
}

impl Drop for Manager {
    fn drop(&mut self) {}
}

#[cfg(test)]
mod tests {
    // use crate::manager::service::ServiceUnit;

    // use super::*;

    #[test]
    fn test_mangerplugin() {}
}
