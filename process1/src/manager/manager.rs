#![allow(clippy::module_inception)]
use super::commands::Commands;
use super::data::DataManager;
use super::manager_config::ManagerConfig;
use super::mount_monitor::MountMonitor;
use super::notify::NotifyEvent;
use super::signals::Signals;
use super::unit::UnitManagerX;
use super::MngErrno;
use event::{EventState, Events};
use nix::sys::reboot::{self, RebootMode};
use nix::sys::socket::UnixCredentials;
use std::collections::HashMap;
use std::error::Error as Err;
use std::io::Error;
use std::rc::Rc;
use utils::error::Error as ServiceError;
use utils::{process_util, Result};

/// manager running mode
#[allow(missing_docs)]
pub enum Mode {
    SYSTEM,
    USER,
}

/// manager action mode
#[allow(missing_docs)]
pub enum Action {
    RUN,
    HELP,
    TEST,
}

/// manager running states
#[allow(missing_docs)]
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

/// Encapsulate manager and expose api to the outside
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
    /// ^v^
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

    /// ^v^
    pub fn startup(&self) -> Result<i32> {
        log::debug!("Adding signals source to event loop.");
        let signal_source = Rc::clone(&self.signal);
        self.event.add_source(signal_source.clone())?;
        self.event.set_enabled(signal_source, EventState::On)?;

        log::debug!("Adding mount source to event loop.");
        let mount_source = Rc::clone(&self.mount_monitor);
        self.event.add_source(mount_source.clone())?;
        self.event.set_enabled(mount_source, EventState::On)?;

        log::debug!("Setup notify socket event.");
        let notify = Rc::clone(&self.notify);
        notify.open_socket().map_err(Error::from)?;
        self.event.add_source(notify.clone())?;
        self.event.set_enabled(notify, EventState::On)?;

        Ok(0)
    }

    /// ^v^
    pub fn add_job(&self, job: JobId) -> Result<(), Error> {
        self.data.add_job(job)
    }

    /// ^v^
    pub fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.data.start_unit(name)
    }

    /// ^v^
    pub fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.data.stop_unit(name)
    }

    /// daemon reload , sig hub
    pub fn rloop(&self) -> Result<Stats> {
        self.data.rloop()
    }

    /// daemon reexec, for rerun manager
    pub fn reexec(&self) -> Result<(), Error> {
        todo!()
    }

    /// registe the manager to event
    fn register(&self, event: &Rc<Events>) {
        let source = Rc::clone(&self.commands);
        event.add_source(source).unwrap();
    }

    /// enable event loop
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

    pub(crate) fn add_job(&self, _job: JobId) -> Result<(), Error> {
        Ok(())
    }

    pub(crate) fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.um.start_unit(name)
    }

    pub(crate) fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.um.stop_unit(name)
    }

    pub(crate) fn rloop(&self) -> Result<Stats> {
        loop {
            self.um.dispatch_load_queue();
            self.event.run(-1)?;
        }
        #[allow(unreachable_code)]
        Ok(Stats::OK)
    }

    pub(crate) fn reboot(&self, reboot_mode: RebootMode) -> Result<(), MngErrno> {
        // self.start_unit("shutdown.target");
        let mut pids = process_util::kill_all_pids(15);
        pids = process_util::wait_pids(pids, 10000000);
        if pids.is_empty() {
            return Ok(());
        }
        pids = process_util::kill_all_pids(9);
        process_util::wait_pids(pids, 10000000);
        log::info!("Rebooting...");
        let _ = reboot::reboot(reboot_mode); // make lint happy
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manager_api() {
        let manager = ManagerX::new(Mode::SYSTEM, Action::RUN);
        let _ = manager.startup();
        let ret = manager.add_job(0);
        assert!(ret.is_ok());

        let ret = manager.start_unit("config.service");
        assert!(ret.is_ok());

        let ret = manager.stop_unit("config.service");
        assert!(ret.is_ok());

        let ret = manager.data.dispatch_sigchld();
        assert!(ret.is_err()); // no signal input

        let ret = manager.data.dispatch_mountinfo();
        assert!(ret.is_ok());

        let ucred = UnixCredentials::new();
        let messages = HashMap::new();
        let fds = Vec::new();
        let ret = manager.data.notify_message(&ucred, &messages, &fds);
        assert!(ret.is_ok());
    }
}
