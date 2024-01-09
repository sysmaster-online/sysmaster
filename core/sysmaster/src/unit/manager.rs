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

///sysmaster entry
/// 1. Load all unit need loaded in a system
/// 2. Drive unit status through job engine;
/// 3. Mainlain all unit life cycle
///
///                    / ---->unit_load
/// ManagerX-> Manager | ---->job_manager
///                      ---->rentry
///
use super::super::job::{JobAffect, JobConf, JobKind, JobManager};
use super::bus::UnitBus;
use super::datastore::UnitDb;
use super::entry::{StartLimitResult, Unit, UnitEmergencyAction, UnitX};
use super::execute::ExecSpawn;
use super::notify::NotifyManager;
use super::rentry::{JobMode, UnitLoadState, UnitRe};
use super::runtime::UnitRT;
use super::sigchld::Sigchld;
use super::submanager::UnitSubManagers;
use super::uload::UnitLoad;
use crate::job::JobResult;
use crate::manager::config::ManagerConfig;
use crate::manager::pre_install::{Install, PresetMode};
use crate::manager::State;
use crate::unit::data::{DataManager, UnitState};
use crate::utils::table::{TableOp, TableSubscribe};
use basic::fs::LookupPaths;
use basic::show_table::{CellColor, ShowTable};
use basic::time::UnitTimeStamp;
use basic::{machine, process, rlimit, signal};
use cmdproto::proto::transient_unit_comm::UnitConfig;
use constants::SIG_SWITCH_ROOT_OFFSET;
use core::error::*;
use core::exec::ExecParameters;
use core::exec::{ExecCommand, ExecContext};
use core::rel::{ReStation, ReStationKind, ReliLastFrame, Reliability};
use core::unit::{
    unit_name_is_valid, UmIf, UnitActiveState, UnitDependencyMask, UnitNameFlags, UnitStatus,
    UnitType,
};
use core::unit::{UnitRelationAtom, UnitRelations};
use event::Events;
use libc::getppid;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::convert::TryFrom;
use std::fs::File;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;

//#[derive(Debug)]
pub(crate) struct UnitManagerX {
    dm: Rc<DataManager>,
    sub_name: String, // key for table-subscriber: UnitState
    data: Rc<UnitManager>,
    lookup_path: Rc<LookupPaths>,
    state: Rc<RefCell<State>>,
    #[allow(dead_code)]
    manager_config: Rc<RefCell<ManagerConfig>>,
}

impl Drop for UnitManagerX {
    fn drop(&mut self) {
        log::debug!("UnitManagerX drop, clear.");
        // repeating protection
        self.dm.clear();
    }
}

impl UnitManagerX {
    pub(crate) fn new(
        eventr: &Rc<Events>,
        relir: &Rc<Reliability>,
        lookup_path: &Rc<LookupPaths>,
        state: Rc<RefCell<State>>,
        manager_config: Rc<RefCell<ManagerConfig>>,
    ) -> UnitManagerX {
        let _dm = Rc::new(DataManager::new());
        let umx = UnitManagerX {
            dm: Rc::clone(&_dm),
            sub_name: String::from("UnitManagerX"),
            data: UnitManager::new(
                eventr,
                relir,
                &_dm,
                lookup_path,
                Rc::clone(&state),
                manager_config.clone(),
            ),
            lookup_path: Rc::clone(lookup_path),
            state,
            manager_config,
        };
        umx.register(&_dm, relir);
        umx
    }

    #[allow(unused)]
    pub(crate) fn get_state(&self) -> State {
        *((*self.state).borrow())
    }

    #[allow(unused)]
    pub(crate) fn set_state(&self, state: State) {
        *self.state.borrow_mut() = state;
    }

    pub(crate) fn register_ex(&self) {
        self.data.register_ex();
    }

    pub(crate) fn entry_clear(&self) {
        self.dm.entry_clear();
        self.data.entry_clear();
    }

    pub(crate) fn entry_coldplug(&self) {
        self.data.entry_coldplug();
    }

    pub(crate) fn start_unit(&self, name: &str, is_manual: bool, job_mode: &str) -> Result<()> {
        self.data.start_unit(name, is_manual, job_mode)
    }

    pub(crate) fn stop_unit(&self, name: &str, is_manual: bool) -> Result<()> {
        self.data.stop_unit(name, is_manual)
    }

    pub(crate) fn reload(&self, name: &str) -> Result<()> {
        self.data.reload(name)
    }

    pub(crate) fn reset_failed(&self, name: &str) -> Result<()> {
        self.data.reset_failed(name)
    }

    pub(crate) fn restart_unit(&self, name: &str, is_manual: bool) -> Result<()> {
        self.data.restart_unit(name, is_manual)
    }

    pub(crate) fn get_unit_status(&self, name: &str) -> Result<UnitStatus> {
        self.data.get_unit_status(name)
    }

    pub(crate) fn get_all_units(&self) -> Result<String> {
        self.data.get_all_units()
    }

    pub(crate) fn child_sigchld_enable(&self, enable: bool) -> i32 {
        self.data.sigchld.enable(enable)
    }

    pub(crate) fn dispatch_load_queue(&self) {
        self.data.rt.dispatch_load_queue()
    }

    pub(crate) fn dispatch_stop_when_bound_queue(&self) {
        self.data
            .rt
            .dispatch_stop_when_bound_queue(self.data.jm.clone());
    }

    fn register(&self, dm: &DataManager, relir: &Reliability) {
        // dm-unit_state
        let subscriber = Rc::clone(&self.data);
        let ret = dm.register_unit_state(&self.sub_name, subscriber.clone());
        assert!(ret.is_none());

        // dm-start_limit_result
        let ret = dm.register_start_limit_result(&self.sub_name, subscriber.clone());
        assert!(ret.is_none());

        let ret = dm.register_job_result(&self.sub_name, subscriber);
        assert!(ret.is_none());

        // reliability-station
        let station = Rc::clone(&self.data);
        let kind = ReStationKind::Level2;
        relir.station_register(&String::from("UnitManager"), kind, station);
    }

