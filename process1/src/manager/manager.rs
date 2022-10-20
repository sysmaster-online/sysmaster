#![allow(clippy::module_inception)]
use super::commands::Commands;
use super::pre_install::{Install, PresetMode};
use super::rentry::{ReliLastFrame, RELI_HISTORY_MAX_DBS};
use super::signals::Signals;
use super::unit::UnitManagerX;
use super::MngErrno;
use crate::reliability::Reliability;
use event::{EventState, Events};
use nix::sys::reboot::{self, RebootMode};
use nix::sys::signal::Signal;
use std::cell::RefCell;
use std::io::Error;
use std::rc::Rc;
use utils::path_lookup::LookupPaths;
use utils::process_util::{self};
use utils::Result;

/// maximal size of process's arguments
pub const MANAGER_ARGS_SIZE_MAX: usize = 5; // 6 - 1

/// Encapsulate manager and expose api to the outside
pub struct ManagerX {
    event: Rc<Events>,
    reli: Rc<Reliability>,
    data: Rc<Manager>,
    commands: Rc<Commands>,
    signal: Rc<Signals>,
}

impl Drop for ManagerX {
    fn drop(&mut self) {
        log::debug!("ManagerX drop, clear.");
        // repeating protection
        self.reli.clear();
        self.event.clear();
    }
}

impl ManagerX {
    /// create factory instance
    pub fn new(mode: Mode, action: Action) -> ManagerX {
        let _event = Rc::new(Events::new().unwrap());
        let _reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let _data = Rc::new(Manager::new(&_event, &_reli, mode, action));
        ManagerX {
            event: Rc::clone(&_event),
            reli: Rc::clone(&_reli),
            data: Rc::clone(&_data),
            commands: Rc::new(Commands::new(&_reli, &_data)),
            signal: Rc::new(Signals::new(&_reli, &_data)),
        }
    }

    /// start up
    pub fn startup(&self) -> Result<i32> {
        let restore = self.data.get_restore();
        log::info!("startup with restore[{}]...", restore);

        // recover
        if restore {
            self.data.recover();
        }

        // setup external connections
        /* enumerate */
        self.data.enumerate();
        /* register entire external events */
        self.register_ex();
        /* register entry's external events */
        if restore {
            self.data.entry_coldplug();
        }

        // add the first job: default job
        if !restore {
            self.data.add_default_job()?;
            self.set_restore(true); // mark restore for next startup
        }

        // it's ok now
        self.data.ok();
        self.reli.clear_last_frame();

        self.data.preset_all()?;

        Ok(0)
    }

    /// enter the main loop
    pub fn main_loop(&self) -> Result<bool> {
        loop {
            let state = self.data.rloop()?;
            match state {
                State::ReLoad => self.reload(),
                State::ReExecute => return self.reexec(),
                State::Reboot => self.reboot(RebootMode::RB_AUTOBOOT),
                State::PowerOff => self.reboot(RebootMode::RB_POWER_OFF),
                State::Halt => self.reboot(RebootMode::RB_HALT_SYSTEM),
                State::KExec => self.reboot(RebootMode::RB_KEXEC),
                State::Suspend => self.reboot(RebootMode::RB_SW_SUSPEND),
                _ => todo!(),
            };
        }
    }

    /// debug action: clear all data restored
    pub fn debug_clear_restore(&self) {
        self.data.clear_restore();
    }

    fn reload(&self) {
        // clear data
        self.data.entry_clear();

        // recover entry
        self.data.recover();

        // rebuild external connections
        /* enumerate */
        self.data.enumerate();
        /* register entry's external events */
        self.data.entry_coldplug();

        // it's ok now
        self.data.ok();
        self.reli.clear_last_frame();
    }

    fn set_restore(&self, enable: bool) {
        match enable {
            true => self.data.enable_restore(),
            false => {
                self.data.clear_restore();
                self.reboot(RebootMode::RB_AUTOBOOT);
            }
        }
    }

    fn reexec(&self) -> Result<bool> {
        self.data.prepare_reexec()?;
        Ok(true)
    }

    fn reboot(&self, reboot_mode: RebootMode) {
        // self.start_unit("shutdown.target");
        let mut pids = process_util::kill_all_pids(15);
        pids = process_util::wait_pids(pids, 10000000);
        if pids.is_empty() {
            return;
        }
        pids = process_util::kill_all_pids(9);
        process_util::wait_pids(pids, 10000000);
        log::info!("Rebooting...");
        let _ = reboot::reboot(reboot_mode); // make lint happy
    }

    fn register_ex(&self) {
        // data
        self.data.register_ex();

        // cmd
        let cmd = Rc::clone(&self.commands);
        self.event.add_source(cmd).unwrap();
        let cmd = Rc::clone(&self.commands);
        self.event.set_enabled(cmd, EventState::On).unwrap();

        // signal
        let signal = Rc::clone(&self.signal);
        self.event.add_source(signal).unwrap();
        let signal = Rc::clone(&self.signal);
        self.event.set_enabled(signal, EventState::On).unwrap();
    }
}

/// manager running mode
#[allow(missing_docs)]
#[derive(PartialEq, Eq)]
pub enum Mode {
    System,
    User,
}

/// manager action mode
#[allow(missing_docs)]
pub enum Action {
    Run,
    Help,
    Test,
}

/// manager running states
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum State {
    Init,
    Ok,
    Exit,
    ReLoad,
    ReExecute,
    Reboot,
    PowerOff,
    Halt,
    KExec,
    Suspend,
    SwitchRoot,
}

pub(crate) struct Manager {
    // associated objects
    event: Rc<Events>,
    reli: Rc<Reliability>,

