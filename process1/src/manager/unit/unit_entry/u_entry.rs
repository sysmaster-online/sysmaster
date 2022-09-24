use super::uu_cgroup::UeCgroup;
use super::uu_child::UeChild;
use super::uu_condition::{
    UeCondition, ASSERT_PATH_EXISTS, CONDITION_FILE_NOT_EMPTY, CONDITION_NEEDS_UPDATE,
    CONDITION_PATH_EXISTS,
};
use super::uu_config::UeConfig;
use super::uu_load::UeLoad;
use crate::manager::data::{DataManager, UnitActiveState, UnitDepConf, UnitState};
use crate::manager::unit::uload_util::UnitFile;
use crate::manager::unit::unit_base::{KillOperation, UnitActionError, UnitLoadState, UnitType};
use crate::manager::{UnitNotifyFlags, UnitRelations};
use cgroup::{self, CgFlags};
use log;
use nix::sys::signal::Signal;
use nix::sys::socket::UnixCredentials;
use nix::unistd::Pid;
use nix::NixPath;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::rc::Rc;
use utils::Result;

use utils::error::Error as ServiceError;

pub struct UnitRef {
    source: Option<String>,
    target: Option<String>,
}

impl UnitRef {
    pub fn new() -> Self {
        UnitRef {
            source: None,
            target: None,
        }
    }

    pub fn set_ref(&mut self, source: String, target: String) {
        self.source = Some(source);
        self.target = Some(target);
    }

    pub fn unset_ref(&mut self) {
        self.source = None;
        self.target = None;
    }

    pub fn target(&self) -> Option<&String> {
        self.target.as_ref()
    }
}

pub struct Unit {
    // associated objects
    dm: Rc<DataManager>,

    // owned objects
    unit_type: UnitType,
    id: String,

    config: Rc<UeConfig>,
    load: UeLoad,
    child: UeChild,
    cgroup: UeCgroup,
    conditions: Rc<UeCondition>,
    sub: Box<dyn UnitObj>,
}

impl PartialEq for Unit {
    fn eq(&self, other: &Self) -> bool {
        self.unit_type == other.unit_type && self.id == other.id
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
        self.id.cmp(&other.id)
    }
}

impl Hash for Unit {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
///The trait Defining Shared Behavior of sub unit
///
/// difference sub unit ref by dynamic trait
///
pub trait UnitObj {
    fn init(&self) {}
    fn done(&self) {}
    fn load(&self, conf: &Vec<PathBuf>) -> Result<(), Box<dyn Error>>;

    fn coldplug(&self) {}
    fn dump(&self) {}

    /// Start a Unit
    /// Each Sub Unit need to implement its own start function
    ///
    fn start(&self) -> Result<(), UnitActionError> {
        Ok(())
    }
    fn stop(&self) -> Result<(), UnitActionError> {
        Ok(())
    }
    fn reload(&self) {}

    fn kill(&self) {}
    fn release_resources(&self) {}
    fn sigchld_events(&self, _pid: Pid, _code: i32, _status: Signal) {}
    fn reset_failed(&self) {}
    fn collect_fds(&self) -> Vec<i32> {
        Vec::new()
    }

    ///Get the the unit state
    ///
    /// Every sub unit  can define self states and map to [`UnitActiveState`]
    ///
    fn current_active_state(&self) -> UnitActiveState;
    fn attach_unit(&self, unit: Rc<Unit>);

    fn notify_message(
        &self,
        _ucred: &UnixCredentials,
        _events: &HashMap<&str, &str>,
        _fds: &Vec<i32>,
    ) -> Result<(), ServiceError> {
        Ok(())
    }
}

impl Unit {
    pub(super) fn new(
        unit_type: UnitType,
        name: &str,
        dmr: &Rc<DataManager>,
        filer: &Rc<UnitFile>,
        sub: Box<dyn UnitObj>,
    ) -> Rc<Self> {
        let _config = Rc::new(UeConfig::new());
        let _u = Rc::new(Unit {
            dm: Rc::clone(dmr),
            unit_type,
            id: String::from(name),
            config: Rc::clone(&_config),
            load: UeLoad::new(dmr, filer, &_config, String::from(name)),
            child: UeChild::new(),
            cgroup: UeCgroup::new(),
            conditions: Rc::new(UeCondition::new()),
            sub,
        });
        _u.sub.attach_unit(Rc::clone(&_u));
        _u
    }

