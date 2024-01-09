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

use super::base::UeBase;
use super::bus::UeBus;
use super::cgroup::UeCgroup;
use super::child::UeChild;
use super::condition::{assert_keys::*, condition_keys::*, UeCondition};
use super::config::UeConfig;
use super::load::UeLoad;
use super::ratelimit::StartLimit;
use super::{UnitEmergencyAction, UnitX};
use crate::unit::data::{DataManager, UnitState};
use crate::unit::rentry::{UnitLoadState, UnitRe};
use crate::unit::util::UnitFile;
use basic::process::{self, my_child};
use basic::time::{now_clockid, UnitTimeStamp};
use cgroup::{self, CgFlags};
use core::error::*;
use core::rel::ReStation;
use core::unit::{KillContext, KillMode, KillOperation, UnitNotifyFlags, UnitWriteFlags};
use core::unit::{SubUnit, UnitActiveState, UnitBase, UnitType};
use libc::{CLOCK_MONOTONIC, CLOCK_REALTIME};
use nix::sys::socket::UnixCredentials;
use nix::sys::wait::WaitStatus;
use nix::unistd::Pid;
use nix::NixPath;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::rc::Rc;

///
pub struct Unit {
    // associated objects
    dm: Rc<DataManager>,

    // owned objects
    base: Rc<UeBase>,

    config: Rc<UeConfig>,
    load: UeLoad,
    child: UeChild,
    cgroup: UeCgroup,
    conditions: Rc<UeCondition>,
    start_limit: StartLimit,
    sub: Box<dyn SubUnit>,
    merged_into: RefCell<Option<Rc<UnitX>>>,
    in_stop_when_bound_queue: RefCell<bool>,
    timestamp: Rc<RefCell<UnitTimeStamp>>,
    bus: UeBus,
}

impl PartialEq for Unit {
    fn eq(&self, other: &Self) -> bool {
        self.base.unit_type() == other.base.unit_type() && self.base.id() == other.base.id()
    }
}

impl Eq for Unit {}

impl PartialOrd for Unit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Unit {
    fn cmp(&self, other: &Self) -> Ordering {
        self.base.id().cmp(&other.base.id())
    }
}

impl Hash for Unit {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.base.id().hash(state);
    }
}

impl ReStation for Unit {
    // no input, no compensate

    // data
    fn db_map(&self, reload: bool) {
        self.base.db_map(reload);
        self.config.db_map(reload);
        self.cgroup.db_map(reload);
        self.load.db_map(reload);
        self.child.db_map(reload);

        self.sub.db_map(reload);
    }

    // data insert
    fn db_insert(&self) {
        self.base.db_insert();
        self.config.db_insert();
        self.cgroup.db_insert();
        self.load.db_insert();
        self.child.db_insert();

        self.sub.db_insert();
    }

    // reload: entry-only
    fn entry_coldplug(&self) {
        // rebuild external connections, like: timer, ...
        // unit-frame: do nothing now

        // sub
        self.sub.entry_coldplug();
    }

    fn entry_clear(&self) {
        // release external connection, like: timer, ...
        // do nothing now

        self.sub.entry_clear();
    }
}

impl UnitBase for Unit {
    fn id(&self) -> String {
        self.id()
    }

    fn unit_type(&self) -> UnitType {
        self.unit_type()
    }

    /*fn get_dependency_list(&self, _unit_name: &str, _atom: libcore::unit::UnitRelationAtom) -> Vec<Rc<Self>> {
        todo!()
    }*/

    fn test_start_limit(&self) -> bool {
        self.test_start_limit()
    }

    fn reset_start_limit(&self) {
        self.reset_start_limit()
    }

    fn kill_context(
        &self,
        k_context: Rc<KillContext>,
        m_pid: Option<Pid>,
        c_pid: Option<Pid>,
        ko: KillOperation,
        main_pid_alien: bool,
    ) -> Result<bool> {
        self.kill_context(k_context, m_pid, c_pid, ko, main_pid_alien)
    }

    fn notify(
        &self,
        original_state: UnitActiveState,
        new_state: UnitActiveState,
        flags: UnitNotifyFlags,
    ) {
        self.notify(original_state, new_state, flags);
    }

