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
//
#![allow(non_snake_case)]
use basic::config::parse_boolean;
use basic::time::USEC_INFINITY;
use core::exec::parse_mode;
use core::rel::{ReDb, ReDbRwTxn, ReDbTable, ReliSwitch, Reliability};
use core::unit::PathType;
use core::Error;
use macros::{EnumDisplay, UnitSection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::rc::Rc;

const RELI_DB_HPATH_CONF: &str = "pathconf";
const RELI_DB_HPATH_MNG: &str = "pathmng";

#[derive(UnitSection, Serialize, Deserialize, Debug, Default, Clone)]
pub struct SectionPath {
    #[entry(append, parser = parse_pathbuf_vec)]
    pub PathExists: Vec<PathBuf>,
    #[entry(append, parser = parse_pathbuf_vec)]
    pub PathExistsGlob: Vec<PathBuf>,
    #[entry(append, parser = parse_pathbuf_vec)]
    pub PathChanged: Vec<PathBuf>,
    #[entry(append, parser = parse_pathbuf_vec)]
    pub PathModified: Vec<PathBuf>,
    #[entry(append, parser = parse_pathbuf_vec)]
    pub DirectoryNotEmpty: Vec<PathBuf>,
    #[entry(default = String::new())]
    pub Unit: String,
    #[entry(default = false)]
    pub MakeDirectory: bool,
    #[entry(default = 0o755, parser = parse_mode)]
    pub DirectoryMode: u32,

    /// TODO: TriggerLimitIntervalSec=
    #[entry(default = USEC_INFINITY)]
    pub TriggerLimitIntervalSec: u64,

    /// TODO: TriggerLimitBurst=
    #[entry(default = u32::MAX)]
    pub TriggerLimitBurst: u32,
}

fn parse_pathbuf_vec(s: &str) -> Result<Vec<PathBuf>, core::error::Error> {
    let mut res = Vec::new();
    for v in s.split_ascii_whitespace() {
        let path =
            basic::fs::parse_absolute_path(v).map_err(|_| core::error::Error::ConfigureError {
                msg: "Invalid PathBuf".to_string(),
            })?;
        res.push(PathBuf::from(path));
    }
    Ok(res)
}

impl SectionPath {
    pub(super) fn set_property(
        &mut self,
        key: &str,
        value: &str,
    ) -> Result<(), core::error::Error> {
        match key {
            "PathExists" => self.PathExists = parse_pathbuf_vec(value)?,
            "PathExistsGlob" => self.PathExistsGlob = parse_pathbuf_vec(value)?,
            "PathChanged" => self.PathChanged = parse_pathbuf_vec(value)?,
            "PathModified" => self.PathModified = parse_pathbuf_vec(value)?,
            "DirectoryNotEmpty" => self.DirectoryNotEmpty = parse_pathbuf_vec(value)?,
            "Unit" => self.Unit = value.to_string(),
            "MakeDirectory" => self.MakeDirectory = parse_boolean(value)?,
            "DirectoryMode" => self.DirectoryMode = parse_mode(value)?,
            "TriggerLimitBurst" => self.TriggerLimitBurst = value.parse::<u32>()?,
            "TriggerLimitIntervalSec" => self.TriggerLimitIntervalSec = value.parse::<u64>()?,
            str_key => {
                return Err(Error::NotFound {
                    what: format!("set timer property:{}", str_key),
                });
            }
        }
        Ok(())
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize, EnumDisplay)]
pub(crate) enum PathState {
    Dead,
    Waiting,
    Running,
    Failed,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub(super) enum PathResult {
    Success,
    FailureResources,
    FailureStartLimitHit,
    FailureUnitStartLimitHit,
    FailureTriggerLimitHit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PathReConf {
    path: SectionPath,
}

impl PathReConf {
    fn new(pathr: &SectionPath) -> PathReConf {
        PathReConf {
            path: pathr.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PathReMng {
    state: PathState,
    result: PathResult,
    path_spec: Vec<(PathType, bool, String)>,
}

impl PathReMng {
    fn new(
        state: PathState,
        result: PathResult,
        path_spec: Vec<(PathType, bool, String)>,
    ) -> PathReMng {
        PathReMng {
            state,
            result,
            path_spec,
        }
    }
}

struct PathReDb<K, V>(ReDb<K, V>);

pub(super) struct PathRe {
    // database: multi-instance(N)
    conf: Rc<PathReDb<String, PathReConf>>, // RELI_DB_HSOCKET_CONF; key: unit_id, data: config;
    mng: Rc<PathReDb<String, PathReMng>>, // RELI_DB_HSOCKET_MNG; key: unit_id, data: state+result+pathspec;
}

impl PathRe {
    pub(super) fn new(relir: &Rc<Reliability>) -> PathRe {
        let conf = Rc::new(PathReDb(ReDb::new(relir, RELI_DB_HPATH_CONF)));
        let mng = Rc::new(PathReDb(ReDb::new(relir, RELI_DB_HPATH_MNG)));
        let rentry = PathRe { conf, mng };
        rentry.register(relir);
        rentry
    }

    pub(super) fn conf_insert(&self, unit_id: &str, path: &SectionPath) {
        let conf = PathReConf::new(path);
        self.conf.0.insert(unit_id.to_string(), conf);
    }

    pub(super) fn _conf_remove(&self, unit_id: &str) {
        self.conf.0.remove(&unit_id.to_string());
    }

    pub(super) fn conf_get(&self, unit_id: &str) -> Option<SectionPath> {
        let conf = self.conf.0.get(&unit_id.to_string());
        conf.map(|c| (c.path))
    }

    pub(super) fn mng_insert(
        &self,
        unit_id: &str,
        state: PathState,
        result: PathResult,
        path_spec: Vec<(PathType, bool, String)>,
    ) {
        let mng = PathReMng::new(state, result, path_spec);
        self.mng.0.insert(unit_id.to_string(), mng);
    }

    pub(super) fn _mng_remove(&self, unit_id: &str) {
        self.mng.0.remove(&unit_id.to_string());
    }

    pub(super) fn mng_get(&self, unit_id: &str) -> Option<(PathState, PathResult)> {
        let mng = self.mng.0.get(&unit_id.to_string());
        mng.map(|m| (m.state, m.result))
    }

    fn register(&self, relir: &Reliability) {
        // rel-db: RELI_DB_HPATH_CONF
        let db = Rc::clone(&self.conf);
        relir.history_db_register(RELI_DB_HPATH_CONF, db);

        // rel-db: RELI_DB_HPATH_MNG
        let db = Rc::clone(&self.mng);
        relir.history_db_register(RELI_DB_HPATH_MNG, db);
    }
}

impl ReDbTable for PathReDb<String, PathReConf> {
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

impl ReDbTable for PathReDb<String, PathReMng> {
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