    fn conditions(&self) -> Rc<UeCondition> {
        let flag = self.conditions.init_flag();
        if flag != 0 {
            return Rc::clone(&self.conditions);
        } else {
            //need to reconstruct the code, expose the config detail out is wrong
            let add_condition = |condop: &str, _params: &str| {
                if _params.is_empty() {
                    return;
                }
                self.conditions.add_condition(condop, String::from(_params));
            };

            let add_assert = |assert_op: &str, _params: &str| {
                if _params.is_empty() {
                    return;
                }
                self.conditions.add_assert(assert_op, String::from(_params));
            };
            add_condition(
                CONDITION_FILE_NOT_EMPTY,
                self.get_config()
                    .config_data()
                    .borrow()
                    .Unit
                    .ConditionFileNotEmpty
                    .as_str(),
            );

            add_condition(
                CONDITION_NEEDS_UPDATE,
                self.get_config()
                    .config_data()
                    .borrow()
                    .Unit
                    .ConditionNeedsUpdate
                    .as_str(),
            );

            add_condition(
                CONDITION_PATH_EXISTS,
                self.get_config()
                    .config_data()
                    .borrow()
                    .Unit
                    .ConditionPathExists
                    .as_str(),
            );

            add_assert(
                ASSERT_PATH_EXISTS,
                self.get_config()
                    .config_data()
                    .borrow()
                    .Unit
                    .AssertPathExists
                    .as_str(),
            );
        }
        Rc::clone(&self.conditions)
    }

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
        let u_state = UnitState::new(original_state, new_state, flags);
        self.dm.insert_unit_state(self.id.clone(), u_state);
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn prepare_exec(&self) -> Result<()> {
        log::debug!("prepare exec cgroup");
        self.cgroup.setup_cg_path(&self.id);

        self.cgroup.prepare_cg_exec()
    }

    pub fn cg_path(&self) -> PathBuf {
        self.cgroup.cg_path()
    }