    fn prepare_exec(&self) -> Result<()> {
        self.prepare_exec()
    }

    fn default_dependencies(&self) -> bool {
        self.default_dependencies()
    }

    fn cg_path(&self) -> PathBuf {
        self.cg_path()
    }

    fn ignore_on_isolate(&self) -> bool {
        self.ignore_on_isolate()
    }

    fn set_ignore_on_isolate(&self, ignore_on_isolate: bool) {
        self.set_ignore_on_isolate(ignore_on_isolate);
    }

    fn guess_main_pid(&self) -> Result<Pid> {
        self.guess_main_pid()
    }

    fn get_unit_timestamp(&self) -> Rc<RefCell<UnitTimeStamp>> {
        self.get_unit_timestamp()
    }

    fn is_load_stub(&self) -> bool {
        self.load.load_state() == UnitLoadState::Stub
    }

    fn transient(&self) -> bool {
        self.load.transient()
    }

    fn transient_file(&self) -> Option<PathBuf> {
        self.load.transient_file()
    }

    fn last_section_private(&self) -> i8 {
        self.load.last_section_private()
    }

    fn set_last_section_private(&self, lsp: i8) {
        self.load.set_last_section_private(lsp);
    }
}

impl Unit {
    /// need to consider use box or rc?
    pub(super) fn new(
        unit_type: UnitType,
        name: &str,
        dmr: &Rc<DataManager>,
        rentryr: &Rc<UnitRe>,
        filer: &Rc<UnitFile>,
        sub: Box<dyn SubUnit>,
    ) -> Rc<Unit> {
        let _base = Rc::new(UeBase::new(rentryr, String::from(name), unit_type));
        let _config = Rc::new(UeConfig::new(&_base));
        let _load = Rc::new(UeLoad::new(dmr, filer, &_base, &_config));
        let _u = Rc::new(Unit {
            dm: Rc::clone(dmr),
            base: Rc::clone(&_base),
            config: Rc::clone(&_config),
            load: UeLoad::new(dmr, filer, &_base, &_config),
            child: UeChild::new(&_base),
            cgroup: UeCgroup::new(&_base),
            conditions: Rc::new(UeCondition::new()),
            sub,
            start_limit: StartLimit::new(),
            merged_into: RefCell::new(None),
            in_stop_when_bound_queue: RefCell::new(false),
            timestamp: Rc::new(RefCell::new(UnitTimeStamp::default())),
            bus: UeBus::new(&_config),
        });
        let owner = Rc::clone(&_u);
        _u.sub.attach_unit(owner);
        _u
    }

