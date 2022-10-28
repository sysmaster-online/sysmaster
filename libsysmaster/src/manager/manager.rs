#![allow(clippy::module_inception)]
use super::commands::Commands;
use super::pre_install::{Install, PresetMode};
use super::rentry::{ReliLastFrame, RELI_HISTORY_MAX_DBS};
use super::signals::{SignalDispatcher, Signals};
use super::unit::UnitManagerX;
use super::MngErrno;
use crate::proto::execute::ExecuterAction;
use crate::reliability::Reliability;
use libevent::{EventState, Events};
use libutils::path_lookup::LookupPaths;
use libutils::process_util::{self};
use libutils::Result;
use nix::sys::reboot::{self, RebootMode};
use nix::sys::signal::Signal;
use std::cell::RefCell;
use std::io::Error;
use std::rc::Rc;

/// maximal size of process's arguments
pub const MANAGER_ARGS_SIZE_MAX: usize = 5; // 6 - 1

struct SignalMgr {
    um: Rc<UnitManagerX>,
}

impl SignalMgr {
    fn new(um: Rc<UnitManagerX>) -> Self {
        SignalMgr { um: Rc::clone(&um) }
    }
    fn reexec(&self) -> Result<i32> {
        Ok(1)
    }
}

impl SignalDispatcher for SignalMgr {
    fn dispatch_signal(&self, signal: &Signal) -> Result<i32> {
        match signal {
            Signal::SIGHUP | Signal::SIGSEGV => self.reexec(),
            Signal::SIGINT => todo!(),
            Signal::SIGQUIT => todo!(),
            Signal::SIGILL => todo!(),
            Signal::SIGTRAP => todo!(),
            Signal::SIGABRT => todo!(),
            Signal::SIGBUS => todo!(),
            Signal::SIGFPE => todo!(),
            Signal::SIGKILL => todo!(),
            Signal::SIGUSR1 => todo!(),
            Signal::SIGUSR2 => todo!(),
            Signal::SIGPIPE => todo!(),
            Signal::SIGALRM => todo!(),
            Signal::SIGTERM => todo!(),
            Signal::SIGSTKFLT => todo!(),
            Signal::SIGCHLD => self.um.child_sigchld_enable(true),
            Signal::SIGCONT => todo!(),
            Signal::SIGSTOP => todo!(),
            Signal::SIGTSTP => todo!(),
            Signal::SIGTTIN => todo!(),
            Signal::SIGTTOU => todo!(),
            Signal::SIGURG => todo!(),
            Signal::SIGXCPU => todo!(),
            Signal::SIGXFSZ => todo!(),
            Signal::SIGVTALRM => todo!(),
            Signal::SIGPROF => todo!(),
            Signal::SIGWINCH => todo!(),
            Signal::SIGIO => todo!(),
            Signal::SIGPWR => todo!(),
            Signal::SIGSYS => todo!(),
            _ => todo!(),
        }
    }
}

struct CommandActionMgr {
    um: Rc<UnitManagerX>,
}

impl CommandActionMgr {
    fn new(um: Rc<UnitManagerX>) -> Self {
        CommandActionMgr { um: Rc::clone(&um) }
    }
}

impl ExecuterAction for CommandActionMgr {
    fn start(&self, service_name: &str) -> Result<(), MngErrno> {
        self.um.start_unit(service_name)
    }

    fn stop(&self, unit_name: &str) -> Result<(), MngErrno> {
        self.um.stop_unit(unit_name)
    }

    fn suspend(&self) -> Result<i32> {
        todo!()
    }

    fn poweroff(&self) -> Result<i32> {
        todo!()
    }

    fn reboot(&self) -> Result<i32> {
        todo!()
    }

    fn halt(&self) -> Result<i32> {
        todo!()
    }

    fn disable(&self, unit_file: &str) -> Result<(), Error> {
        self.um.disable_unit(unit_file)
    }

    fn enable(&self, unit_file: &str) -> Result<(), Error> {
        self.um.enable_unit(unit_file)
    }
}

/// Encapsulate manager and expose api to the outside
pub struct Manager {
    event: Rc<Events>,
    reli: Rc<Reliability>,
    commands: Rc<Commands<CommandActionMgr>>,
    signal: Rc<Signals<SignalMgr>>,
    mode: Mode,
    action: Action,
    state: RefCell<State>,
    um: Rc<UnitManagerX>,
    lookup_path: Rc<LookupPaths>,
}

