// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use constants::LOG_FILE_PATH;
use core::unit::UnitStatus;
use nix::sys::signalfd::siginfo;
#[cfg(test)]
pub(crate) use rentry::RELI_HISTORY_MAX_DBS;
use std::vec;
pub(crate) mod alive_timer;
pub(crate) mod commands;
pub(crate) mod config;
pub(crate) mod pre_install;
pub(crate) mod rentry;
pub(crate) mod signals;
use self::config::ManagerConfig;
use crate::unit::UnitManagerX;
use alive_timer::AliveTimer;
use basic::fs::LookupPaths;
use basic::process::{self};
use basic::{CGROUP_SYSMASTER, MULTI_USER_TARGET};
use cgroup::CgController;
use cgroup::{cg_create_and_attach, CgFlags};
use cmdproto::proto::execute::ExecuterAction;
use cmdproto::proto::transient_unit_comm::UnitConfig;
use commands::Commands;
use core::error::*;
use core::rel::{ReliConf, ReliLastFrame, Reliability};
use event::{EventState, Events};
use log::{self, Level};
use nix::sys::reboot::{self, RebootMode};
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use pre_install::{Install, PresetMode};
use signals::{SignalDispatcher, Signals};
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;

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
        self.um.set_state(State::ReExecute);
        Ok(1)
    }

    fn reload(&self) -> Result<i32> {
        self.um.set_state(State::ReLoad);
        Ok(1)
    }
}

impl SignalDispatcher for SignalMgr {
    fn dispatch_signal(&self, signal: &siginfo) -> Result<i32> {
        /* Received signal should be in the set defined in EVENT_SIGNALS */
        match signal.ssi_signo as libc::c_int {
            libc::SIGHUP => self.reload(),
            libc::SIGTERM => self.reexec(),
            libc::SIGCHLD => Ok(self.um.child_sigchld_enable(true)),
            /* Kernel will send SIGINT to PID1 when users press ctrl-alt-del,
             * init should forward SIGINT to sysmaster. */
            libc::SIGINT => self
                .um
                .start_unit("ctrl-alt-del.target", false, "replace")
                .map(|_| 1),
            _ => Ok(0),
        }
    }
}

struct CommandActionMgr {
    um: Rc<UnitManagerX>,
    state: Rc<RefCell<State>>,
}

impl CommandActionMgr {
    fn new(um: Rc<UnitManagerX>, state: Rc<RefCell<State>>) -> Self {
        CommandActionMgr { um, state }
    }

    fn set_state(&self, state: State) {
        if *self.state.borrow_mut() != state {
            *self.state.borrow_mut() = state;
        }
    }
}

impl ExecuterAction for CommandActionMgr {
    type Error = core::error::Error;
    type Status = core::unit::UnitStatus;
    // type Result<T, Error> = Result<T, E>;
    fn start(&self, unit_name: &str) -> Result<(), Self::Error> {
        self.um.start_unit(unit_name, true, "replace")
    }

    fn stop(&self, unit_name: &str) -> Result<(), Self::Error> {
        self.um.stop_unit(unit_name, true)
    }

    fn restart(&self, unit_name: &str) -> Result<(), Self::Error> {
        self.um.restart_unit(unit_name, true)
    }

    fn reload(&self, unit_name: &str) -> Result<(), Self::Error> {
        self.um.reload(unit_name)
    }

    fn isolate(&self, unit_name: &str) -> Result<(), Self::Error> {
        self.um.start_unit(unit_name, true, "isolate")
    }

    fn reset_failed(&self, unit_name: &str) -> cmdproto::error::Result<(), Self::Error> {
        self.um.reset_failed(unit_name)
    }

    fn status(&self, unit_name: &str) -> Result<UnitStatus, Self::Error> {
        self.um.get_unit_status(unit_name)
    }

    fn list_units(&self) -> Result<String, Self::Error> {
        self.um.get_all_units()
    }