    pub(crate) fn enable_unit(&self, unit_file: &str) -> Result<()> {
        log::info!("Enabling unit {}.", unit_file);
        let install = Install::new(PresetMode::Disable, self.lookup_path.clone());
        install.unit_enable_files(unit_file)?;
        Ok(())
    }

    pub(crate) fn disable_unit(&self, unit_file: &str) -> Result<()> {
        log::info!("Disabling unit {}.", unit_file);
        let install = Install::new(PresetMode::Disable, self.lookup_path.clone());
        install.unit_disable_files(unit_file)?;
        Ok(())
    }

    pub(crate) fn mask_unit(&self, unit_file: &str) -> Result<()> {
        log::info!("Masking unit {}.", unit_file);
        let link_name_path = std::path::Path::new(basic::fs::ETC_SYSTEM_PATH).join(unit_file);
        let target_path = std::path::Path::new("/dev/null");
        basic::fs::symlink(
            target_path.to_str().unwrap(),
            link_name_path.to_str().unwrap(),
            false,
        )
        .context(UtilSnafu)
    }

    pub(crate) fn unmask_unit(&self, unit_file: &str) -> Result<()> {
        log::info!("Unmasking unit {}.", unit_file);
        let link_name_path = std::path::Path::new(basic::fs::ETC_SYSTEM_PATH).join(unit_file);
        if !link_name_path.exists() {
            return Ok(());
        }

        let target = match link_name_path.read_link() {
            Ok(target_path) => target_path,
            Err(_) => {
                return Ok(());
            }
        };

        if !target.ends_with("/dev/null") {
            return Ok(());
        }

        // So, this is a symlink points to /dev/null
        if let Err(e) = nix::unistd::unlinkat(
            None,
            &link_name_path,
            nix::unistd::UnlinkatFlags::NoRemoveDir,
        ) {
            log::warn!(
                "Failed to unlink {}: {}",
                link_name_path.to_str().unwrap(),
                e
            );
            return Err(e).context(NixSnafu);
        }

        Ok(())
    }

    pub(crate) fn switch_root(&self, init: &[String]) -> Result<()> {
        if machine::Machine::in_initrd(None) {
            let mut str_paras = String::new();
            for para in init {
                str_paras.push_str(&format!("{}\n", para));
            }
            if !str_paras.is_empty() {
                let mut para_file = File::create(constants::INIT_PARA_PATH)?;
                let metadata = para_file.metadata()?;
                let mut permissions = metadata.permissions();
                permissions.set_mode(0o600);
                para_file.set_permissions(permissions)?;
                para_file.write_all(str_paras.as_bytes()).map_err(|err| {
                    log::error!(
                        "Failed to write init para to {}: {}",
                        constants::INIT_PARA_PATH,
                        err
                    );
                    err
                })?;
            }

            signal::reset_all_signal_handlers();
            signal::reset_signal_mask();
            rlimit::rlimit_nofile_safe();
            let res = unsafe { libc::kill(getppid(), libc::SIGRTMIN() + SIG_SWITCH_ROOT_OFFSET) };
            if res == 0 {
                self.set_state(State::SwitchRoot);
            }
            Errno::result(res)
                .map(drop)
                .map_err(|e| Error::Nix { source: e })
        } else {
            log::warn!("not in initrd.");
            Err(Error::Other {
                msg: "not in initrd.".to_owned(),
            })
        }
    }

    pub(crate) fn start_transient_unit(
        &self,
        job_mode: &str,
        unit_config: &UnitConfig,
        aux_units: &[UnitConfig],
    ) -> Result<()> {
        self.data
            .start_transient_unit(job_mode, unit_config, aux_units)
    }
}

/// the struct for manager the unit instance
pub struct UnitManager {
    // associated objects
    events: Rc<Events>,
    reli: Rc<Reliability>,
    state: Rc<RefCell<State>>,

    // owned objects
    rentry: Rc<UnitRe>,
    db: Rc<UnitDb>,
    rt: Rc<UnitRT>,
    load: Rc<UnitLoad>,
    jm: Rc<JobManager>,
    exec: ExecSpawn,
    sigchld: Sigchld,
    notify: NotifyManager,
    sms: Rc<UnitSubManagers>,
    bus: UnitBus,
}

impl UmIf for UnitManager {
    /// check the unit s_u_name and t_u_name have atom relation. If 't_u_name' is empty checks if the unit has any dependency of that atom.
    fn unit_has_dependency(&self, s_u_name: &str, atom: UnitRelationAtom, t_u_name: &str) -> bool {
        let s_unit = if let Some(s_unit) = self.db.units_get(s_u_name) {
            s_unit
        } else {
            return false;
        };

        if t_u_name.is_empty() {
            return true;
        }

        let t_unit = if let Some(unit) = self.db.units_get(t_u_name) {
            unit
        } else {
            return false;
        };

        self.db.dep_is_dep_atom_with(&s_unit, atom, &t_unit)
    }

    ///add a unit dependency to th unit deplist
    /// can called by sub unit
    /// sub unit add some default dependency
    ///
    fn unit_add_dependency(
        &self,
        unit_name: &str,
        relation: UnitRelations,
        target_name: &str,
        add_ref: bool,
        mask: UnitDependencyMask,
    ) -> Result<()> {
        let s_unit = if let Some(unit) = self.load_unitx(unit_name) {
            unit
        } else {
            return Err(Error::UnitActionENoent);
        };
        let t_unit = if let Some(unit) = self.load_unitx(target_name) {
            unit
        } else {
            return Err(Error::UnitActionENoent);
        };

        self.rt
            .unit_add_dependency(s_unit, relation, t_unit, add_ref, mask);
        Ok(())
    }

    ///add two unit dependency to the unit
    /// can called by sub unit
    /// sub unit add some default dependency
    ///
    fn unit_add_two_dependency(
        &self,
        unit_name: &str,
        ra: UnitRelations,
        rb: UnitRelations,
        target_name: &str,
        add_ref: bool,
        mask: UnitDependencyMask,
    ) -> Result<()> {
        self.unit_add_dependency(unit_name, ra, target_name, add_ref, mask)?;

        self.unit_add_dependency(unit_name, rb, target_name, add_ref, mask)
    }