    fn conditions(&self) -> Rc<UeCondition> {
        let flag = self.conditions.init_flag();
        if flag != 0 {
            return Rc::clone(&self.conditions);
        } else {
            //need to reconstruct the code, expose the config detail out is wrong
            macro_rules! add_condition_simplified {
                ($key: ident, $value: ident) => {
                    let params = self
                        .get_config()
                        .config_data()
                        .borrow()
                        .Unit
                        .$value
                        .to_string();
                    if !params.is_empty() {
                        self.conditions.add_condition($key, params);
                    }
                };
            }

            macro_rules! add_assert_simplified {
                ($key: ident, $value: ident) => {
                    let params = self
                        .get_config()
                        .config_data()
                        .borrow()
                        .Unit
                        .$value
                        .to_string();
                    if !params.is_empty() {
                        self.conditions.add_assert($key, params);
                    }
                };
            }

            // ConditionACPower is different, it's Option<bool>, not String.
            if let Some(v) = self
                .get_config()
                .config_data()
                .borrow()
                .Unit
                .ConditionACPower
            {
                self.conditions
                    .add_condition(CONDITION_AC_POWER, v.to_string());
            }

            add_condition_simplified!(CONDITION_CAPABILITY, ConditionCapability);
            add_condition_simplified!(CONDITION_DIRECTORY_NOT_EMPTY, ConditionDirectoryNotEmpty);
            add_condition_simplified!(CONDITION_FILE_IS_EXECUTABLE, ConditionFileIsExecutable);
            add_condition_simplified!(CONDITION_FILE_NOT_EMPTY, ConditionFileNotEmpty);

            // Same as ConditionACPower, it's Option<bool>.
            if let Some(v) = self
                .get_config()
                .config_data()
                .borrow()
                .Unit
                .ConditionFirstBoot
            {
                self.conditions
                    .add_condition(CONDITION_FIRST_BOOT, v.to_string());
            }

            add_condition_simplified!(CONDITION_KERNEL_COMMAND_LINE, ConditionKernelCommandLine);
            add_condition_simplified!(CONDITION_NEEDS_UPDATE, ConditionNeedsUpdate);
            add_condition_simplified!(CONDITION_PATH_EXISTS, ConditionPathExists);
            add_condition_simplified!(CONDITION_PATH_EXISTS_GLOB, ConditionPathExistsGlob);
            add_condition_simplified!(CONDITION_PATH_IS_DIRECTORY, ConditionPathIsDirectory);
            add_condition_simplified!(CONDITION_PATH_IS_MOUNT_POINT, ConditionPathIsMountPoint);
            add_condition_simplified!(CONDITION_PATH_IS_READ_WRITE, ConditionPathIsReadWrite);
            add_condition_simplified!(CONDITION_PATH_IS_SYMBOLIC_LINK, ConditionPathIsSymbolicLink);
            add_condition_simplified!(CONDITION_SECURITY, ConditionSecurity);
            add_condition_simplified!(CONDITION_USER, ConditionUser);

            add_assert_simplified!(ASSERT_PATH_EXISTS, AssertPathExists);
        }
        Rc::clone(&self.conditions)
    }

    pub fn unit_trigger_notify(&self) {
        self.sub.trigger_notify()
    }

    ///
    pub fn notify(
        &self,
        original_state: UnitActiveState,
        new_state: UnitActiveState,
        flags: UnitNotifyFlags,
    ) {
        if original_state != new_state {
            log::debug!(
                "unit active state change from: {:?} to {:?}",
                original_state,
                new_state
            );
        }

        let mut unit_timestamp = self.timestamp.borrow_mut();

        unit_timestamp.state_change_timestamp.realtime = now_clockid(CLOCK_REALTIME);
        unit_timestamp.state_change_timestamp.monotonic = now_clockid(CLOCK_MONOTONIC);

        if original_state.is_inactive_or_failed() && !new_state.is_inactive_or_failed() {
            unit_timestamp.inactive_exit_timestamp = unit_timestamp.state_change_timestamp;
        } else if !original_state.is_inactive_or_failed() && new_state.is_inactive_or_failed() {
            unit_timestamp.inactive_enter_timestamp = unit_timestamp.state_change_timestamp;
        }

        if !original_state.is_active_or_reloading() && new_state.is_active_or_reloading() {
            unit_timestamp.active_enter_timestamp = unit_timestamp.state_change_timestamp;
        } else if original_state.is_active_or_reloading() && !new_state.is_active_or_reloading() {
            unit_timestamp.active_exit_timestamp = unit_timestamp.state_change_timestamp;
        }

        let u_state = UnitState::new(original_state, new_state, flags);
        self.dm.insert_unit_state(self.id(), u_state);
    }

    ///
    pub fn id(&self) -> String {
        self.base.id()
    }

    ///
    pub fn set_id(&self, id: &str) {
        self.base.set_id(id)
    }

    /// return pids of the unit
    pub fn get_pids(&self) -> Vec<Pid> {
        self.child.get_pids()
    }

    /// return description
    pub fn get_description(&self) -> Option<String> {
        self.load.get_description()
    }

    /// return documentation
    pub fn get_documentation(&self) -> Option<String> {
        self.load.get_documentation()
    }

    ///
    pub fn prepare_exec(&self) -> Result<()> {
        log::debug!("prepare exec cgroup");
        self.cgroup.setup_cg_path();

        self.cgroup
            .prepare_cg_exec()
            .map_err(|_| core::error::Error::ConvertToSysmaster)
    }

