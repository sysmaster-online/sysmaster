use super::uu_base::UeBase;
use super::uu_cgroup::UeCgroup;
use super::uu_child::UeChild;
use super::uu_condition::{
    UeCondition, ASSERT_PATH_EXISTS, CONDITION_FILE_NOT_EMPTY, CONDITION_NEEDS_UPDATE,
    CONDITION_PATH_EXISTS,
};
use super::uu_config::UeConfig;
use super::uu_load::UeLoad;
use super::uu_ratelimit::StartLimit;
use crate::core::unit::data::{DataManager, UnitDepConf, UnitState};
use crate::core::unit::uload_util::UnitFile;
use crate::core::unit::unit_rentry::{UnitLoadState, UnitRe};
use crate::core::unit::UnitRelations;
use libcgroup::{self, CgFlags};
use libutils::error::Error as ServiceError;
use libutils::process_util::my_child;
use libutils::Result;
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
use sysmaster::reliability::ReStation;
use sysmaster::unit::{KillContext, KillMode, KillOperation, UnitNotifyFlags};
use sysmaster::unit::{SubUnit, UnitActionError, UnitActiveState, UnitBase, UnitType};

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
        self.base.id().cmp(other.base.id())
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
    fn db_map(&self) {
        self.base.db_map();
        self.config.db_map();
        self.cgroup.db_map();
        self.load.db_map();
        self.child.db_map();

        self.sub.db_map();
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
    fn id(&self) -> &String {
        self.id()
    }

    /*fn get_dependency_list(&self, _unit_name: &str, _atom: libsysmaster::unit::UnitRelationAtom) -> Vec<Rc<Self>> {
        todo!()
    }*/

    fn test_start_limit(&self) -> bool {
        self.test_start_limit()
    }

    fn kill_context(
        &self,
        k_context: Rc<KillContext>,
        m_pid: Option<Pid>,
        c_pid: Option<Pid>,
        ko: KillOperation,
    ) -> Result<(), Box<dyn Error>> {
        self.kill_context(k_context, m_pid, c_pid, ko)
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

    fn insert_two_deps(
        &self,
        ra: UnitRelations,
        rb: UnitRelations,
        u_name: String,
    ) -> Result<(), Box<dyn Error>> {
        self.insert_two_deps(ra, rb, u_name);
        Ok(())
    }
    fn insert_dep(&self, ra: UnitRelations, u_name: String) {
        self.insert_dep(ra, u_name);
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

    fn guess_main_pid(&self) -> Result<Pid, Box<dyn Error>> {
        self.guess_main_pid()
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
        let u_state = UnitState::new(original_state, new_state, flags);
        self.dm.insert_unit_state(self.id().clone(), u_state);
    }

    ///
    pub fn id(&self) -> &String {
        self.base.id()
    }

    /// return pids of the unit
    pub fn get_pids(&self) -> Vec<Pid> {
        self.child.get_pids()
    }

    /// return description
    pub fn get_description(&self) -> Option<String> {
        self.load.get_description()
    }

    ///
    pub fn prepare_exec(&self) -> Result<()> {
        log::debug!("prepare exec cgroup");
        self.cgroup.setup_cg_path();

        self.cgroup.prepare_cg_exec()
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
    ) -> Result<(), Box<dyn Error>> {
        let sig = ko.to_signal();
        if let Some(pid) = m_pid {
            match nix::sys::signal::kill(pid, sig) {
                Ok(_) => {
                    if sig != Signal::SIGCONT && sig != Signal::SIGKILL {
                        match nix::sys::signal::kill(pid, Signal::SIGCONT) {
                            Ok(_) => {}
                            Err(e) => {
                                log::debug!("kill pid {} errno: {}", pid, e)
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to kill main service: error: {}", e);
                }
            }
        }
        if let Some(pid) = c_pid {
            match nix::sys::signal::kill(pid, sig) {
                Ok(_) => {
                    if sig != Signal::SIGCONT && sig != Signal::SIGKILL {
                        match nix::sys::signal::kill(pid, Signal::SIGCONT) {
                            Ok(_) => {}
                            Err(e) => {
                                log::debug!("kill pid {} errno: {}", pid, e)
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to kill control service: error: {}", e);
                }
            }
        }

        if !self.cgroup.cg_path().is_empty()
            && (k_context.kill_mode() == KillMode::ControlGroup
                || (k_context.kill_mode() == KillMode::Mixed && ko == KillOperation::KillKill))
        {
            let pids = self.pids_set(m_pid, c_pid);

            match libcgroup::cg_kill_recursive(
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
    pub fn guess_main_pid(&self) -> Result<Pid, Box<dyn Error>> {
        let cg_path = self.cgroup.cg_path();

        if cg_path.is_empty() {
            return Err(
                "cgroup path is empty, can not guess main pid from cgroup path"
                    .to_string()
                    .into(),
            );
        }
        let pids = libcgroup::cg_get_pids(&cg_path);
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

    ///insert unit dep conf
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
            u_name
        );
        let mut ud_conf = UnitDepConf::new();

        for rl in [ra, rb] {
            ud_conf.deps.insert(rl, vec![u_name.clone()]);
        }

        self.dm.insert_ud_config(self.id().to_string(), ud_conf)
    }

    ///
    pub fn insert_dep(&self, ra: UnitRelations, u_name: String) -> Option<UnitDepConf> {
        log::debug!("insert relation {:?} to unit {}", ra, u_name);
        let mut ud_conf = UnitDepConf::new();
        ud_conf.deps.insert(ra, vec![u_name]);

        self.dm.insert_ud_config(self.id().to_string(), ud_conf)
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
            return true;
        }

        self.start_limit.set_hit(true);
        // todo emergency action

        false
    }

    ///
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
            Ok(_) => {
                {
                    let paths = self.load.get_unit_id_fragment_pathbuf();
                    log::debug!("begin exec sub class load");
                    let ret = self.sub.load(paths);

                    if let Err(e) = ret {
                        return Err(format!("load Unit {} failed, error: {}", self.id(), e).into());
                    }

                    self.load.set_load_state(UnitLoadState::UnitLoaded);
                };
                Ok(())
            }
            Err(e) => {
                self.load.set_load_state(UnitLoadState::UnitNotFound);
                Err(e)
            }
        }
    }

    ///
    pub fn start(&self) -> Result<(), UnitActionError> {
        let active_state = self.current_active_state();
        let us_is_active_or_reloading = matches!(
            active_state,
            UnitActiveState::UnitActive | UnitActiveState::UnitReloading
        );

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

    ///
    pub fn stop(&self, force: bool) -> Result<(), UnitActionError> {
        if !force {
            let active_state = self.current_active_state();
            let inactive_or_failed = matches!(
                active_state,
                UnitActiveState::UnitInActive | UnitActiveState::UnitFailed
            );

            if inactive_or_failed {
                return Err(UnitActionError::UnitActionEAlready);
            }
        }

        self.sub.stop(force)
    }

    pub(super) fn sigchld_events(&self, pid: Pid, code: i32, signal: Signal) {
        self.sub.sigchld_events(pid, code, signal)
    }

    pub(super) fn load_state(&self) -> UnitLoadState {
        self.load.load_state()
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

    pub(in crate::core) fn notify_message(
        &self,
        ucred: &UnixCredentials,
        messages: &HashMap<&str, &str>,
        fds: Vec<i32>,
    ) -> Result<(), ServiceError> {
        self.sub.notify_message(ucred, messages, fds)
    }
}

#[cfg(test)]
mod tests {
    use super::Unit;
    use crate::core::manager::RELI_HISTORY_MAX_DBS;
    use crate::core::unit::test::test_utils::UmIfD;
    use crate::core::unit::unit_rentry::UnitRe;
    use libutils::{logger, path_lookup::LookupPaths};
    use std::rc::Rc;
    use sysmaster::reliability::Reliability;
    use sysmaster::unit::UnitType;

    use crate::core::{plugin::Plugin, unit::data::DataManager, unit::uload_util::UnitFile};
    fn unit_init() -> Rc<Unit> {
        logger::init_log_with_console("test_unit_entry", 4);
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let rentry = Rc::new(UnitRe::new(&reli));

        let mut l_path = LookupPaths::new();
        l_path.init_lookup_paths();
        let lookup_path = Rc::new(l_path);
        let unit_file = UnitFile::new(&lookup_path);

        let dm = DataManager::new();
        let plugin = Plugin::get_instance();
        let umifd = Rc::new(UmIfD);
        let sub_obj = plugin
            .create_unit_obj_with_um(UnitType::UnitService, umifd)
            .unwrap();
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
        assert_eq!(_unit.current_active_state(),UnitActiveState::UnitActive);*/
    }

    #[allow(dead_code)]
    fn test_unit_condition() {
        let _unit = unit_init();
        let load_stat = _unit.load_unit();
        assert!(load_stat.is_ok());
        assert!(_unit.conditions().conditions_test());
    }
}