    // owned objects
    mode: Mode,
    action: Action,
    state: RefCell<State>,

    um: UnitManagerX,
    lookup_path: Rc<LookupPaths>,
}

impl Drop for Manager {
    fn drop(&mut self) {
        log::debug!("Manager drop, clear.");
        // repeating protection
        self.reli.clear();
        self.event.clear();
    }
}

type JobId = i32;

impl Manager {
    pub(crate) fn get_job(&self, _id: JobId) -> Result<(), Error> {
        todo!()
    }

    pub(crate) fn get_unit(&self, _name: &str) -> Result<(), Error> {
        todo!()
    }

    pub(crate) fn enumerate(&self) {
        self.um.enumerate()
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

    pub(crate) fn reset_failed(&mut self) -> Result<(), Error> {
        todo!()
    }

    pub(crate) fn exit(&self) -> Result<i32> {
        self.set_state(State::Exit);
        Ok(0)
    }

    pub(crate) fn reexec(&self) -> Result<i32> {
        self.set_state(State::ReExecute);
        Ok(0)
    }

    pub(crate) fn reboot(&self) -> Result<i32> {
        self.set_state(State::Reboot);
        Ok(0)
    }

    pub(crate) fn poweroff(&self) -> Result<i32> {
        self.set_state(State::PowerOff);
        Ok(0)
    }

    pub(crate) fn halt(&self) -> Result<i32> {
        self.set_state(State::Halt);
        Ok(0)
    }

    pub(crate) fn kexec(&self) -> Result<i32> {
        self.set_state(State::KExec);
        Ok(0)
    }

    pub(crate) fn suspend(&self) -> Result<i32> {
        self.set_state(State::Suspend);
        Ok(0)
    }

    pub(crate) fn switch_root(&self) -> Result<i32> {
        self.set_state(State::SwitchRoot);
        Ok(0)
    }

    pub(super) fn new(
        eventr: &Rc<Events>,
        relir: &Rc<Reliability>,
        mode: Mode,
        action: Action,
    ) -> Manager {
        let mut l_path = LookupPaths::new();
        l_path.init_lookup_paths();
        let lookup_path = Rc::new(l_path);

        Manager {
            event: Rc::clone(eventr),
            reli: Rc::clone(relir),
            mode,
            action,
            state: RefCell::new(State::Init),
            um: UnitManagerX::new(eventr, relir, &lookup_path),
            lookup_path,
        }
    }

    pub(super) fn register_ex(&self) {
        self.um.register_ex();
    }

    pub(super) fn rloop(&self) -> Result<State> {
        while self.state() == State::Ok {
            // queue
            self.um.dispatch_load_queue();

            // event
            self.reli.set_last_frame1(ReliLastFrame::OtherEvent as u32);
            self.event.run(-1)?;
            self.reli.clear_last_frame();
        }

        Ok(self.state())
    }

    pub(super) fn recover(&self) {
        self.reli.recover();
    }

    pub(super) fn entry_coldplug(&self) {
        self.um.entry_coldplug();
    }

    pub(super) fn add_default_job(&self) -> Result<i32> {
        self.reli.set_last_frame1(ReliLastFrame::ManagerOp as u32);
        // add target "SPECIAL_DEFAULT_TARGET"
        if let Err(e) = self.start_unit("basic.target") {
            log::error!("Failed to start basic.target: {:?}", e);
        }
        self.reli.clear_last_frame();
        Ok(0)
    }

    pub(super) fn clear_restore(&self) {
        self.reli.data_clear();
    }

    pub(super) fn enable_restore(&self) {
        self.reli.set_enable(true);
    }

    pub(super) fn get_restore(&self) -> bool {
        self.reli.enable()
    }

    pub(super) fn prepare_reexec(&self) -> Result<(), Error> {
        // restore external resource, like: fd, ...
        // do nothing now
        Ok(())
    }

    pub(super) fn ok(&self) {
        self.set_state(State::Ok);
    }

    pub(super) fn check_finished(&self) -> Result<(), Error> {
        todo!()
    }

    pub(super) fn dispatch_signal(&self, signal: &Signal) -> Result<i32> {
        match *signal {
            Signal::SIGCHLD => self.um.child_sigchld_enable(true),
            Signal::SIGHUP | Signal::SIGSEGV => self.reexec(),
            Signal::SIGINT => todo!(),

            Signal::SIGKILL => todo!(),
            Signal::SIGUSR1 => todo!(),
            Signal::SIGUSR2 => todo!(),
            _ => todo!(),
        }
    }

    fn entry_clear(&self) {
        self.um.entry_clear();
    }

    fn set_state(&self, state: State) {
        *self.state.borrow_mut() = state;
    }

    fn state(&self) -> State {
        *self.state.borrow()
    }

    pub(crate) fn preset_all(&self) -> Result<(), Error> {
        if self.mode != Mode::System {
            let install = Install::new(PresetMode::Enable, self.lookup_path.clone());
            install.preset_all()?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utils::logger;

    //#[test]
    fn manager_api() {
        logger::init_log_with_console("test_target_unit_load", 4);

        // new
        let manager = ManagerX::new(Mode::System, Action::Run);
        manager.data.clear_restore(); // clear all data

        // startup
        let ret = manager.startup();
        assert!(ret.is_ok());

        // start unit
        let ret = manager.data.start_unit("config.service");
        assert!(ret.is_ok());

        // stop unit
        let ret = manager.data.stop_unit("config.service");
        assert!(ret.is_ok());

        // dispatch signal
        let ret = manager.data.dispatch_signal(&Signal::SIGCHLD);
        assert!(ret.is_ok());
    }
}
