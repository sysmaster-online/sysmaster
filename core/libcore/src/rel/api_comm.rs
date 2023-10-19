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

#[cfg(debug)]
use super::debug::{self};
use crate::{error::*, rel::base};
use base::{RELI_DIR, RELI_INTERNAL_MAX_DBS};
use basic::do_entry_or_return_io_error;
use heed::{Env, EnvOpenOptions};
use nix::sys::stat::{self, Mode};
use std::fs::{self};
use std::path::{Path, PathBuf};

/// the configuration of reliability instance
#[derive(Clone)]
pub struct ReliConf {
    // history
    pub(super) map_size: Option<usize>,
    pub(super) max_dbs: Option<u32>,
}

impl ReliConf {
    /// create the configuration
    pub fn new() -> ReliConf {
        ReliConf {
            map_size: None,
            max_dbs: None,
        }
    }

    /// set the map size
    pub fn set_map_size(&mut self, map_size: usize) -> &mut Self {
        self.map_size = Some(map_size);
        self
    }

    /// set the max numbers of db
    pub fn set_max_dbs(&mut self, max_dbs: u32) -> &mut Self {
        self.max_dbs = Some(max_dbs);
        self
    }
}

impl Default for ReliConf {
    fn default() -> Self {
        ReliConf::new()
    }
}

/// get the debug flag of switch
pub fn reli_debug_get_switch() -> bool {
    #[cfg(debug)]
    return debug::switch();
    #[cfg(not(debug))]
    return true;
}

pub(super) fn reli_prepare() -> Result<String> {
    // directory
    base::reli_dir_prepare()?; // again
    let dir_string = base::reli_dir_get().unwrap();

    // sub-directory
    /* create '/run/sysmaster/reliability/sub_dir' or 'xxx/reliability/sub_dir' with mode 700 */
    let old_mask = stat::umask(Mode::from_bits_truncate(!0o700));
    let ret = reli_subdir_prepare(&dir_string);
    let _ = stat::umask(old_mask);
    if let Err(e) = ret {
        log::error!("reliability prepare failed: dir{:?}, {}", dir_string, e);
        return Err(e);
    }

    Ok(dir_string)
}

const RELI_SUB_A_DIR: &str = "a";
const RELI_SUB_B_DIR: &str = "b";
const RELI_BFLAG_FILE: &str = "b.effect";

fn reli_subdir_prepare(hdir: &str) -> Result<()> {
    // sub-directory: reliability.mdb
    let hpath = hpath_path_get(hdir);
    #[cfg(debug)]
    if debug::empty() {
        // delete it with 'empty' flag, meaning that preparation without history database.
        if hpath.exists() {
            do_entry_or_return_io_error!(fs::remove_dir_all, hpath, "remove");
        }
    }

    if !hpath.exists() {
        do_entry_or_return_io_error!(fs::create_dir_all, hpath, "create");
    }

    // a + b
    let a = hpath.join(RELI_SUB_A_DIR);
    if !a.exists() {
        do_entry_or_return_io_error!(fs::create_dir_all, a, "create");
    }

    let b = hpath.join(RELI_SUB_B_DIR);
    if !b.exists() {
        do_entry_or_return_io_error!(fs::create_dir_all, b, "create");
    }

    Ok(())
}

pub(super) fn create_env(conf: &ReliConf, hdir: &str, b_exist: bool) -> heed::Result<Env> {
    let hpath = hpath_path_get(hdir);
    let path = hpath.join(subdir_cur_get(b_exist));
    open_env(path, conf.map_size, conf.max_dbs)
}

fn open_env(path: PathBuf, map_size: Option<usize>, max_dbs: Option<u32>) -> heed::Result<Env> {
    let mut eoo = EnvOpenOptions::new();

    // size
    if let Some(size) = map_size {
        eoo.map_size(size);
    }

    // dbs
    let mut max = RELI_INTERNAL_MAX_DBS;
    if let Some(m) = max_dbs {
        max += m;
    }
    eoo.max_dbs(max);

    // open
    eoo.open(path)
}

#[allow(dead_code)]
pub(super) fn subdir_next_get(b_exist: bool) -> String {
    if b_exist {
        // b->a
        String::from(RELI_SUB_A_DIR)
    } else {
        // a->b
        String::from(RELI_SUB_B_DIR)
    }
}

pub(super) fn subdir_cur_get(b_exist: bool) -> String {
    if b_exist {
        // b
        String::from(RELI_SUB_B_DIR)
    } else {
        // a
        String::from(RELI_SUB_A_DIR)
    }
}

pub(super) fn bflag_path_get(hpath: PathBuf) -> PathBuf {
    hpath.join(RELI_BFLAG_FILE)
}

pub(super) fn hpath_path_get(hdir: &str) -> PathBuf {
    Path::new(hdir).join(RELI_DIR)
}