impl Drop for Manager {
    fn drop(&mut self) {
        log::debug!("ManagerX drop, clear.");
        // repeating protection
        self.reli.clear();
        self.event.clear();
    }
}

impl Manager {
    /// create factory instance
    pub fn new(mode: Mode, action: Action) -> Self {
        let _event = Rc::new(Events::new().unwrap());
        let _reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let mut l_path = LookupPaths::new();
        l_path.init_lookup_paths();
        let lookup_path = Rc::new(l_path);
        let umx = Rc::new(UnitManagerX::new(&_event, &_reli, &lookup_path));
        let _signal = Rc::new(Signals::new(&_reli, SignalMgr::new(Rc::clone(&umx))));
        let _commands = Rc::new(Commands::new(&_reli, CommandActionMgr::new(Rc::clone(&umx))));
        Manager {
            event: Rc::clone(&_event),
            reli: Rc::clone(&_reli),
            commands: _commands,
            signal: _signal,
            mode,
            action,
            state: RefCell::new(State::Init),
            um: umx,
            lookup_path,
        }
    }

    fn add_default_job(&self) -> Result<i32> {
        self.reli.set_last_frame1(ReliLastFrame::ManagerOp as u32);
        // add target "SPECIAL_DEFAULT_TARGET"
        if let Err(e) = self.um.start_unit("basic.target") {
            log::error!("Failed to start basic.target: {:?}", e);
        }
        self.reli.clear_last_frame();
        Ok(0)
    }

    fn rloop(&self) -> Result<State> {
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

    /// start up
    pub fn startup(&self) -> Result<i32> {
        let restore = self.reli.enable();
        log::info!("startup with restore[{}]...", restore);

        // recover
        if restore {
            self.reli.recover();
        }

        // setup external connections
        /* enumerate */
        self.um.enumerate();
        /* register entire external events */
        self.register_ex();
        /* register entry's external events */
        if restore {
            self.um.entry_coldplug();
        }

        // add the first job: default job
        if !restore {
            self.add_default_job()?;
            self.set_restore(true); // mark restore for next startup
        }

        // it's ok now
        self.set_state(State::Ok);
        self.reli.clear_last_frame();

        self.preset_all()?;

        Ok(0)
    }

    /// enter the main loop
    pub fn main_loop(&self) -> Result<bool> {
        loop {
            let state = self.rloop()?;
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
        self.clear_restore();
    }

    fn reload(&self) {
        // clear data
        self.um.entry_clear();

        // recover entry
        self.reli.recover();

        // rebuild external connections
        /* enumerate */
        self.um.enumerate();
        /* register entry's external events */
        self.um.entry_coldplug();

        // it's ok now
        self.set_state(State::Ok);
        self.reli.clear_last_frame();
    }

    fn clear_restore(&self) {
        self.reli.data_clear();
    }

    fn set_restore(&self, enable: bool) {
        match enable {
            true => self.reli.set_enable(true),
            false => {
                self.clear_restore();
                self.reboot(RebootMode::RB_AUTOBOOT);
            }
        }
    }

    fn reexec(&self) -> Result<bool> {
        self.set_state(State::ReExecute);
        self.prepare_reexec()?;
        Ok(true)
    }

    fn prepare_reexec(&self) -> Result<(), Error> {
        // restore external resource, like: fd, ...
        // do nothing now
        Ok(())
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
        self.um.register_ex();

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
#[derive(PartialEq, Eq, Debug)]
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

    pub(super) fn recover(&self) {
        self.reli.recover();
    }

    pub(super) fn entry_coldplug(&self) {
        self.um.entry_coldplug();
    }

    pub(super) fn enable_restore(&self) {
        self.reli.set_enable(true);
    }

    pub(super) fn get_restore(&self) -> bool {
        self.reli.enable()
    }

    pub(super) fn ok(&self) {
        self.set_state(State::Ok);
    }

    pub(super) fn check_finished(&self) -> Result<(), Error> {
        todo!()
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
    use libutils::logger;

    //#[test]
    fn manager_api() {
        logger::init_log_with_console("test_target_unit_load", 4);

        // new
        let manager = Manager::new(Mode::System, Action::Run);
        manager.clear_restore(); // clear all data

        // startup
        let ret = manager.startup();
        assert!(ret.is_ok());
    }
}