    fn suspend(&self) -> Result<i32, Self::Error> {
        self.set_state(State::Suspend);
        Ok(0)
    }

    fn poweroff(&self) -> Result<i32, Self::Error> {
        self.set_state(State::PowerOff);
        Ok(0)
    }

    fn reboot(&self) -> Result<i32, Self::Error> {
        self.set_state(State::Reboot);
        Ok(0)
    }

    fn halt(&self) -> Result<i32, Self::Error> {
        self.set_state(State::Halt);
        Ok(0)
    }

    fn disable(&self, unit_file: &str) -> Result<(), Self::Error> {
        self.um
            .disable_unit(unit_file)
            .map(|_| self.daemon_reload())
    }

    fn enable(&self, unit_file: &str) -> Result<(), Self::Error> {
        self.um.enable_unit(unit_file).map(|_| self.daemon_reload())
    }

    fn mask(&self, unit_file: &str) -> Result<(), Self::Error> {
        self.um.mask_unit(unit_file).map(|_| self.daemon_reload())
    }

    fn unmask(&self, unit_file: &str) -> Result<(), Self::Error> {
        self.um.unmask_unit(unit_file).map(|_| self.daemon_reload())
    }

    fn daemon_reload(&self) {
        self.set_state(State::ReLoad);
    }

    fn daemon_reexec(&self) {
        self.set_state(State::ReExecute);
    }

    fn switch_root(&self, init: &[String]) -> Result<(), Self::Error> {
        self.um.switch_root(init)
    }

    fn start_transient_unit(
        &self,
        job_mode: &str,
        unit_config: &UnitConfig,
        aux_units: &[UnitConfig],
    ) -> Result<(), Self::Error> {
        self.um
            .start_transient_unit(job_mode, unit_config, aux_units)
    }
}

/// Encapsulate manager and expose api to the outside
pub struct Manager {
    event: Rc<Events>,
    reli: Rc<Reliability>,
    commands: Rc<Commands<CommandActionMgr>>,
    signal: Rc<Signals<SignalMgr>>,
    mode: Mode,
    _action: Action,
    state: Rc<RefCell<State>>,
    um: Rc<UnitManagerX>,
    lookup_path: Rc<LookupPaths>,
    alive_timer: Rc<AliveTimer>,
    #[allow(dead_code)]
    config: Rc<RefCell<ManagerConfig>>,
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
    pub fn new(mode: Mode, action: Action, manager_config: Rc<RefCell<ManagerConfig>>) -> Self {
        let event = Rc::new(Events::new().unwrap());
        let reli = Rc::new(Reliability::new(
            ReliConf::new()
                .set_map_size(manager_config.borrow().DbSize)
                .set_max_dbs(rentry::RELI_HISTORY_MAX_DBS),
        ));
        let mut l_path = LookupPaths::new();
        l_path.init_lookup_paths();
        let lookup_path = Rc::new(l_path);
        let state = Rc::new(RefCell::new(State::Init));
        let um = Rc::new(UnitManagerX::new(
            &event,
            &reli,
            &lookup_path,
            Rc::clone(&state),
            manager_config.clone(),
        ));

        let alive_timer = Rc::new(AliveTimer::new(&event));

        Manager {
            event,
            commands: Rc::new(Commands::new(
                &reli,
                CommandActionMgr::new(Rc::clone(&um), Rc::clone(&state)),
            )),
            signal: Rc::new(Signals::new(&reli, SignalMgr::new(Rc::clone(&um)))),
            reli,
            mode,
            _action: action,
            state,
            um,
            lookup_path,
            alive_timer,
            config: manager_config,
        }
    }

    fn add_default_job(&self) -> Result<i32> {
        self.reli.set_last_frame1(ReliLastFrame::ManagerOp as u32);
        // add target "SPECIAL_DEFAULT_TARGET"
        if let Err(e) = self.um.start_unit(MULTI_USER_TARGET, false, "replace") {
            log::error!("Failed to start multi-user.target: {:?}", e);
        }
        self.reli.clear_last_frame();
        Ok(0)
    }