    /// load the unit for reference name
    fn load_unit_success(&self, name: &str) -> bool {
        if let Some(unit) = self.load_unitx(name) {
            return unit.load_state() == UnitLoadState::Loaded;
        }

        false
    }

    fn unit_enabled(&self, name: &str) -> Result<()> {
        let u = if let Some(unit) = self.db.units_get(name) {
            unit
        } else {
            return Err(Error::UnitActionENoent);
        };

        if u.load_state() != UnitLoadState::Loaded {
            log::error!("related service unit: {} is not loaded", name);
            return Err(Error::UnitActionENoent);
        }

        if u.activated() {
            return Err(Error::UnitActionEBusy);
        }

        Ok(())
    }

    /// check if there is already a job in process
    fn has_job(&self, name: &str) -> bool {
        let u = match self.db.units_get(name) {
            None => return false,
            Some(v) => v,
        };
        self.jm.has_job(&u)
    }

    /// check if there is already a stop job in process
    fn has_stop_job(&self, name: &str) -> bool {
        let u = match self.db.units_get(name) {
            None => return false,
            Some(v) => v,
        };
        self.jm.has_stop_job(&u)
    }

    /// check if there is already a start job in process
    fn has_start_job(&self, name: &str) -> bool {
        let u = match self.db.units_get(name) {
            None => return false,
            Some(v) => v,
        };
        self.jm.has_start_job(&u)
    }

    /// check the unit that will be triggered by {name} is in active or activating state
    fn relation_active_or_pending(&self, name: &str) -> bool {
        let deps = self.db.dep_gets(name, UnitRelations::UnitTriggers);
        let mut pending: bool = false;
        for dep in deps.iter() {
            if dep.active_or_activating() {
                pending = true;
                break;
            }
        }

        pending
    }

    fn unit_destroy_runtime_data(&self, runtime_directory: Vec<PathBuf>) -> Result<()> {
        for d in runtime_directory {
            if let Err(e) = std::fs::remove_dir_all(&d) {
                log::info!("Failed to remove {:?}: {}, ignoring", d, e);
            }
        }
        Ok(())
    }

    fn unit_start_by_job(&self, name: &str) -> Result<()> {
        self.start_unit(name, false, "replace")
    }

    ///
    fn events(&self) -> Rc<Events> {
        Rc::clone(&self.events)
    }

    fn child_unwatch_pid(&self, id: &str, pid: Pid) {
        self.child_unwatch_pid(id, pid)
    }

    fn rentry_trigger_merge(&self, unit_id: &str, force: bool) {
        self.jm.rentry_trigger_merge(unit_id, force)
    }

    ///
    fn trigger_unit(&self, lunit: &str) {
        self.jm.trigger_unit(lunit)
    }

    /// get trigger by id
    fn unit_get_trigger(&self, id: &str) -> String {
        let deps = self.get_dependency_list(id, UnitRelationAtom::UnitAtomTriggers);

        if !deps.is_empty() {
            return deps[0].clone();
        }

        String::new()
    }

    /// Tests whether the unit to trigger is loaded
    fn test_trigger_loaded(&self, id: &str) -> bool {
        let trigger = self.unit_get_trigger(id);
        if trigger.is_empty() {
            log::error!("{} Refusing to start, no unit to trigger.", id);
            return false;
        }

        match self.rentry.load_get(&trigger) {
            Some(state) => {
                if state.0 == UnitLoadState::Loaded {
                    return true;
                }
                log::error!(
                    "{} Refusing to start, unit {} to trigger not loaded.",
                    id,
                    trigger,
                );
                false
            }
            None => true,
        }
    }

    /// call the exec spawn to start the child service
    fn exec_spawn(
        &self,
        unit: &str,
        cmdline: &ExecCommand,
        params: &mut ExecParameters,
        ctx: Rc<ExecContext>,
    ) -> Result<Pid> {
        let unit = self.units_get(unit);
        if let Some(u) = unit {
            self.exec.spawn(&u, cmdline, params, ctx)
        } else {
            Err(Error::SpawnError)
        }
    }

    /// add pid and its correspond unit to
    fn child_watch_pid(&self, id: &str, pid: Pid) {
        self.db.child_add_watch_pid(id, pid)
    }

    fn child_watch_all_pids(&self, id: &str) {
        self.db.child_watch_all_pids(id)
    }

    fn child_unwatch_all_pids(&self, id: &str) {
        self.db.child_unwatch_all_pids(id)
    }

    fn notify_socket(&self) -> Option<PathBuf> {
        self.notify.notify_sock()
    }

    fn same_unit_with_pid(&self, unit: &str, pid: Pid) -> bool {
        if !process::valid_pid(pid) {
            return false;
        }

        let p_unit = self.db.get_unit_by_pid(pid);
        if p_unit.is_none() {
            return false;
        }

        if p_unit.unwrap().id() == unit {
            return true;
        }

        false
    }

    fn collect_socket_fds(&self, name: &str) -> Vec<i32> {
        let deps = self.db.dep_gets(name, UnitRelations::UnitTriggeredBy);
        let mut fds = Vec::new();
        for dep in deps.iter() {
            if dep.unit_type() != UnitType::UnitSocket {
                continue;
            }

            fds.extend(dep.collect_fds())
        }

        fds
    }

    fn get_dependency_list(&self, unit_name: &str, atom: UnitRelationAtom) -> Vec<String> {
        let s_unit = if let Some(unit) = self.db.units_get(unit_name) {
            unit
        } else {
            log::error!("unit [{}] not found!!!!!", unit_name);
            return Vec::new();
        };
        let dep_units = self.db.dep_gets_atom(&s_unit, atom);
        dep_units
            .iter()
            .map(|uxr| uxr.unit().id())
            .collect::<Vec<_>>()
    }

