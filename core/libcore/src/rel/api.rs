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
use super::debug::{self, ReliDebug};
use super::{
    base::{RELI_DATA_FILE, RELI_DIR, RELI_INTERNAL_MAX_DBS, RELI_LOCK_FILE},
    enable::ReliEnable,
    history::ReliHistory,
    last::ReliLast,
    pending::ReliPending,
    station::ReliStation,
    ReDbTable, ReStation, ReStationKind,
};
use crate::{error::*, rel::base};
use basic::{do_entry_log, do_entry_or_return_io_error};
use heed::{CompactionOption, Env, EnvOpenOptions};
use nix::sys::stat::{self, Mode};
use std::fmt;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// the configuration of reliability instance
pub struct ReliConf {
    // history
    map_size: Option<usize>,
    max_dbs: Option<u32>,
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

/// reliability instance
pub struct Reliability {
    // debug
    #[cfg(debug)]
    debug: ReliDebug,

    // environment
    env: Rc<Env>,

    // directory
    b_exist: bool,
    hdir: String, // home-directory

    // control data
    enable: ReliEnable,

    // output data
    last: ReliLast,
    history: ReliHistory,
    pending: ReliPending,

    // input & recover
    station: ReliStation,
}

impl fmt::Debug for Reliability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Reliability")
            .field("env.path", &self.env.path())
            .field("enable", &self.enable)
            .field("last", &self.last)
            .field("history", &self.history)
            .field("pending", &self.pending)
            .field("station", &self.station)
            .finish()
    }
}

impl Drop for Reliability {
    fn drop(&mut self) {
        log::debug!("Reliability drop, clear.");
        // repeating protection
        self.clear();
    }
}

impl Reliability {
    /// create reliability instance
    pub fn new(conf: &ReliConf) -> Reliability {
        // init environment, path: dir/reliability.mdb/(a|b)/
        let hdir = reli_prepare().expect("reliability prepare");
        let hpath = hpath_path_get(&hdir);
        let b_exist = bflag_path_get(hpath.clone()).exists();
        let path = hpath.join(subdir_cur_get(b_exist));
        let e = Rc::new(open_env(path.clone(), conf.map_size, conf.max_dbs).expect("open env"));
        log::info!("open with path {:?} successfully.", path);

        let reli = Reliability {
            #[cfg(debug)]
            debug: ReliDebug::new(&hdir),
            env: Rc::clone(&e),
            b_exist,
            hdir,
            enable: ReliEnable::new(&e),
            last: ReliLast::new(&e),
            history: ReliHistory::new(&e),
            pending: ReliPending::new(&e),
            station: ReliStation::new(),
        };
        reli.debug_enable();
        reli
    }

    /// set the enable flag
    pub fn set_enable(&self, enable: bool) {
        self.enable.set_enable(enable);
    }

    /// set the last unit
    pub fn set_last_unit(&self, unit_id: &str) {
        self.last.set_unit(unit_id);
    }

    /// clear the last unit
    pub fn clear_last_unit(&self) {
        self.history.commit();
        self.last.clear_unit();
    }

    /// set the last frame
    pub fn set_last_frame(&self, f1: u32, f2: Option<u32>, f3: Option<u32>) {
        self.last.set_frame(f1, f2, f3);
    }

    /// set the last frame with just one parameter
    pub fn set_last_frame1(&self, f1: u32) {
        self.last.set_frame(f1, None, None);
    }

    /// set the last frame with two parameters
    pub fn set_last_frame2(&self, f1: u32, f2: u32) {
        self.last.set_frame(f1, Some(f2), None);
    }

    /// clear the last frame
    pub fn clear_last_frame(&self) {
        self.history.commit();
        self.last.clear_frame();
    }

    /// register history database
    pub fn history_db_register(&self, name: &str, db: Rc<dyn ReDbTable>) {
        self.history.db_register(name, db);
    }

    /// set the fd's 'cloexec' flag and record it
    pub fn fd_cloexec(&self, fd: i32, cloexec: bool) -> Result<()> {
        self.pending.fd_cloexec(fd, cloexec)
    }

    /// take the fd away
    pub fn fd_take(&self, fd: i32) -> i32 {
        self.pending.fd_take(fd)
    }

    /// register a station
    pub fn station_register(&self, name: &str, kind: ReStationKind, station: Rc<dyn ReStation>) {
        self.station.station_register(name, kind, station);
    }

    /// [process reentrant] recover the data
    /// if reload is true, only map result class parameters.
    pub fn recover(&self, reload: bool) {
        // ignore last's input
        self.last.ignore_set(true);

        self.history.import();
        self.input_rebuild();
        self.db_compensate();
        self.db_map(reload);
        self.make_consistent(reload);

        // restore last's ignore
        self.last.ignore_set(false);

        // clear last
        self.last.clear_unit();
        self.last.clear_frame();
    }

    /// compact the database
    pub fn compact(&self) -> Result<()> {
        // action with mode 700, excluding group and other users
        let old_mask = stat::umask(Mode::from_bits_truncate(!0o700));
        let ret = self.compact_body();
        let _ = stat::umask(old_mask);
        ret
    }

