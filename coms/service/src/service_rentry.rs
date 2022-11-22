#![allow(non_snake_case)]
use confique::Config;
use libsysmaster::{DeserializeWith, ExecCommand, KillMode};
use libsysmaster::{ReDb, ReDbRoTxn, ReDbRwTxn, ReDbTable, Reliability};
use nix::unistd::Pid;
use serde::{Deserialize, Deserializer, Serialize};
use std::rc::Rc;

struct ServiceReDb<K, V>(ReDb<K, V>);

const RELI_DB_HSERVICE_CONF: &str = "svcconf";
const RELI_DB_HSERVICE_MNG: &str = "svcmng";

#[derive(PartialEq, Eq, Serialize, Deserialize, EnumString, Display, Debug, Clone, Copy)]
pub(super) enum ServiceType {
    #[strum(serialize = "simple")]
    #[serde(alias = "simple")]
    Simple,
    #[strum(serialize = "forking")]
    #[serde(alias = "forking")]
    Forking,
    #[strum(serialize = "oneshot")]
    #[serde(alias = "oneshot")]
    Oneshot,
    #[strum(serialize = "notify")]
    #[serde(alias = "notify")]
    Notify,
    #[strum(serialize = "idle")]
    Idle,
    #[strum(serialize = "exec")]
    Exec,
    TypeMax,
    TypeInvalid = -1,
}

impl Default for ServiceType {
    fn default() -> Self {
        ServiceType::Simple
    }
}

impl DeserializeWith for ServiceType {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;

        match s.as_ref() {
            "simple" => Ok(ServiceType::Simple),
            "forking" => Ok(ServiceType::Forking),
            "oneshot" => Ok(ServiceType::Oneshot),
            "notify" => Ok(ServiceType::Notify),
            &_ => Ok(ServiceType::Simple),
        }
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize, EnumString, Display, Debug, Clone, Copy)]
pub(super) enum NotifyAccess {
    #[strum(serialize = "none")]
    #[serde(alias = "none")]
    None,
    #[strum(serialize = "main")]
    #[serde(alias = "main")]
    Main,
}

#[derive(Config, Default, Clone, Debug, Serialize, Deserialize)]
pub(super) struct SectionService {
    #[config(deserialize_with = ServiceType::deserialize_with)]
    #[config(default = "simple")]
    pub Type: ServiceType,
    #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    pub ExecStart: Option<Vec<ExecCommand>>,
    #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    pub ExecStartPre: Option<Vec<ExecCommand>>,
    #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    pub ExecStartPost: Option<Vec<ExecCommand>>,
    #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    pub ExecStop: Option<Vec<ExecCommand>>,
    #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    pub ExecStopPost: Option<Vec<ExecCommand>>,
    #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    pub ExecReload: Option<Vec<ExecCommand>>,
    #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    pub ExecCondition: Option<Vec<ExecCommand>>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    pub Sockets: Option<Vec<String>>,
    pub WatchdogUSec: Option<u64>,
    pub PIDFile: Option<String>,
    #[config(default = false)]
    pub RemainAfterExit: bool,
    pub NotifyAccess: Option<NotifyAccess>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    pub Environment: Option<Vec<String>>,
    #[config(deserialize_with = KillMode::deserialize_with)]
    #[config(default = "none")]
    pub kill_mode: KillMode,
}