    fn get_unit_timestamp(&self, unit_name: &str) -> Rc<RefCell<UnitTimeStamp>> {
        let s_unit = if let Some(unit) = self.db.units_get(unit_name) {
            unit
        } else {
            log::error!("unit [{}] not found!!!!!", unit_name);
            return Rc::new(RefCell::new(UnitTimeStamp::default()));
        };
        s_unit.unit().get_unit_timestamp()
    }

    fn unit_has_default_dependecy(&self, _unit_name: &str) -> bool {
        let s_unit = if let Some(s_unit) = self.db.units_get(_unit_name) {
            s_unit
        } else {
            return false;
        };
        s_unit.default_dependencies()
    }

    fn units_get_all(&self, unit_type: Option<UnitType>) -> Vec<String> {
        let units = self.db.units_get_all(unit_type);
        units.iter().map(|uxr| uxr.unit().id()).collect::<Vec<_>>()
    }

    fn current_active_state(&self, _unit_name: &str) -> UnitActiveState {
        let s_unit = if let Some(s_unit) = self.db.units_get(_unit_name) {
            s_unit
        } else {
            return UnitActiveState::Failed;
        };
        s_unit.current_active_state()
    }

    fn get_subunit_state(&self, _unit_name: &str) -> String {
        let s_unit = if let Some(s_unit) = self.db.units_get(_unit_name) {
            s_unit
        } else {
            return String::new();
        };
        s_unit.get_subunit_state()
    }

    fn unit_start_directly(&self, _name: &str) -> Result<()> {
        if let Some(unit) = self.db.units_get(_name) {
            unit.start()
        } else {
            Err(Error::UnitActionENoent)
        }
    }

    fn unit_stop(&self, _name: &str, force: bool) -> Result<()> {
        if let Some(unit) = self.db.units_get(_name) {
            unit.stop(force)
        } else {
            Err(Error::UnitActionENoent)
        }
    }

    fn restart_unit(&self, name: &str, is_manual: bool) -> Result<()> {
        let unit = match self.load_unitx(name) {
            None => {
                return Err(Error::UnitActionENoent);
            }
            Some(v) => v,
        };

        if is_manual
            && unit
                .get_config()
                .config_data()
                .borrow()
                .Unit
                .RefuseManualStop
        {
            return Err(Error::UnitActionERefuseManualStop);
        }

        if unit
            .get_config()
            .config_data()
            .borrow()
            .Unit
            .RefuseManualStart
        {
            return Err(Error::UnitActionERefuseManualStart);
        }

        self.jm.exec(
            &JobConf::new(&unit, JobKind::Restart),
            JobMode::Replace,
            &mut JobAffect::new(false),
        )?;
        Ok(())
    }

    fn private_section(&self, unit_type: UnitType) -> String {
        self.sms.private_section(unit_type)
    }

    /// set the service's socket fd
    fn service_set_socket_fd(&self, service_name: &str, fd: i32) {
        let service = match self.units_get(service_name) {
            None => return,
            Some(v) => v,
        };
        service.set_socket_fd(fd);
    }

    /// release the service's socket fd
    fn service_release_socket_fd(&self, service_name: &str, fd: i32) {
        let service = match self.units_get(service_name) {
            None => return,
            Some(v) => v,
        };
        service.release_socket_fd(fd);
    }

    /// setup existing mount
    fn setup_existing_mount(
        &self,
        unit_name: &str,
        what: &str,
        mount_where: &str,
        options: &str,
        fstype: &str,
    ) {
        let mount = match self.units_get(unit_name) {
            None => {
                log::error!("Failed to get unit: {}", unit_name);
                return;
            }
            Some(v) => v,
        };
        mount.setup_existing_mount(what, mount_where, options, fstype);
        mount.set_load_state(UnitLoadState::Loaded);
    }

    /// setup new mount
    fn setup_new_mount(
        &self,
        unit_name: &str,
        what: &str,
        mount_where: &str,
        options: &str,
        fstype: &str,
    ) {
        let mount = match self.load_unitx(unit_name) {
            None => {
                log::error!("Failed to load a new mount {}", unit_name);
                return;
            }
            Some(v) => {
                log::info!("Created a new mount {}", unit_name);
                v
            }
        };
        mount.setup_new_mount(what, mount_where, options, fstype);
        mount.set_load_state(UnitLoadState::Loaded);
    }

    /// update mount state
    fn update_mount_state_by_mountinfo(&self, unit_name: &str) {
        let mount = match self.units_get(unit_name) {
            None => {
                log::error!("Failed to update the mount state of {}", unit_name);
                return;
            }
            Some(v) => v,
        };
        mount.update_mount_state_by_mountinfo();
    }

    fn trigger_notify(&self, name: &str) {
        let deps = self.db.dep_gets(name, UnitRelations::UnitTriggeredBy);
        for dep in deps.iter() {
            dep.unit_trigger_notify()
        }
    }
}

/// the declaration "pub(self)" is for identification only.
impl UnitManager {
    /// delete the pid from the db
    fn child_unwatch_pid(&self, id: &str, pid: Pid) {
        self.db.child_unwatch_pid(id, pid)
    }

    ///
    #[allow(unused)]
    pub(crate) fn get_state(&self) -> State {
        *((*self.state).borrow())
    }

    ///
    pub(crate) fn set_state(&self, state: State) {
        *self.state.borrow_mut() = state;
    }

    ///
    pub fn units_get(&self, name: &str) -> Option<Rc<Unit>> {
        if !unit_name_is_valid(name, UnitNameFlags::PLAIN | UnitNameFlags::INSTANCE) {
            return None;
        }
        self.db.units_get(name).map(|uxr| uxr.unit())
    }

