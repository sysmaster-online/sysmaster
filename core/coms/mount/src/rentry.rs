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

use bitflags::bitflags;
use core::{
    exec::{PreserveMode, Rlimit, RuntimeDirectory, StateDirectory, WorkingDirectory},
    rel::{ReDb, ReDbRwTxn, ReDbTable, ReliSwitch, Reliability},
    unit::{KillMode, KillOperation},
};
use macros::{EnumDisplay, UnitSection};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, rc::Rc};

const RELI_DB_HMOUNT_MNG: &str = "mntmng";
const RELI_DB_HMOUNTM_FRAME: &str = "mntm-frame";
const RELI_DB_HMOUNTM_CONFIG: &str = "mntconf";
const RELI_LAST_KEY: u32 = 0; // singleton

//Mount contains two states: unmounted Dead and mounted Mounted. The corresponding unit status is inactive and active
#[derive(PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize, EnumDisplay)]
pub(super) enum MountState {
    Dead,
    Mounting,
    MountingDone,
    Mounted,
    Remounting,
    Unmounting,
    RemountingSigterm,
    RemountingSigKill,
    UnmountingSigterm,
    UnmountingSigkill,
    Failed,
    // Cleaning currently not used
}

impl MountState {
    pub fn to_kill_option(self) -> KillOperation {
        match self {
            MountState::RemountingSigterm => KillOperation::KillRestart,
            MountState::UnmountingSigterm => KillOperation::KillTerminate,
            MountState::RemountingSigKill | MountState::UnmountingSigkill => {
                KillOperation::KillKill
            }
            _ => KillOperation::KillInvalid,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) enum MountResult {
    Success,
    FailureResources,
    FailureTimeout,
    FailureExitCode,
    FailureSignal,
    FailureCoreDump,
    FailureStartLimitHit,
    FailureProtocol,
}

bitflags! {
    /// Flags set when process /p/s/mountinfo
    pub struct MountProcFlags: u16 {
        /// Initial state: empty
        const EMPTY = 0;
        /// The mountpoint is mounted
        const IS_MOUNTED = 1 << 0;
        /// The mountpoint is just mounted
        const JUST_MOUNTED = 1 << 1;
        /// Some mount parameters are changed
        const JUST_CHANGED = 1 << 2;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MountReConf {
    mount: SectionMount,
}

impl MountReConf {
    fn new(mount: &SectionMount) -> MountReConf {
        MountReConf {
            mount: mount.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MountReMng {
    state: MountState,
}

impl MountReMng {
    fn new(state: MountState) -> MountReMng {
        MountReMng { state }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(super) enum MountReFrame {
    Monitor,
}

struct MountReDb<K, V>(ReDb<K, V>);

pub(super) struct MountRe {
    conf: Rc<MountReDb<String, MountReConf>>, // RELI_DB_ESERVICE_CONF; key: unit_id, data: config;

    // database: multi-instance(N)
    mng: Rc<MountReDb<String, MountReMng>>, // RELI_DB_HMOUNT_MNG; key: unit_id, data: state;

    // database: singleton(1)
    frame: Rc<MountReDb<u32, MountReFrame>>, // RELI_DB_HMOUNTM_FRAME; key: RELI_LAST_KEY, data: MountReFrame;
}

impl MountRe {
    pub(super) fn new(relir: &Rc<Reliability>) -> MountRe {
        let mng = Rc::new(MountReDb(ReDb::new(relir, RELI_DB_HMOUNT_MNG)));
        let frame = Rc::new(MountReDb(ReDb::new(relir, RELI_DB_HMOUNTM_FRAME)));
        let conf = Rc::new(MountReDb(ReDb::new(relir, RELI_DB_HMOUNTM_CONFIG)));
        let rentry = MountRe { conf, mng, frame };
        rentry.register(relir);
        rentry
    }

    pub(super) fn conf_insert(&self, unit_id: &str, service: &SectionMount) {
        let conf = MountReConf::new(service);
        self.conf.0.insert(unit_id.to_string(), conf);
    }

    pub(super) fn conf_get(&self, unit_id: &str) -> Option<SectionMount> {
        let conf = self.conf.0.get(&unit_id.to_string());
        conf.map(|c| c.mount)
    }

    pub(super) fn mng_insert(&self, unit_id: &str, state: MountState) {
        let mng = MountReMng::new(state);
        self.mng.0.insert(unit_id.to_string(), mng);
    }

    pub(super) fn mng_get(&self, unit_id: &str) -> Option<MountState> {
        let mng = self.mng.0.get(&unit_id.to_string());
        mng.map(|m| m.state)
    }

    pub(super) fn set_last_frame(&self, frame: MountReFrame) {
        self.frame.0.insert(RELI_LAST_KEY, frame);
    }

    pub(super) fn clear_last_frame(&self) {
        self.frame.0.remove(&RELI_LAST_KEY);
    }

    pub(super) fn last_frame(&self) -> Option<MountReFrame> {
        self.frame.0.get(&RELI_LAST_KEY)
    }

    fn register(&self, relir: &Reliability) {
        // rel-db: RELI_DB_HMOUNT_MNG
        let db = Rc::clone(&self.mng);
        relir.history_db_register(RELI_DB_HMOUNT_MNG, db);

        // rel-db: RELI_DB_HMOUNTM_FRAME
        let db = Rc::clone(&self.frame);
        relir.history_db_register(RELI_DB_HMOUNTM_FRAME, db);
    }
}

impl ReDbTable for MountReDb<String, MountReMng> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.0.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.0.cache_2_db(db_wtxn);
    }

    fn flush(&self, db_wtxn: &mut ReDbRwTxn, switch: ReliSwitch) {
        self.0.data_2_db(db_wtxn, switch);
    }

    fn import<'a>(&self) {
        self.0.db_2_cache();
    }

    fn switch_set(&self, switch: ReliSwitch) {
        self.0.switch_buffer(switch);
    }
}

impl ReDbTable for MountReDb<u32, MountReFrame> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.0.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.0.cache_2_db(db_wtxn);
    }

    fn flush(&self, db_wtxn: &mut ReDbRwTxn, switch: ReliSwitch) {
        self.0.data_2_db(db_wtxn, switch);
    }

    fn import<'a>(&self) {
        self.0.db_2_cache();
    }

    fn switch_set(&self, switch: ReliSwitch) {
        self.0.switch_buffer(switch);
    }
}

#[derive(UnitSection, Default, Clone, Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct SectionMount {
    #[entry(default = String::new())]
    pub What: String,
    #[entry(default = String::new())]
    pub Where: String,
    #[entry(default = String::new())]
    pub Type: String,
    #[entry(default = String::new())]
    pub Options: String,
    #[entry(default = 0o755)]
    pub DirectoryMode: u32,
    #[entry(default = false)]
    pub ForceUnmount: bool,

    // Exec
    #[entry(default = String::new())]
    pub User: String,
    #[entry(default = String::new())]
    pub Group: String,
    #[entry(default = String::from("0022"))]
    pub UMask: String,
    #[entry(parser = basic::fs::parse_pathbuf)]
    pub RootDirectory: Option<PathBuf>,
    #[entry(default = WorkingDirectory::default(), parser = core::exec::parse_working_directory)]
    pub WorkingDirectory: WorkingDirectory,
    #[entry(default = StateDirectory::default(), parser = core::exec::parse_state_directory)]
    pub StateDirectory: StateDirectory,
    #[entry(default = RuntimeDirectory::default(), parser = core::exec::parse_runtime_directory)]
    pub RuntimeDirectory: RuntimeDirectory,
    #[entry(default = PreserveMode::No)]
    pub RuntimeDirectoryPreserve: PreserveMode,
    pub LimitCORE: Option<Rlimit>,
    pub LimitNOFILE: Option<Rlimit>,
    pub LimitNPROC: Option<Rlimit>,
    #[entry(parser = core::exec::parse_environment)]
    pub Environment: Option<HashMap<String, String>>,
    #[entry(append)]
    pub EnvironmentFile: Vec<String>,
    pub SELinuxContext: Option<String>,

    // Kill
    #[entry(default = KillMode::ControlGroup)]
    pub KillMode: KillMode,
    #[entry(default = String::from("SIGTERM"))]
    pub KillSignal: String,
}