    pub fn kill_context(
        &self,
        m_pid: Option<Pid>,
        c_pid: Option<Pid>,
        ko: KillOperation,
    ) -> Result<(), Box<dyn Error>> {
        let sig = ko.to_signal();
        if m_pid.is_some() {
            match nix::sys::signal::kill(m_pid.unwrap(), sig) {
                Ok(_) => {
                    if sig != Signal::SIGCONT && sig != Signal::SIGKILL {
                        match nix::sys::signal::kill(m_pid.unwrap(), Signal::SIGCONT) {
                            Ok(_) => {}
                            Err(e) => {
                                log::debug!("kill pid {} errno: {}", m_pid.unwrap(), e)
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to kill main service: error: {}", e);
                }
            }
        }
        if c_pid.is_some() {
            match nix::sys::signal::kill(c_pid.unwrap(), sig) {
                Ok(_) => {
                    if sig != Signal::SIGCONT && sig != Signal::SIGKILL {
                        match nix::sys::signal::kill(c_pid.unwrap(), Signal::SIGCONT) {
                            Ok(_) => {}
                            Err(e) => {
                                log::debug!("kill pid {} errno: {}", c_pid.unwrap(), e)
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to kill control service: error: {}", e);
                }
            }
        }

        if !self.cgroup.cg_path().is_empty() {
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

        Ok(())
    }

    pub fn default_dependencies(&self) -> bool {
        self.get_config()
            .config_data()
            .borrow()
            .Unit
            .DefaultDependencies
    }

    pub fn ignore_on_isolate(&self) -> bool {
        self.get_config()
            .config_data()
            .borrow()
            .Unit
            .IgnoreOnIsolate
    }

    pub fn set_ignore_on_isolate(&self, ignore_on_isolate: bool) {
        self.get_config()
            .config_data()
            .borrow_mut()
            .Unit
            .IgnoreOnIsolate = ignore_on_isolate;
    }

    fn pids_set(&self, m_pid: Option<Pid>, c_pid: Option<Pid>) -> HashSet<Pid> {
        let mut pids = HashSet::new();

        if m_pid.is_some() {
            pids.insert(m_pid.unwrap());
        }

        if c_pid.is_some() {
            pids.insert(c_pid.unwrap());
        }

        pids
    }

    pub fn insert_two_deps(
        &self,
        ra: UnitRelations,
        rb: UnitRelations,
        u_name: String,
    ) -> Option<UnitDepConf> {
        log::debug!(
            "insert two relations {:?} and {:?} to unit {}",
            ra,
            rb,
            u_name.to_string()
        );
        let mut ud_conf = UnitDepConf::new();

        for rl in [ra, rb] {
            ud_conf.deps.insert(rl, vec![u_name.clone()]);
        }

        self.dm.insert_ud_config(self.get_id().to_string(), ud_conf)
    }

    pub fn insert_dep(&self, ra: UnitRelations, u_name: String) -> Option<UnitDepConf> {
        log::debug!("insert relation {:?} to unit {}", ra, u_name.to_string());
        let mut ud_conf = UnitDepConf::new();
        ud_conf.deps.insert(ra, vec![u_name.clone()]);

        self.dm.insert_ud_config(self.get_id().to_string(), ud_conf)
    }

    pub(super) fn get_config(&self) -> Rc<UeConfig> {
        self.config.clone()
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

    pub(super) fn load_unit(&self) -> Result<(), Box<dyn Error>> {
        self.set_in_load_queue(false);
        // Mount unit doesn't have config file, set its loadstate to
        // UnitLoaded directly.
        if self.unit_type() == UnitType::UnitMount {
            self.load.set_load_state(UnitLoadState::UnitLoaded);
            return Ok(());
        }
        match self.load.load_unit_confs() {
            Ok(_) => Ok({
                let paths = self.load.get_unit_id_fragment_pathbuf();
                log::debug!("begin exec sub class load");
                let ret = self.sub.load(&paths);

                if let Err(e) = ret {
                    return Err(format!("load Unit {} failed, error: {}", self.id, e).into());
                }

                self.load.set_load_state(UnitLoadState::UnitLoaded);
            }),
            Err(e) => {
                self.load.set_load_state(UnitLoadState::UnitNotFound);
                return Err(e);
            }
        }
    }

    pub(super) fn current_active_state(&self) -> UnitActiveState {
        self.sub.current_active_state()
    }

    pub(super) fn start(&self) -> Result<(), UnitActionError> {
        let active_state = self.current_active_state();
        let us_is_active_or_reloading = match active_state {
            UnitActiveState::UnitActive | UnitActiveState::UnitReloading => true,
            _ => false,
        };

        if us_is_active_or_reloading {
            return Err(UnitActionError::UnitActionEAlready);
        }

        if active_state == UnitActiveState::UnitMaintenance {
            return Err(UnitActionError::UnitActionEAgain);
        }

        if self.load_state() != UnitLoadState::UnitLoaded {
            return Err(UnitActionError::UnitActionEInval);
        }
        if active_state != UnitActiveState::UnitActivating && !self.conditions().conditions_test() {
            log::debug!("Starting failed because condition test failed");
            return Err(UnitActionError::UnitActionEInval);
        }
        if active_state != UnitActiveState::UnitActivating && !self.conditions().asserts_test() {
            log::error!("Starting failed because assert test failed");
            return Err(UnitActionError::UnitActionEInval);
        }

        self.sub.start()
    }

    pub(super) fn stop(&self) -> Result<(), UnitActionError> {
        let active_state = self.current_active_state();
        let inactive_or_failed = match active_state {
            UnitActiveState::UnitInActive | UnitActiveState::UnitFailed => true,
            _ => false,
        };

        if inactive_or_failed {
            return Err(UnitActionError::UnitActionEAlready);
        }

        self.sub.stop()
    }

    pub(super) fn sigchld_events(&self, pid: Pid, code: i32, signal: Signal) {
        self.sub.sigchld_events(pid, code, signal)
    }

    pub(super) fn load_state(&self) -> UnitLoadState {
        self.load.load_state()
    }

    pub(super) fn unit_type(&self) -> UnitType {
        self.unit_type
    }

    pub(super) fn collect_fds(&self) -> Vec<i32> {
        self.sub.collect_fds()
    }

    pub(in crate::manager) fn notify_message(
        &self,
        ucred: &UnixCredentials,
        messages: &HashMap<&str, &str>,
        fds: &Vec<i32>,
    ) -> Result<(), ServiceError> {
        self.sub.notify_message(ucred, messages, fds)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use utils::logger;

    use crate::{
        manager::{data::DataManager, unit::uload_util::UnitFile, UnitType},
        plugin::Plugin,
    };

    use super::Unit;

    fn unit_init() -> Rc<Unit> {
        logger::init_log_with_console("test_unit_entry", 4);
        let unit_file = UnitFile::new();
        let dm = DataManager::new();
        let plugin = Plugin::get_instance();
        let sub_obj = plugin.create_unit_obj(UnitType::UnitService);
        let unit = Unit::new(
            UnitType::UnitService,
            "config.service",
            &Rc::new(dm),
            &Rc::new(unit_file),
            sub_obj.unwrap().into_unitobj(),
        );
        unit
    }

    #[test]
    fn test_unit_load() {
        let _unit = unit_init();
        let load_stat = _unit.load_unit();
        assert!(load_stat.is_ok());
        /*let stat = _unit.start();//sub unit依赖 UnitManager，依赖关系不太合适，后续需要优化
        assert!(stat.is_ok());
        assert_eq!(_unit.current_active_state(),UnitActiveState::UnitActive);*/
    }

    fn test_unit_condition() {
        let _unit = unit_init();
        let load_stat = _unit.load_unit();
        assert!(load_stat.is_ok());
        assert!(_unit.conditions().conditions_test());
    }
}