    ///
    pub fn unit_emergency_action(&self, action: UnitEmergencyAction, reason: String) {
        if action == UnitEmergencyAction::None {
            return;
        }
        if matches!(
            action,
            UnitEmergencyAction::Reboot | UnitEmergencyAction::Poweroff | UnitEmergencyAction::Exit
        ) {
            if let Some(shutdown_target) = self.units_get("shutdown.target") {
                if shutdown_target
                    .current_active_state()
                    .is_active_or_activating()
                {
                    return;
                }
                let shutdown_target_unitx = Rc::new(UnitX::from_unit(shutdown_target));
                if self.jm.has_start_like_job(&shutdown_target_unitx) {
                    return;
                }
            }
        }
        match action {
            UnitEmergencyAction::Reboot => {
                log::info!("Rebooting by starting reboot.target caused by {}", reason);
                if self.unit_start_by_job("reboot.target").is_err() {
                    log::error!("Failed to start reboot.target.");
                }
            }
            UnitEmergencyAction::RebootForce => {
                log::info!("Rebooting forcely caused by {}", reason);
                self.set_state(State::Reboot);
            }
            UnitEmergencyAction::RebootImmediate => {
                log::info!("Rebooting immediately caused by {}", reason);
                nix::unistd::sync();
                if nix::sys::reboot::reboot(nix::sys::reboot::RebootMode::RB_AUTOBOOT).is_err() {
                    log::error!("Failed to reboot immediately.");
                }
            }
            UnitEmergencyAction::Poweroff => {
                log::info!(
                    "Poweroffing by starting poweroff.target caused by {}",
                    reason
                );
                if self.unit_start_by_job("poweroff.target").is_err() {
                    log::error!("Failed to start poweroff.target.");
                }
            }
            UnitEmergencyAction::PoweroffForce => {
                log::info!("Poweroffing forcely caused by {}", reason);
                self.set_state(State::PowerOff);
            }
            UnitEmergencyAction::PoweroffImmediate => {
                log::info!("Poweroffing immediately caused by {}", reason);
                nix::unistd::sync();
                if nix::sys::reboot::reboot(nix::sys::reboot::RebootMode::RB_POWER_OFF).is_err() {
                    log::error!("Failed to poweroff immediately.");
                }
            }
            UnitEmergencyAction::Exit => {
                log::info!("Exiting by starting exit.target caused by {}", reason);
                if self.unit_start_by_job("exit.target").is_err() {
                    log::error!("Failed to start exit.target.");
                }
            }
            UnitEmergencyAction::ExitForce => {
                log::info!("Exiting forcely caused by {}", reason);
                self.set_state(State::Exit);
            }
            _ => {}
        }
    }

    fn start_unit(&self, name: &str, is_manual: bool, job_mode_str: &str) -> Result<()> {
        let unit = match self.load_unitx(name) {
            None => {
                return Err(Error::UnitActionENoent);
            }
            Some(v) => v,
        };
        if is_manual
            && unit
                .get_config()
                .config_data()
                .borrow()
                .Unit
                .RefuseManualStart
        {
            return Err(Error::UnitActionERefuseManualStart);
        }
        let job_mode = match JobMode::from_str(job_mode_str) {
            Err(e) => {
                log::info!("Failed to parse job mode: {}, assuming JobMode::Replace", e);
                JobMode::Replace
            }
            Ok(v) => v,
        };
        self.jm.exec(
            &JobConf::new(&unit, JobKind::Start),
            job_mode,
            &mut JobAffect::new(false),
        )?;
        log::debug!("job exec success");
        Ok(())
    }

    ///
    pub fn reliability(&self) -> Rc<Reliability> {
        Rc::clone(&self.reli)
    }

    #[allow(dead_code)]
    pub(crate) fn get_unit_by_pid(&self, pid: Pid) -> Option<Rc<UnitX>> {
        self.db.get_unit_by_pid(pid)
    }

    fn stop_unit(&self, name: &str, is_manual: bool) -> Result<()> {
        let unit = match self.load_unitx(name) {
            None => {
                return Err(Error::UnitActionENoent);
            }
            Some(v) => v,
        };

        if is_manual
            && matches!(
                unit.load_state(),
                UnitLoadState::NotFound | UnitLoadState::Error | UnitLoadState::BadSetting
            )
            && unit.active_state() != UnitActiveState::Active
        {
            return Err(Error::Other {
                msg: format!("unit {} Not Found", unit.id()),
            });
        }

        if is_manual
            && unit
                .get_config()
                .config_data()
                .borrow()
                .Unit
                .RefuseManualStop
        {
            return Err(Error::UnitActionERefuseManualStop);
        }
        self.jm.exec(
            &JobConf::new(&unit, JobKind::Stop),
            JobMode::Replace,
            &mut JobAffect::new(false),
        )?;
        Ok(())
    }

    pub(self) fn reload(&self, name: &str) -> Result<()> {
        if let Some(unit) = self.load_unitx(name) {
            self.jm.exec(
                &JobConf::new(&unit, JobKind::Reload),
                JobMode::Replace,
                &mut JobAffect::new(false),
            )?;
            Ok(())
        } else {
            Err(Error::Internal)
        }
    }

    pub(self) fn reset_failed(&self, name: &str) -> Result<()> {
        if let Some(unit) = self.units_get(name) {
            unit.reset_failed();
            Ok(())
        } else {
            Err(Error::LoadError {
                msg: format!("Failed to load {}", name),
            })
        }
    }

    pub(self) fn start_transient_unit(
        &self,
        job_mode_str: &str,
        unit_config: &UnitConfig,
        aux_units: &[UnitConfig],
    ) -> Result<()> {
        let job_mode = JobMode::from_str(job_mode_str);
        if let Err(e) = job_mode {
            log::info!("Failed to parse job mode{}, err: {}", job_mode_str, e);
            return Err(Error::InvalidData);
        }

        let unit = self
            .bus
            .transient_unit_from_message(&unit_config.unit_properties, &unit_config.unit_name)
            .map_err(|e| {
                log::info!("Failed to get transient unit with err: {}", e);
                e
            })?;

        for unit_config in aux_units {
            self.bus
                .transient_unit_from_message(&unit_config.unit_properties, &unit_config.unit_name)
                .map_err(|e| {
                    log::info!("Failed to get transient aux unit with err: {}", e);
                    e
                })?;
        }

        self.jm.exec(
            &JobConf::new(&unit, JobKind::Start),
            job_mode.unwrap(),
            &mut JobAffect::new(false),
        )?;

        Ok(())
    }