    fn compact_body(&self) -> Result<()> {
        // a -> b or b -> a
        // prepare next
        let hpath = hpath_path_get(&self.hdir);
        let next_path = hpath.join(subdir_next_get(self.b_exist));
        let next_file = next_path.join(RELI_DATA_FILE);

        // clear next: delete and re-create the whole directory
        do_entry_or_return_io_error!(fs::remove_dir_all, next_path, "remove");
        do_entry_or_return_io_error!(fs::create_dir_all, next_path, "create");

        // copy to next
        self.env
            .copy_to_path(next_file.clone(), CompactionOption::Disabled)
            .context(HeedSnafu)?;
        log::info!("compact to file {:?} successfully.", next_file);

        // remark the next flag at last: the another one
        let bflag = bflag_path_get(hpath.clone());
        if self.b_exist {
            do_entry_or_return_io_error!(fs::remove_file, bflag, "remove");
        } else {
            do_entry_or_return_io_error!(File::create, bflag, "create");
        }

        // try to clear previous: it would be done in the next re-exec, but we try to delete it as soon as possible.
        let cur_path = hpath.join(subdir_cur_get(self.b_exist));
        let cur_data = cur_path.join(RELI_DATA_FILE);
        let cur_lock = cur_path.join(RELI_LOCK_FILE);
        do_entry_log!(fs::remove_file, cur_data, "remove");
        do_entry_log!(fs::remove_file, cur_lock, "remove");

        Ok(())
    }

    /// get the enable flag
    pub fn enable(&self) -> bool {
        self.enable.enable()
    }

    /// get env
    pub(super) fn env(&self) -> &Env {
        &self.env
    }

    /// get the last unit
    pub fn last_unit(&self) -> Option<String> {
        self.last.unit()
    }

    /// get the last frame
    pub fn last_frame(&self) -> Option<(u32, Option<u32>, Option<u32>)> {
        self.last.frame()
    }

    /// clear all data
    pub fn data_clear(&self) {
        // data-only
        /* control */
        self.enable.data_clear();
        /* output */
        self.last.data_clear();
        self.history.data_clear();
        self.pending.data_clear();
    }

    /// [repeating protection] clear all registers
    pub fn clear(&self) {
        self.history.clear();
        self.station.clear();
    }

    /// get the ignore flag of last data
    pub fn last_ignore(&self) -> bool {
        self.last.ignore()
    }

    /// get the switch flag of history data
    pub fn history_switch(&self) -> Option<bool> {
        self.history.switch()
    }

    /// do the debug action: enable the recover process
    pub fn debug_enable(&self) {
        #[cfg(debug)]
        if self.debug.enable() {
            log::info!("reliability debug: enable data...");
            self.set_enable(true);
        }
    }

    /// do the debug action: clear data excluding enable
    pub fn debug_clear(&self) {
        #[cfg(debug)]
        if self.debug.clear() {
            // clear data excluding enable
            let enable = self.enable();
            self.data_clear();
            self.set_enable(enable);
        }
    }

    /// do the debug action: panic
    pub fn debug_panic(&self) {
        #[cfg(debug)]
        self.debug.panic();
    }

    /// do the debug action: sleep
    pub fn debug_sleep(&self) {
        #[cfg(debug)]
        self.debug.sleep();
    }

    fn input_rebuild(&self) {
        self.history.switch_set(Some(true));

        self.station.input_rebuild();

        self.history.switch_set(Some(false));
    }

    fn db_compensate(&self) {
        // get last
        let lframe = self.last_frame();
        let lunit = self.last_unit();

        log::debug!("db_compensate, lframe:{:?}, lunit{:?}.", lframe, lunit);
        self.station.db_compensate(lframe, lunit);
        self.history.commit();
    }

    /// map data from database
    /// reload determine whether the configuration needs to be reloaded based on the situation.
    fn db_map(&self, reload: bool) {
        self.history.switch_set(Some(true));

        self.station.db_map(reload);

        self.history.switch_set(Some(false));
    }

    fn make_consistent(&self, reload: bool) {
        let lframe = self.last_frame();
        let lunit = self.last_unit();
        log::debug!("make_consistent, lframe:{:?}, lunit{:?}.", lframe, lunit);

        // flush data with switch 'true'(buffer), because the data would be not equal with db.
        if reload {
            self.db_flush(true);
        }

        // make consistent and commit
        self.pending.make_consistent();
        self.station.make_consistent(lframe, lunit);
        self.history.commit();

        // flush data with switch 'false'(cache) unconditionally.
        self.db_flush(false);
    }

    fn db_flush(&self, reload: bool) {
        if reload {
            self.history.switch_set(Some(true)); // clear data before using buffer

            // update all changes to buffer
            self.station.db_insert();

            // clear db, submit data from all buffers to db, clear buffer
            self.history.flush(true); // switch = true

            self.history.switch_set(Some(false));
        } else {
            self.history.flush(false); // switch = false

            self.history.switch_set(None);
        }
    }
}

/// get the debug flag of switch
pub fn reli_debug_get_switch() -> bool {
    #[cfg(debug)]
    return debug::switch();
    #[cfg(not(debug))]
    return true;
}

fn reli_prepare() -> Result<String> {
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

fn subdir_next_get(b_exist: bool) -> String {
    if b_exist {
        // b->a
        String::from(RELI_SUB_A_DIR)
    } else {
        // a->b
        String::from(RELI_SUB_B_DIR)
    }
}

fn subdir_cur_get(b_exist: bool) -> String {
    if b_exist {
        // b
        String::from(RELI_SUB_B_DIR)
    } else {
        // a
        String::from(RELI_SUB_A_DIR)
    }
}

fn bflag_path_get(hpath: PathBuf) -> PathBuf {
    hpath.join(RELI_BFLAG_FILE)
}

fn hpath_path_get(hdir: &str) -> PathBuf {
    Path::new(hdir).join(RELI_DIR)
}