    fn rloop(&self) -> Result<State> {
        while self.state() == State::Ok {
            // queue
            self.um.dispatch_load_queue();
            self.um.dispatch_stop_when_bound_queue();

            // event
            self.reli.set_last_frame1(ReliLastFrame::OtherEvent as u32);

            let ret = self.event.run(-1);
            if ret.is_err() {
                log::error!("event run loop error is: {:?}", ret);
            }
            self.reli.clear_last_frame();
        }

        Ok(self.state())
    }

    /// start up
    pub fn startup(&self, reload: bool) -> Result<i32> {
        self.reli.debug_clear();

        let restore = self.reli.enable();
        log::info!("startup with restore[{}] reload[{}]", restore, reload);

        // recover
        if reload {
            // sctl daemon-reexec needs to reload the configuration.
            self.reli.recover(true);
        } else if restore {
            // Fault recovery or process 1 execution does not require reloading the configuration.
            self.reli.recover(false);
        } else {
            // Do nothing during the initialization phase or when interrupted by process one before restore is true.
        }

        self.run_generators();

        // preset file before add default job
        self.preset_all().unwrap();

        // setup external connections
        /* register entire external events */
        self.register_ex();
        /* register entry's external events */
        if reload || restore {
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
                State::SwitchRoot => return Ok(false),
                _ => todo!(),
            };
        }
    }

    /// debug action: clear all data restored
    pub fn debug_clear_restore(&self) {
        self.reli.data_clear();
    }

    /// create cgroup and attach self to it
    pub fn setup_cgroup(&self) -> Result<()> {
        let cg_init = PathBuf::from(CGROUP_SYSMASTER);

        if let Err(e) = cg_create_and_attach(&cg_init, Pid::from_raw(0)) {
            return Err(Error::Cgroup { source: e });
        }

        log::debug!("kill all pids except sysmaster in the sysmaster cgroup");
        cgroup::cg_kill_recursive(
            &cg_init,
            Signal::SIGKILL,
            CgFlags::IGNORE_SELF,
            HashSet::new(),
        )
        .map_err(|e| Error::Other {
            msg: format!("failed to kill cgroup: {:?}, error: {}", cg_init, e),
        })?;

        Ok(())
    }

    fn reload(&self) {
        self.config.borrow_mut().reload(&self.mode);
        log::init_log(
            "sysmaster",
            Level::from_str(&self.config.borrow().LogLevel).unwrap(),
            self.config
                .borrow()
                .LogTarget
                .split(&[' ', '-'][..])
                .collect(),
            LOG_FILE_PATH,
            self.config.borrow().LogFileSize,
            self.config.borrow().LogFileNumber,
            false,
        );

        // flush db
        if let Err(e) = self.reli.flush() {
            log::error!("flush failed with error:{:?}, try next reload.", e);
        } else {
            log::info!("reload start.");

            self.run_generators();

            // clear data
            self.um.entry_clear();

            // recover entry
            self.reli.recover(true);

            // rebuild external connections
            /* register entry's external events */
            self.um.entry_coldplug();
        }

        // it's ok now
        self.set_state(State::Ok);
        self.reli.clear_last_frame();
    }

    fn set_restore(&self, enable: bool) {
        match enable {
            true => self.reli.set_enable(true),
            false => {
                self.reli.data_clear();
                self.reboot(RebootMode::RB_AUTOBOOT);
            }
        }
    }

    fn reexec(&self) -> Result<bool> {
        self.set_state(State::ReExecute);
        self.prepare_reexec()?;
        Ok(true)
    }

    fn prepare_reexec(&self) -> Result<()> {
        // restore external resource, like: fd, ...
        // do nothing now

        // flush db
        self.reli.flush()?;

        // compact db
        self.reli.compact()?;
        Ok(())
    }

    fn reboot(&self, reboot_mode: RebootMode) {
        log::debug!("RebootMode: {:?}", reboot_mode);
        // self.start_unit("shutdown.target");
        if let Ok(mut cg_ctrl) = CgController::new("sysmaster", Pid::from_raw(0)) {
            if let Err(e) = cg_ctrl.trim(false) {
                log::debug!("CgController trim err: {}", e.to_string());
            }
        }

        let mut pids = process::kill_all_pids(15);
        pids = process::wait_pids(pids, 10000000);
        if !pids.is_empty() {
            pids = process::kill_all_pids(9);
            process::wait_pids(pids, 10000000);
        }

        log::info!("Rebooting...");
        log::flush!();
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

        // time
        let timer = Rc::clone(&self.alive_timer);
        timer.enable(true);
    }

    fn run_generators(&self) {
        let paths = vec!["/usr/lib/sysmaster/system-generators"];

        if let Err(err) = basic::exec::execute_directories(paths) {
            log::error!("run generators err: {}", err);
        }
    }
}

