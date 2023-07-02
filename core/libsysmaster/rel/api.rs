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
    enable::{self, ReliEnable},
    history::{self, ReliHistory},
    last::{self, ReliLast},
    pending::{self, ReliPending},
    station::ReliStation,
    ReDbTable, ReStation, ReStationKind,
};
use crate::{error::*, rel::base};
use heed::Database;
use nix::sys::stat::{self, Mode};
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
#[derive(Debug)]
pub struct Reliability {
    // debug
    #[cfg(debug)]
    debug: ReliDebug,

    // control data
    enable: ReliEnable,

    // output data
    last: ReliLast,
    history: ReliHistory,
    pending: ReliPending,

    // input & recover
    station: ReliStation,
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
        let dir_s = reli_prepare().expect("reliability prepare");
        let reli = Reliability {
            #[cfg(debug)]
            debug: ReliDebug::new(&dir_s),
            enable: ReliEnable::new(&dir_s),
            last: ReliLast::new(&dir_s),
            history: ReliHistory::new(&dir_s, conf.map_size, conf.max_dbs),
            pending: ReliPending::new(&dir_s),
            station: ReliStation::new(),
        };
        reli.debug_enable();
        reli
    }

    /// set the enable flag
    pub fn set_enable(&self, enable: bool) {
        self.enable.set_enable(enable);
    }

    /// create data base for reliability
    pub fn create_database<KC, DC>(&self, name: Option<&str>) -> Result<Database<KC, DC>>
    where
        KC: 'static,
        DC: 'static,
    {
        self.history.env().create_database(name).context(HeedSnafu)
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
        if reload {
            self.db_flush();
        }
        self.make_consistent();

        // restore last's ignore
        self.last.ignore_set(false);

        // clear last
        self.last.clear_unit();
        self.last.clear_frame();
    }

    /// compact the database
    pub fn compact(&self) -> Result<()> {
        self.history.compact()
    }

    /// get the enable flag
    pub fn enable(&self) -> bool {
        self.enable.enable()
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
    pub fn history_switch(&self) -> bool {
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
        self.history.switch_set(true);

        self.station.input_rebuild();

        self.history.switch_set(false);
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
        self.history.switch_set(true);

        self.station.db_map(reload);

        self.history.switch_set(false);
    }

    /// flush all data from buffer to db
    fn db_flush(&self) {
        // clear data before using buffer
        self.history.switch_set(true);

        // update all changes to buffer
        self.station.db_insert();

        // clear db, submit data from all buffers to db, clear buffer
        self.history.flush();

        self.history.switch_set(false);

        // Due to changes in db, reload the data from db to cache.
        self.history.import();
    }

    fn make_consistent(&self) {
        // get last
        let lframe = self.last_frame();
        let lunit = self.last_unit();

        log::debug!("make_consistent, lframe:{:?}, lunit{:?}.", lframe, lunit);
        self.pending.make_consistent();
        self.station.make_consistent(lframe, lunit);
        self.history.commit();
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

    // prepare
    /* create '/run/sysmaster/reliability/sub_dir' or 'xxx/reliability/sub_dir' with mode 700 */
    let old_mask = stat::umask(Mode::from_bits_truncate(!0o700));
    let ret = reli_do_prepare(&dir_string);
    let _ = stat::umask(old_mask);
    if let Err(e) = ret {
        log::error!("reliability prepare failed: dir{:?}, {}", dir_string, e);
        return Err(e);
    }

    Ok(dir_string)
}

fn reli_do_prepare(dir_string: &str) -> Result<()> {
    // enable
    enable::prepare(dir_string)?;

    // last
    last::prepare(dir_string)?;

    // history
    history::prepare(dir_string)?;

    // pending
    pending::prepare(dir_string)?;

    Ok(())
}