    fn get_unit_cgroup_path(&self, unit: Rc<Unit>) -> String {
        let res = match unit.cg_path().to_str() {
            Some(res) => res.to_string(),
            None => String::new(),
        };
        if res.is_empty() {
            return "Empty cgroup path".to_string();
        }
        res
    }

    fn get_unit_status_pids(&self, unit: Rc<Unit>) -> String {
        let pids = unit.get_pids();
        if pids.is_empty() {
            return "No process".to_string();
        }
        let mut res = String::new();
        for pid in pids.iter() {
            if !res.is_empty() {
                res += "\n";
            }
            res += &pid.to_string();
            res += " ";
            res += &basic::cmdline::Cmdline::new(*pid).get_cmdline();
        }
        res
    }

    pub(self) fn get_unit_status(&self, name: &str) -> Result<UnitStatus> {
        let unit = match self.units_get(name) {
            Some(unit) => {
                if unit.load_state() == UnitLoadState::NotFound {
                    return Err(Error::NotExisted);
                }
                unit
            }
            None => {
                return Err(Error::NotExisted);
            }
        };
        let error_code = match self.current_active_state(name) {
            // systemd will return 3 if the unit's state is failed or inactive.
            UnitActiveState::Failed | UnitActiveState::InActive => 3,
            _ => 0,
        };
        Ok(UnitStatus::new(
            name.to_string(),
            unit.get_description(),
            unit.get_documentation(),
            self.load_unit_success(name).to_string(),
            self.get_subunit_state(name),
            self.current_active_state(name).to_string(),
            self.get_unit_cgroup_path(unit.clone()),
            self.get_unit_status_pids(unit.clone()),
            error_code,
        ))
    }

    pub(self) fn get_all_units(&self) -> Result<String> {
        let mut list_units_table = ShowTable::new();
        list_units_table.add_line(vec!["UNIT", "LOAD", "ACTIVE", "SUB", "DESCRIPTION"]);
        for unit_type in UnitType::iterator() {
            list_units_table.set_current_row_underline(true);
            for unit_name in self.units_get_all(Some(unit_type)) {
                let unit = match self.units_get(&unit_name) {
                    Some(unit) => unit,
                    None => {
                        log::info!("Failed to get unit: {}", unit_name);
                        continue;
                    }
                };
                let load_state = self.load_unit_success(&unit_name).to_string();
                let active_state = self.current_active_state(&unit_name).to_string();
                let sub_state = self.get_subunit_state(&unit_name);
                let description = match unit.get_description() {
                    None => String::from(&unit_name),
                    Some(str) => str,
                };
                list_units_table.add_line(vec![
                    &unit_name,
                    &load_state,
                    &active_state,
                    &sub_state,
                    &description,
                ]);
                if active_state == "failed" {
                    list_units_table.set_current_row_color(CellColor::Red);
                }
            }
        }
        Ok(list_units_table.to_string())
    }

    pub(self) fn new(
        eventr: &Rc<Events>,
        relir: &Rc<Reliability>,
        dmr: &Rc<DataManager>,
        lookup_path: &Rc<LookupPaths>,
        state: Rc<RefCell<State>>,
        manager_config: Rc<RefCell<ManagerConfig>>,
    ) -> Rc<UnitManager> {
        let log_target = manager_config.borrow().LogTarget.clone();
        let log_file_size = manager_config.borrow().LogFileSize;
        let log_file_num = manager_config.borrow().LogFileNumber;

        let _rentry = Rc::new(UnitRe::new(relir));
        let _db = Rc::new(UnitDb::new(&_rentry));
        let _rt = Rc::new(UnitRT::new(relir, &_rentry, &_db));
        let _load = Rc::new(UnitLoad::new(dmr, &_rentry, &_db, &_rt, lookup_path));
        let _jm = Rc::new(JobManager::new(eventr, relir, &_db, dmr));
        let _sms = Rc::new(UnitSubManagers::new(
            relir,
            &log_target,
            log_file_size,
            log_file_num,
        ));
        let um = Rc::new(UnitManager {
            events: Rc::clone(eventr),
            reli: Rc::clone(relir),
            state,
            rentry: Rc::clone(&_rentry),
            db: Rc::clone(&_db),
            rt: Rc::clone(&_rt),
            load: Rc::clone(&_load),
            jm: Rc::clone(&_jm),
            exec: ExecSpawn::new(),
            sigchld: Sigchld::new(eventr, relir, &_db, &_jm),
            notify: NotifyManager::new(eventr, relir, &_rentry, &_db, &_jm),
            sms: Rc::clone(&_sms),
            bus: UnitBus::new(relir, &_load, &_jm, &_sms),
        });
        um.load.set_um(&um);
        let umif = Rc::clone(&um);
        um.sms.set_um(umif);
        um
    }

    fn load_unitx(&self, name: &str) -> Option<Rc<UnitX>> {
        self.load.load_unit(name)
    }

    fn submit_to_stop_when_bound_queue(&self, unit: Rc<UnitX>) {
        self.rt.submit_to_stop_when_bound_queue(unit);
    }
}

// inert states need jm,so put here
impl TableSubscribe<String, UnitState> for UnitManager {
    fn notify(&self, op: &TableOp<String, UnitState>) {
        match op {
            TableOp::TableInsert(name, config) => self.insert_states(name, config),
            TableOp::TableRemove(name, _) => self.remove_states(name),
        }
    }
}

// insert start_limit_hit
impl TableSubscribe<String, StartLimitResult> for UnitManager {
    fn notify(&self, op: &TableOp<String, StartLimitResult>) {
        match op {
            TableOp::TableInsert(name, config) => self.insert_start_limit_res(name, config),
            TableOp::TableRemove(name, _) => self.remove_start_limit_res(name),
        }
    }
}