impl SectionService {
    pub(super) fn set_notify_access(&mut self, v: NotifyAccess) {
        self.NotifyAccess = Some(v);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ServiceReConf {
    service: SectionService,
}

impl ServiceReConf {
    fn new(servicer: &SectionService) -> ServiceReConf {
        ServiceReConf {
            service: servicer.clone(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize)]
pub(super) enum ServiceState {
    Dead,
    Condition,
    StartPre,
    Start,
    StartPost,
    Running,
    Exited,
    Reload,
    Stop,
    StopWatchdog,
    StopPost,
    StopSigterm,
    StopSigkill,
    FinalWatchdog,
    FinalSigterm,
    FinalSigkill,
    Failed,
    Cleaning,
}

impl Default for ServiceState {
    fn default() -> Self {
        ServiceState::Dead
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize)]
pub(super) enum ServiceResult {
    Success,
    FailureProtocol,
    FailureResources,
    FailureSignal,
    FailureStartLimitHit,
    ResultInvalid,
}

impl Default for ServiceResult {
    fn default() -> Self {
        ServiceResult::ResultInvalid
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone, Serialize, Deserialize)]
pub(super) enum ServiceCommand {
    Condition,
    StartPre,
    Start,
    StartPost,
    Reload,
    Stop,
    StopPost,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) enum NotifyState {
    Unknown,
    Ready,
    Stopping,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ServiceReMng {
    state: ServiceState,
    result: ServiceResult,
    main_pid: Option<i32>,    // i32 ==> nix::unistd::Pid ==> libc::pid_t
    control_pid: Option<i32>, // i32 ==> nix::unistd::Pid ==> libc::pid_t
    main_cmd_len: usize,
    control_cmd_type: Option<ServiceCommand>,
    control_cmd_len: usize,
    notify_state: NotifyState,
}

impl ServiceReMng {
    #[allow(clippy::too_many_arguments)]
    fn new(
        state: ServiceState,
        result: ServiceResult,
        main_pid: Option<i32>,
        control_pid: Option<i32>,
        main_cmd_len: usize,
        control_cmd_type: Option<ServiceCommand>,
        control_cmd_len: usize,
        notify_state: NotifyState,
    ) -> ServiceReMng {
        ServiceReMng {
            state,
            result,
            main_pid,
            control_pid,
            main_cmd_len,
            control_cmd_type,
            control_cmd_len,
            notify_state,
        }
    }
}

pub(super) struct ServiceRe {
    // database: multi-instance(N)
    conf: Rc<ServiceReDb<String, ServiceReConf>>, // RELI_DB_ESERVICE_CONF; key: unit_id, data: config;
    mng: Rc<ServiceReDb<String, ServiceReMng>>, // RELI_DB_HSERVICE_MNG; key: unit_id, data: state+result+main(pid+cmd)+control(pid+cmd)+notify_state;
}

impl ServiceRe {
    pub(super) fn new(relir: &Reliability) -> ServiceRe {
        let conf = Rc::new(ServiceReDb(ReDb::new(relir, RELI_DB_HSERVICE_CONF)));
        let mng = Rc::new(ServiceReDb(ReDb::new(relir, RELI_DB_HSERVICE_MNG)));
        let rentry = ServiceRe { conf, mng };
        rentry.register(relir);
        rentry
    }

    pub(super) fn conf_insert(&self, unit_id: &str, service: &SectionService) {
        let conf = ServiceReConf::new(service);
        self.conf.0.insert(unit_id.to_string(), conf);
    }

    pub(super) fn _conf_remove(&self, unit_id: &String) {
        self.conf.0.remove(unit_id);
    }

    pub(super) fn conf_get(&self, unit_id: &String) -> Option<SectionService> {
        let conf = self.conf.0.get(unit_id);
        conf.map(|c| c.service)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn mng_insert(
        &self,
        unit_id: &str,
        state: ServiceState,
        result: ServiceResult,
        main_pid: Option<Pid>,
        control_pid: Option<Pid>,
        main_cmd_len: usize,
        control_cmd_type: Option<ServiceCommand>,
        control_cmd_len: usize,
        notify_state: NotifyState,
    ) {
        let m_pid = main_pid.map(|x| x.as_raw() as i32);
        let c_pid = control_pid.map(|x| x.as_raw() as i32);
        let mng = ServiceReMng::new(
            state,
            result,
            m_pid,
            c_pid,
            main_cmd_len,
            control_cmd_type,
            control_cmd_len,
            notify_state,
        );
        self.mng.0.insert(unit_id.to_string(), mng);
    }

    pub(super) fn _mng_remove(&self, unit_id: &String) {
        self.mng.0.remove(unit_id);
    }

    #[allow(clippy::type_complexity)]
    pub(super) fn mng_get(
        &self,
        unit_id: &String,
    ) -> Option<(
        ServiceState,
        ServiceResult,
        Option<Pid>,
        Option<Pid>,
        usize,
        Option<ServiceCommand>,
        usize,
        NotifyState,
    )> {
        let mng = self.mng.0.get(unit_id);
        mng.map(|m| {
            (
                m.state,
                m.result,
                m.main_pid.map(Pid::from_raw),
                m.control_pid.map(Pid::from_raw),
                m.main_cmd_len,
                m.control_cmd_type,
                m.control_cmd_len,
                m.notify_state,
            )
        })
    }

    fn register(&self, relir: &Reliability) {
        // reliability-db: RELI_DB_HSERVICE_CONF
        let db = Rc::clone(&self.conf);
        relir.history_db_register(RELI_DB_HSERVICE_CONF, db);

        // reliability-db: RELI_DB_HSERVICE_MNG
        let db = Rc::clone(&self.mng);
        relir.history_db_register(RELI_DB_HSERVICE_MNG, db);
    }
}

impl ReDbTable for ServiceReDb<String, ServiceReConf> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.0.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.0.cache_2_db(db_wtxn);
    }

    fn import<'a>(&self, db_rtxn: &'a ReDbRoTxn) {
        self.0.db_2_cache(db_rtxn);
    }

    fn ignore_set(&self, ignore: bool) {
        self.0.set_ignore(ignore);
    }
}

impl ReDbTable for ServiceReDb<String, ServiceReMng> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.0.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.0.cache_2_db(db_wtxn);
    }

    fn import<'a>(&self, db_rtxn: &'a ReDbRoTxn) {
        self.0.db_2_cache(db_rtxn);
    }

    fn ignore_set(&self, ignore: bool) {
        self.0.set_ignore(ignore);
    }
}