/// manager running mode
#[allow(missing_docs)]
#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug)]
pub enum Mode {
    System,
    User,
}

/// manager action mode
#[allow(missing_docs)]
#[allow(dead_code)]
pub enum Action {
    Run,
    Help,
    Test,
}

/// manager running states
#[allow(missing_docs)]
#[allow(dead_code)]
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
    #[allow(dead_code)]
    pub(crate) fn get_job(&self, _id: JobId) -> Result<()> {
        todo!()
    }

    #[allow(dead_code)]
    pub(crate) fn get_unit(&self, _name: &str) -> Result<()> {
        todo!()
    }

    #[allow(dead_code)]
    pub(crate) fn clear_jobs(&self) -> Result<()> {
        todo!()
    }

    #[allow(dead_code)]
    pub(crate) fn reset_failed(&mut self) -> Result<()> {
        todo!()
    }

    #[allow(dead_code)]
    pub(crate) fn exit(&self) -> Result<i32> {
        self.set_state(State::Exit);
        Ok(0)
    }

    #[allow(dead_code)]
    pub(crate) fn poweroff(&self) -> Result<i32> {
        self.set_state(State::PowerOff);
        Ok(0)
    }

    #[allow(dead_code)]
    pub(crate) fn halt(&self) -> Result<i32> {
        self.set_state(State::Halt);
        Ok(0)
    }

    #[allow(dead_code)]
    pub(crate) fn kexec(&self) -> Result<i32> {
        self.set_state(State::KExec);
        Ok(0)
    }

    #[allow(dead_code)]
    pub(crate) fn suspend(&self) -> Result<i32> {
        self.set_state(State::Suspend);
        Ok(0)
    }

    #[allow(dead_code)]
    pub(crate) fn switch_root(&self) -> Result<i32> {
        self.set_state(State::SwitchRoot);
        Ok(0)
    }

    #[allow(dead_code)]
    pub(super) fn ok(&self) {
        self.set_state(State::Ok);
    }

    #[allow(dead_code)]
    pub(super) fn check_finished(&self) -> Result<()> {
        todo!()
    }

    fn set_state(&self, state: State) {
        *self.state.borrow_mut() = state;
    }

    fn state(&self) -> State {
        *self.state.borrow()
    }

    pub(crate) fn preset_all(&self) -> Result<()> {
        if self.mode != Mode::System {
            return Ok(());
        }

        let install = Install::new(PresetMode::Enable, self.lookup_path.clone());
        install.preset_all().unwrap();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //#[test]
    #[allow(dead_code)]
    fn manager_api() {
        log::init_log_to_console("manager_api", log::Level::Trace);

        // new
        let manager = Manager::new(
            Mode::System,
            Action::Run,
            Rc::new(RefCell::new(ManagerConfig::new(&Mode::System))),
        );
        manager.reli.data_clear(); // clear all data

        // startup
        let ret = manager.startup(false);
        assert!(ret.is_ok());
    }
}