impl TableSubscribe<String, JobResult> for UnitManager {
    fn notify(&self, op: &TableOp<String, JobResult>) {
        match op {
            TableOp::TableInsert(name, config) => self.instert_job_result(name, config),
            TableOp::TableRemove(name, _) => self.remove_job_result(name),
        }
    }
}

impl UnitManager {
    fn insert_states(&self, source: &str, state: &UnitState) {
        log::debug!("insert unit states source {}, state: {:?}", source, state);
        let unitx = if let Some(u) = self.db.units_get(source) {
            u
        } else {
            return;
        };

        if state.os != UnitActiveState::Failed && state.ns == UnitActiveState::Failed {
            self.unit_emergency_action(
                unitx.get_failure_action(),
                "unit ".to_string() + source + " failed",
            );
        }
        if !state.os.is_inactive_or_failed() && state.ns == UnitActiveState::InActive {
            self.unit_emergency_action(
                unitx.get_success_action(),
                "unit ".to_string() + source + " succeeded",
            );
        }

        if let Err(_e) = self.jm.try_finish(&unitx, state.os, state.ns, state.flags) {
            // debug
        }
        let atom = UnitRelationAtom::UnitAtomTriggeredBy;
        for other in self.db.dep_gets_atom(&unitx, atom) {
            other.trigger(&unitx);
        }

        /* Trigger BindsTo unit state change */
        if state.ns.is_inactive_or_failed() { /* submit to stop_when_bound_queue */ }

        if state.ns.is_active_or_reloading() {
            /* submit to stop_when_bound_queue */
            self.submit_to_stop_when_bound_queue(unitx);
        }
    }

    fn remove_states(&self, _source: &str) {
        todo!();
    }

    fn insert_start_limit_res(&self, source: &str, start_limit_res: &StartLimitResult) {
        if start_limit_res == &StartLimitResult::StartLimitNotHit {
            return;
        }
        let unitx = if let Some(u) = self.db.units_get(source) {
            u
        } else {
            return;
        };
        let reason = "unit ".to_string() + source + " hit StartLimit";
        self.unit_emergency_action(unitx.get_start_limit_action(), reason)
    }

    fn remove_start_limit_res(&self, _source: &str) {}

    fn instert_job_result(&self, source: &str, job_result: &JobResult) {
        if job_result != &JobResult::TimeOut {
            return;
        }
        let unitx = if let Some(u) = self.db.units_get(source) {
            u
        } else {
            return;
        };
        let reason = "the job of unit ".to_string() + source + " timedout";
        self.unit_emergency_action(unitx.get_job_timeout_action(), reason)
    }

    fn remove_job_result(&self, _source: &str) {}

    fn make_unit_consistent(&self, lunit: Option<&String>) {
        if lunit.is_none() {
            return;
        }

        if let Some(unit) = self.db.units_get(lunit.unwrap()) {
            unit.remove_transient();
        }
    }
}

impl ReStation for UnitManager {
    // input
    fn input_rebuild(&self) {
        // sigchld
        self.sigchld.input_rebuild();

        // sub-manager
        self.sms.input_rebuild();
    }

    // compensate
    fn db_compensate_last(&self, lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        let (frame, _, _) = lframe;
        if let Ok(f) = ReliLastFrame::try_from(frame) {
            match f {
                ReliLastFrame::Queue => self.rt.db_compensate_last(lframe, lunit),
                ReliLastFrame::JobManager => self.jm.db_compensate_last(lframe, lunit),
                ReliLastFrame::SigChld => self.sigchld.db_compensate_last(lframe, lunit),
                ReliLastFrame::CgEvent => todo!(),
                ReliLastFrame::Notify => self.notify.db_compensate_last(lframe, lunit),
                ReliLastFrame::SubManager => self.sms.db_compensate_last(lframe, lunit),
                _ => {} // not concerned, do nothing
            };
        }
    }

    fn db_compensate_history(&self) {
        // queue: do nothing

        // job
        self.jm.db_compensate_history();

        // sig-child: do nothing

        // cg-event: do nothing

        // notify: do nothing
    }

    fn do_compensate_last(&self, lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        let (frame, _, _) = lframe;
        if let Ok(f) = ReliLastFrame::try_from(frame) {
            match f {
                ReliLastFrame::Queue => self.rt.do_compensate_last(lframe, lunit),
                ReliLastFrame::JobManager => self.jm.do_compensate_last(lframe, lunit),
                ReliLastFrame::SigChld => self.sigchld.do_compensate_last(lframe, lunit),
                ReliLastFrame::CgEvent => todo!(),
                ReliLastFrame::Notify => self.notify.do_compensate_last(lframe, lunit),
                ReliLastFrame::SubManager => self.sms.do_compensate_last(lframe, lunit),
                ReliLastFrame::CmdOp => self.make_unit_consistent(lunit),
                _ => {} // not concerned, do nothing
            };
        }
    }

    fn do_compensate_others(&self, lunit: Option<&String>) {
        // queue: do nothing

        // job
        self.jm.do_compensate_others(lunit);

        // sig-child: do nothing

        // cg-event: do nothing

        // notify: do nothing
    }

    // data
    fn db_map(&self, reload: bool) {
        // unit_datastore(with unit_entry)
        /* unit-sets with unit_entry */
        for unit_id in self.rentry.base_keys().iter() {
            if reload {
                let unit = self.load.load_unit(unit_id).unwrap();
                unit.db_map(reload);
            } else {
                let unit = self.load.try_new_unit(unit_id).unwrap();
                unit.db_map(reload);
                self.db.units_insert(unit_id.to_string(), unit);
            }
        }
        /* others: unit-dep and unit-child */
        self.db.db_map_excl_units(reload);

        // rt
        self.rt.db_map(reload);

        // job
        self.jm.db_map(reload);

        // notify
        self.notify.db_map(reload);

        // sub-manager
        self.sms.db_map(reload);
    }