    /// return the cgroup name of the unit
    pub fn cg_path(&self) -> PathBuf {
        self.cgroup.cg_path()
    }

    /// kill the process belongs to the unit
    pub fn kill_context(
        &self,
        k_context: Rc<KillContext>,
        m_pid: Option<Pid>,
        c_pid: Option<Pid>,
        ko: KillOperation,
        main_pid_alien: bool,
    ) -> Result<bool> {
        let mut wait_exit = false;
        let sig = ko.to_signal(k_context.clone());
        log::debug!(
            "unit: {}, kill operation: {:?}, kill signal: {}, main_pid: {:?}, control_pid: {:?}",
            self.id(),
            ko,
            sig,
            m_pid,
            c_pid
        );
        if let Some(pid) = m_pid {
            match process::kill_and_cont(pid, sig) {
                Ok(_) => {
                    if !main_pid_alien {
                        wait_exit = true;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to kill pid {}, errno: {}", pid, e);
                }
            }
        }
        if let Some(pid) = c_pid {
            match process::kill_and_cont(pid, sig) {
                Ok(_) => {
                    wait_exit = true;
                }
                Err(e) => {
                    log::warn!("Failed to kill pid {}, errno: {}", pid, e);
                }
            }
        }

        if !self.cgroup.cg_path().is_empty()
            && (k_context.kill_mode() == KillMode::ControlGroup
                || (k_context.kill_mode() == KillMode::Mixed && ko == KillOperation::KillKill))
        {
            let pids = self.pids_set(m_pid, c_pid);

            match cgroup::cg_kill_recursive(
                &self.cg_path(),
                sig,
                CgFlags::IGNORE_SELF | CgFlags::SIGCONT,
                pids,
            ) {
                Ok(_) => {}
                Err(_) => {
                    log::debug!("failed to kill cgroup context, {:?}", self.cg_path());
                }
            }
        }

        Ok(wait_exit)
    }

    ///
    pub fn default_dependencies(&self) -> bool {
        self.get_config()
            .config_data()
            .borrow()
            .Unit
            .DefaultDependencies
    }

    ///
    pub fn ignore_on_isolate(&self) -> bool {
        self.get_config()
            .config_data()
            .borrow()
            .Unit
            .IgnoreOnIsolate
    }

    ///
    pub fn set_ignore_on_isolate(&self, ignore_on_isolate: bool) {
        self.get_config()
            .config_data()
            .borrow_mut()
            .Unit
            .IgnoreOnIsolate = ignore_on_isolate;
    }

    /// guess main pid from the cgroup path
    pub fn guess_main_pid(&self) -> Result<Pid> {
        let cg_path = self.cgroup.cg_path();

        if cg_path.is_empty() {
            return Err(
                "cgroup path is empty, can not guess main pid from cgroup path"
                    .to_string()
                    .into(),
            );
        }
        let pids = cgroup::cg_get_pids(&cg_path);
        if pids.is_empty() {
            return Err(format!("No process in cgroup path: {:?}", cg_path).into());
        }
        let mut main_pid = Pid::from_raw(0);

        for pid in pids {
            if pid == main_pid {
                continue;
            }

            if !my_child(pid) {
                continue;
            }

            main_pid = pid;
            break;
        }
        Ok(main_pid)
    }

    fn pids_set(&self, m_pid: Option<Pid>, c_pid: Option<Pid>) -> HashSet<Pid> {
        let mut pids = HashSet::new();

        if let Some(pid) = m_pid {
            pids.insert(pid);
        }

        if let Some(pid) = c_pid {
            pids.insert(pid);
        }

        pids
    }

    ///
    pub fn get_success_action(&self) -> UnitEmergencyAction {
        self.config.config_data().borrow().Unit.SuccessAction
    }

    ///
    pub fn get_failure_action(&self) -> UnitEmergencyAction {
        self.config.config_data().borrow().Unit.FailureAction
    }

    ///
    pub fn get_start_limit_action(&self) -> UnitEmergencyAction {
        self.config.config_data().borrow().Unit.StartLimitAction
    }

    pub fn get_job_timeout_action(&self) -> UnitEmergencyAction {
        self.config.config_data().borrow().Unit.JobTimeoutAction
    }

    ///
    pub fn current_active_state(&self) -> UnitActiveState {
        self.sub.current_active_state()
    }

    ///
    pub fn get_subunit_state(&self) -> String {
        self.sub.get_subunit_state()
    }

    /// test start rate, if start more than burst times in interval time, return error
    fn test_start_limit(&self) -> bool {
        if self.config.config_data().borrow().Unit.StartLimitInterval > 0
            && self.config.config_data().borrow().Unit.StartLimitBurst > 0
        {
            self.start_limit.init_from_config(
                self.config.config_data().borrow().Unit.StartLimitInterval,
                self.config.config_data().borrow().Unit.StartLimitBurst,
            );
        }

        if self.start_limit.ratelimit_below() {
            self.start_limit.set_hit(false);
            self.dm
                .insert_start_limit_result(self.id(), super::StartLimitResult::StartLimitNotHit);
            return true;
        }

        self.start_limit.set_hit(true);
        self.dm
            .insert_start_limit_result(self.id(), super::StartLimitResult::StartLimitHit);
        false
    }

    fn reset_start_limit(&self) {
        self.start_limit.reset_limit()
    }

    ///
    pub(super) fn get_config(&self) -> Rc<UeConfig> {
        self.config.clone()
    }

    pub(super) fn trigger(&self, other: &Self) {
        let other_unit_id = other.id();
        self.sub.trigger(&other_unit_id);
    }

    pub(super) fn in_load_queue(&self) -> bool {
        self.load.in_load_queue()
    }

    pub(super) fn set_in_load_queue(&self, t: bool) {
        self.load.set_in_load_queue(t);
    }

    pub(super) fn in_target_dep_queue(&self) -> bool {
        self.load.in_target_dep_queue()
    }

    pub(super) fn set_in_target_dep_queue(&self, t: bool) {
        self.load.set_in_target_dep_queue(t);
    }

    pub(super) fn in_stop_when_bound_queue(&self) -> bool {
        *self.in_stop_when_bound_queue.borrow()
    }

    pub(super) fn set_in_stop_when_bound_queue(&self, t: bool) {
        *self.in_stop_when_bound_queue.borrow_mut() = t
    }

    pub(super) fn get_real_name(&self) -> String {
        self.load.get_real_name()
    }

    pub(super) fn get_all_names(&self) -> Vec<String> {
        self.load.get_all_names()
    }

    pub(super) fn set_merge_into(&self, unit: Option<Rc<UnitX>>) {
        *self.merged_into.borrow_mut() = unit;
    }

    pub(super) fn merged_into(&self) -> Option<Rc<UnitX>> {
        self.merged_into.borrow().clone()
    }

    pub(super) fn load_unit(&self) -> Result<()> {
        self.set_in_load_queue(false);
        self.load.finalize_transient()?;
        match self.load.load_unit_confs() {
            Ok(_) => {
                let paths = self.load.get_unit_id_fragment_pathbuf();
                log::debug!("Begin exec sub class load");

                if let Err(err) = self.sub.load(paths) {
                    if let Error::Nix { source } = err {
                        if source == nix::Error::ENOEXEC {
                            self.load.set_load_state(UnitLoadState::BadSetting);
                            return Err(err);
                        }
                    }
                    self.load.set_load_state(UnitLoadState::Error);
                    return Err(err);
                }

                self.load.set_load_state(UnitLoadState::Loaded);
                Ok(())
            }
            Err(e) => {
                self.load.set_load_state(UnitLoadState::NotFound);
                Err(e)
            }
        }
    }

    /// Stub or Merges is temporarily state which represent not load complete
    pub(super) fn load_complete(&self) -> bool {
        self.load_state() != UnitLoadState::Stub && self.load_state() != UnitLoadState::Merged
    }

    ///
    pub(super) fn validate_load_state(&self) -> Result<()> {
        match self.load_state() {
            UnitLoadState::Stub | UnitLoadState::Merged => Err(Error::LoadError {
                msg: format!("unexpected load state of unit: {}", self.id()),
            }),
            UnitLoadState::Loaded => Ok(()),
            UnitLoadState::NotFound => Err(Error::LoadError {
                msg: format!("unit file is not found: {}", self.id()),
            }),
            UnitLoadState::Error => Err(Error::LoadError {
                msg: format!("load unit file failed, adjust the unit file: {}", self.id()),
            }),
            UnitLoadState::BadSetting => Err(Error::LoadError {
                msg: format!("unit file {} has bad setting", self.id()),
            }),
            UnitLoadState::Masked => Err(Error::LoadError {
                msg: format!("unit file {} is masked", self.id()),
            }),
        }
    }

    ///
    pub(super) fn get_perpetual(&self) -> bool {
        self.sub.get_perpetual()
    }

    ///
    pub fn start(&self) -> Result<()> {
        let active_state = self.current_active_state();
        if active_state.is_active_or_reloading() {
            log::debug!(
                "The unit {} is already active or reloading, skipping.",
                self.id()
            );
            return Err(Error::UnitActionEAlready);
        }

        if active_state == UnitActiveState::Maintenance {
            log::error!("Failed to start {}: unit is in maintenance", self.id());
            return Err(Error::UnitActionEAgain);
        }

        if self.load_state() != UnitLoadState::Loaded {
            log::error!("Failed to start {}: unit hasn't been loaded.", self.id());
            return Err(Error::UnitActionEInval);
        }

        if active_state != UnitActiveState::Activating && !self.conditions().conditions_test() {
            log::info!("The condition check failed, not starting {}.", self.id());
            return Err(Error::UnitActionEComm);
        }

        if active_state != UnitActiveState::Activating && !self.conditions().asserts_test() {
            log::info!("The assert check failed, not starting {}.", self.id());
            return Err(Error::UnitActionEProto);
        }

        self.sub.start()
    }

    ///
    pub fn stop(&self, force: bool) -> Result<()> {
        if !force {
            let active_state = self.current_active_state();
            let inactive_or_failed = matches!(
                active_state,
                UnitActiveState::InActive | UnitActiveState::Failed
            );

            if inactive_or_failed {
                log::debug!(
                    "The unit {} is already inactive or dead, skipping.",
                    self.id()
                );
                return Err(Error::UnitActionEAlready);
            }
        }

        self.sub.stop(force)
    }

    /// reload the unit
    pub fn reload(&self) -> Result<()> {
        if !self.sub.can_reload() {
            log::info!("Unit {} can not be reloaded", self.id());
            return Err(Error::UnitActionEBadR);
        }

        let active_state = self.current_active_state();
        if active_state == UnitActiveState::Reloading {
            log::info!("Unit {} is being reloading", self.id());
            return Err(Error::UnitActionEAgain);
        }

        if active_state != UnitActiveState::Active {
            log::info!("Unit {} is not active, no need to reload", self.id());
            return Err(Error::UnitActionENoExec);
        }

        log::info!("Reloading {}", self.id());
        match self.sub.reload() {
            Ok(_) => Ok(()),
            Err(e) => match e {
                Error::UnitActionEOpNotSupp => {
                    self.notify(active_state, active_state, UnitNotifyFlags::EMPTY);
                    Ok(())
                }
                _ => Err(e),
            },
        }
    }

    pub(crate) fn reset_failed(&self) {
        self.sub.reset_failed()
    }

    pub(super) fn sigchld_events(&self, wait_status: WaitStatus) {
        self.sub.sigchld_events(wait_status)
    }

    pub fn load_state(&self) -> UnitLoadState {
        self.load.load_state()
    }

    pub(super) fn load_paths(&self) -> Vec<PathBuf> {
        self.load.paths()
    }

    pub(super) fn transient(&self) -> bool {
        self.load.transient()
    }

    pub fn set_load_state(&self, state: UnitLoadState) {
        self.load.set_load_state(state)
    }

    pub(super) fn make_transient(&self, path: Option<PathBuf>) {
        self.load.make_transient(path)
    }

    pub(super) fn remove_transient(&self) {
        self.load.remove_transient()
    }

    pub(super) fn child_add_pids(&self, pid: Pid) {
        self.child.add_pids(pid);
    }

    pub(super) fn child_remove_pids(&self, pid: Pid) {
        self.child.remove_pids(pid);
    }

    pub(super) fn unit_type(&self) -> UnitType {
        self.base.unit_type()
    }

    pub(super) fn collect_fds(&self) -> Vec<i32> {
        self.sub.collect_fds()
    }

    pub(crate) fn set_socket_fd(&self, fd: i32) {
        self.sub.set_socket_fd(fd)
    }

    pub(crate) fn release_socket_fd(&self, fd: i32) {
        self.sub.release_socket_fd(fd)
    }

    pub(crate) fn setup_existing_mount(
        &self,
        what: &str,
        mount_where: &str,
        options: &str,
        fstype: &str,
    ) {
        self.sub
            .setup_existing_mount(what, mount_where, options, fstype);
    }

    pub(crate) fn setup_new_mount(
        &self,
        what: &str,
        mount_where: &str,
        options: &str,
        fstype: &str,
    ) {
        self.sub.setup_new_mount(what, mount_where, options, fstype);
    }

    pub(crate) fn update_mount_state_by_mountinfo(&self) {
        self.sub.update_mount_state_by_mountinfo();
    }

    pub(crate) fn notify_message(
        &self,
        ucred: &UnixCredentials,
        messages: &HashMap<&str, &str>,
        fds: Vec<i32>,
    ) -> Result<()> {
        self.sub.notify_message(ucred, messages, fds)
    }

    pub fn get_unit_timestamp(&self) -> Rc<RefCell<UnitTimeStamp>> {
        Rc::clone(&self.timestamp)
    }

    pub(crate) fn set_sub_property(
        &self,
        key: &str,
        value: &str,
        flags: UnitWriteFlags,
    ) -> Result<()> {
        self.sub.unit_set_property(key, value, flags)
    }

    pub(crate) fn set_property(&self, key: &str, value: &str) -> Result<()> {
        self.bus.set_property(key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::Unit;
    use crate::manager::RELI_HISTORY_MAX_DBS;
    use crate::unit::rentry::UnitRe;
    use crate::unit::test::test_utils::UmIfD;
    use basic::fs::LookupPaths;
    use core::rel::{ReliConf, Reliability};
    use core::unit::UnitType;
    use std::rc::Rc;

    use crate::{
        unit::data::DataManager,
        unit::util::{self, UnitFile},
    };
    fn unit_init() -> Rc<Unit> {
        log::init_log_to_console("unit_init", log::Level::Trace);
        let reli = Rc::new(Reliability::new(
            ReliConf::new().set_max_dbs(RELI_HISTORY_MAX_DBS),
        ));
        let rentry = Rc::new(UnitRe::new(&reli));

        let mut l_path = LookupPaths::new();
        let test_units_dir = libtests::get_project_root()
            .unwrap()
            .join("tests/test_units/")
            .to_string_lossy()
            .to_string();
        l_path.search_path.push(test_units_dir);
        let lookup_path = Rc::new(l_path);
        let unit_file = UnitFile::new(&lookup_path);

        let dm = DataManager::new();
        let umifd = Rc::new(UmIfD);
        let sub_obj = util::create_subunit_with_um(UnitType::UnitService, umifd.clone()).unwrap();
        sub_obj.attach_um(umifd);
        sub_obj.attach_reli(Rc::clone(&reli));
        Unit::new(
            UnitType::UnitService,
            "config.service",
            &Rc::new(dm),
            &rentry,
            &Rc::new(unit_file),
            sub_obj,
        )
    }

    #[test]
    fn test_unit_load() {
        let _unit = unit_init();
        let load_stat = _unit.load_unit();
        assert!(load_stat.is_ok());
        /*let stat = _unit.start();
        assert!(stat.is_ok());
        assert_eq!(_unit.current_active_state(),UnitActiveState::Active);*/
    }

    #[allow(dead_code)]
    fn test_unit_condition() {
        let _unit = unit_init();
        let load_stat = _unit.load_unit();
        assert!(load_stat.is_ok());
        assert!(_unit.conditions().conditions_test());
    }
}