    // data
    fn db_insert(&self) {
        for unit in self.db.units_get_all(None).iter() {
            unit.db_insert();
        }

        /* others: unit-dep and unit-child */
        self.db.db_insert_excl_units();

        // rt
        self.rt.db_insert();

        // job
        self.jm.db_insert();

        // notify
        self.notify.db_insert();

        // sub-manager
        self.sms.db_insert();
    }

    // reload
    fn register_ex(&self) {
        // notify
        self.notify.register_ex();

        // sub-manager
        self.sms.enumerate();
    }

    fn entry_coldplug(&self) {
        for unit in self.db.units_get_all(None).iter() {
            // unit
            unit.entry_coldplug();

            // job
            self.jm.coldplug_unit(unit);
        }
    }

    fn entry_clear(&self) {
        // job
        self.jm.entry_clear();

        // rt
        self.rt.entry_clear();

        // db
        self.db.entry_clear();
    }
}

/// the trait used for translate to UnitObj
/*pub trait UnitSubClass: SubUnit + UnitMngUtil {
    /// the method of translate to UnitObj
    fn into_unitobj(self: Box<Self>) -> Box<dyn SubUnit>;
}*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::{rentry::RELI_HISTORY_MAX_DBS, Mode};
    use crate::mount::setup;
    use core::rel::ReliConf;
    use core::unit::UnitActiveState;
    use event::Events;
    use nix::sys::wait::WaitStatus;
    use std::thread;
    use std::time::Duration;

    fn init_dm_for_test() -> (Rc<DataManager>, Rc<Events>, Rc<UnitManager>) {
        log::init_log_to_console("init_dm_for_test", log::Level::Trace);
        let mut l_path = LookupPaths::new();
        let test_units_dir = libtests::get_project_root()
            .unwrap()
            .join("tests/test_units/")
            .to_string_lossy()
            .to_string();
        l_path.search_path.push(test_units_dir);
        let lookup_path = Rc::new(l_path);

        let event = Rc::new(Events::new().unwrap());
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(
            ReliConf::new().set_max_dbs(RELI_HISTORY_MAX_DBS),
        ));
        let state = Rc::new(RefCell::new(State::Init));
        let um = UnitManager::new(
            &event,
            &reli,
            &dm,
            &lookup_path,
            state,
            Rc::new(RefCell::new(ManagerConfig::new(&Mode::System))),
        );
        (dm, event, um)
    }

    #[allow(dead_code)]
    fn setup_mount_point() -> Result<()> {
        setup::mount_setup()
    }

    #[test]
    fn test_service_unit_load() {
        let dm = init_dm_for_test();
        let unit_name = String::from("config.service");
        let unit = dm.2.load_unitx(&unit_name);

        match unit {
            Some(_unit_obj) => assert_eq!(_unit_obj.id(), "config.service"),
            None => println!("test unit load, not found unit: {}", unit_name),
        };
    }

    // #[test]
    #[allow(dead_code)]
    fn test_service_unit_start() {
        let ret = setup_mount_point();
        if ret.is_err() {
            return;
        }

        let dm = init_dm_for_test();
        let unit_name = String::from("config.service");
        let unit = dm.2.load_unitx(&unit_name);

        assert!(unit.is_some());
        let u = unit.unwrap();

        let ret = u.start();
        assert!(ret.is_ok());

        log::debug!("unit start end!");
        let ret = u.stop(false);
        assert!(ret.is_ok());
        log::debug!("unit stop end!");
    }

    // #[test]
    #[allow(dead_code)]
    fn test_socket_unit_start_and_stop() {
        let ret = setup_mount_point();
        if ret.is_err() {
            return;
        }

        let dm = init_dm_for_test();

        let unit_name = String::from("test.socket");
        let unit = dm.2.load_unitx(&unit_name);

        assert!(unit.is_some());
        let u = unit.unwrap();

        let ret = u.start();
        log::debug!("socket start ret is: {:?}", ret);
        assert!(ret.is_ok());

        thread::sleep(Duration::from_secs(4));
        let wait_status = WaitStatus::Exited(Pid::from_raw(-1), 0);
        u.sigchld_events(wait_status);
        assert_eq!(u.active_state(), UnitActiveState::Active);

        let ret = u.stop(false);
        log::debug!("socket stop ret is: {:?}", ret);
        assert!(ret.is_ok());

        thread::sleep(Duration::from_secs(4));
        assert_eq!(u.active_state(), UnitActiveState::DeActivating);
        u.sigchld_events(wait_status);

        assert_eq!(u.active_state(), UnitActiveState::InActive);
    }

    #[test]
    fn test_service_unit_start_conflicts() {
        let dm = init_dm_for_test();
        let conflict_unit_name = String::from("conflict.service");
        let confilict_unit = dm.2.start_unit(&conflict_unit_name, false, "replace");

        assert!(confilict_unit.is_ok());
    }

    #[test]
    fn test_units_load() {
        let dm = init_dm_for_test();
        let mut unit_name_lists: Vec<String> = Vec::new();

        unit_name_lists.push("config.service".to_string());
        // unit_name_lists.push("testsunit.target".to_string());
        for u_name in unit_name_lists.iter() {
            let unit = dm.2.load_unitx(u_name);

            match unit {
                Some(_unit_obj) => assert_eq!(&_unit_obj.id(), u_name),
                None => println!("test unit load, not found unit: {}", u_name),
            };
        }
    }
    #[test]
    fn test_target_unit_load() {
        let dm = init_dm_for_test();
        let mut unit_name_lists: Vec<String> = Vec::new();

        unit_name_lists.push("testsunit.target".to_string());
        // unit_name_lists.push("testsunit.target".to_string());
        for u_name in unit_name_lists.iter() {
            let unit = dm.2.load_unitx(u_name);
            match unit {
                Some(_unit_obj) => {
                    println!(
                        "{:?}",
                        (*_unit_obj.get_config().config_data())
                            .borrow()
                            .Unit
                            .Requires
                    );
                    assert_eq!(&_unit_obj.id(), u_name);
                }
                None => println!("test unit load, not found unit: {}", u_name),
            };
        }
    }
}
