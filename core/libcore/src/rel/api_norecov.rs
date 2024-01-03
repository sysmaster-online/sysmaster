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
use super::debug::ReliDebug;
use super::{
    api_comm::{bflag_path_get, create_env, hpath_path_get, reli_prepare, ReliConf},
    base::ReliSwitch,
    //enable::ReliEnable,
    history::ReliHistory,
    //last::ReliLast,
    //pending::ReliPending,
    station::ReliStation,
    ReDbTable,
    ReStation,
    ReStationKind,
};
use crate::error::*;
use crate::utils::fd;
use heed::Env;
use std::rc::Rc;
use std::{cell::RefCell, fmt};

/// reliability instance
pub struct Reliability {
    // debug
    #[cfg(debug)]
    debug: ReliDebug,

    // environment
    env: RefCell<Option<Rc<Env>>>,

    // configuration
    config: ReliConf,

    // directory
    b_exist: bool,
    hdir: String, // home-directory

    // output data
    history: ReliHistory,

    // input & recover
    station: ReliStation,
}

impl fmt::Debug for Reliability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Reliability")
            .field("env", &self.env.borrow().is_some())
            .field("history", &self.history)
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

        let reli = Reliability {
            #[cfg(debug)]
            debug: ReliDebug::new(&hdir),
            env: RefCell::new(None),
            config: conf.clone(),
            b_exist,
            hdir,
            history: ReliHistory::new(),
            station: ReliStation::new(),
        };
        reli.debug_enable();
        reli
    }

    /// set the enable flag
    pub fn set_enable(&self, _enable: bool) {
        // do nothing
    }

    /// set the last unit
    pub fn set_last_unit(&self, _unit_id: &str) {
        // do nothing
    }

    /// clear the last unit
    pub fn clear_last_unit(&self) {
        // do nothing
    }

    /// set the last frame
    pub fn set_last_frame(&self, _f1: u32, _f2: Option<u32>, _f3: Option<u32>) {
        // do nothing
    }

    /// set the last frame with just one parameter
    pub fn set_last_frame1(&self, f1: u32) {
        self.set_last_frame(f1, None, None);
    }

    /// set the last frame with two parameters
    pub fn set_last_frame2(&self, f1: u32, f2: u32) {
        self.set_last_frame(f1, Some(f2), None);
    }

    /// clear the last frame
    pub fn clear_last_frame(&self) {
        // do nothing
    }

    /// register history database
    pub fn history_db_register(&self, name: &str, db: Rc<dyn ReDbTable>) {
        self.history.db_register(name, db);
    }

    /// set the fd's 'cloexec' flag and record it
    pub fn fd_cloexec(&self, fd: i32, cloexec: bool) -> Result<()> {
        // just set the fd's 'cloexec' flag
        basic::fd::fd_cloexec(fd, cloexec).context(NixSnafu)?;
        Ok(())
    }

    /// take the fd away
    pub fn fd_take(&self, fd: i32) -> i32 {
        // do nothing
        fd
    }

    /// register a station
    pub fn station_register(&self, name: &str, kind: ReStationKind, station: Rc<dyn ReStation>) {
        self.station.station_register(name, kind, station);
    }

    /// [process reentrant] recover the data
    /// if reload is true, only map result class parameters.
    pub fn recover(&self, reload: bool) {
        self.history_import();
        self.input_rebuild();
        self.db_compensate();
        self.db_map(reload);
        self.make_consistent(reload);
    }

    /// flush to the database
    pub fn flush(&self) -> Result<()> {
        self.db_flush(true)?;
        Ok(())
    }

    /// compact the database
    pub fn compact(&self) -> Result<()> {
        // do nothing
        Ok(())
    }

    /// get the enable flag
    pub fn enable(&self) -> bool {
        false
    }

    /// get env
    pub(super) fn env(&self) -> Option<Rc<Env>> {
        self.env.borrow().clone()
    }

    /// get the last unit
    pub fn last_unit(&self) -> Option<String> {
        None
    }

    /// get the last frame
    pub fn last_frame(&self) -> Option<(u32, Option<u32>, Option<u32>)> {
        None
    }

    /// clear all data
    pub fn data_clear(&self) {
        // data-only
        /* output */
        let env = create_env(&self.config, &self.hdir, self.b_exist).unwrap();
        self.env.replace(Some(Rc::new(env)));
        self.history.data_clear(&self.env().unwrap());
        self.env.replace(None);
    }

    /// [repeating protection] clear all registers
    pub fn clear(&self) {
        self.history.clear();
        self.station.clear();
    }

    /// get the ignore flag of last data
    pub fn last_ignore(&self) -> bool {
        true
    }

    /// get the switch flag of history data
    pub fn history_switch(&self) -> ReliSwitch {
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

    fn history_import(&self) {
        let env = create_env(&self.config, &self.hdir, self.b_exist).unwrap();
        self.env.replace(Some(Rc::new(env)));
        self.history.import();
        self.env.replace(None);
    }

    fn input_rebuild(&self) {
        self.history.switch_set(ReliSwitch::Ignore);

        self.station.input_rebuild();

        self.history.switch_set(ReliSwitch::CacheAll);
    }

    fn db_compensate(&self) {
        // get last
        let lframe = self.last_frame();
        let lunit = self.last_unit();

        log::debug!("db_compensate, lframe:{:?}, lunit{:?}.", lframe, lunit);
        self.station.db_compensate(lframe, lunit);
    }

    /// map data from database
    /// reload determine whether the configuration needs to be reloaded based on the situation.
    fn db_map(&self, reload: bool) {
        self.history.switch_set(ReliSwitch::Ignore);

        self.station.db_map(reload);

        self.history.switch_set(ReliSwitch::CacheAll);
    }

    fn make_consistent(&self, _reload: bool) {
        let lframe = self.last_frame();
        let lunit = self.last_unit();
        log::debug!("make_consistent, lframe:{:?}, lunit{:?}.", lframe, lunit);

        // make consistent
        self.station.make_consistent(lframe, lunit);

        self.db_flush(false).expect("flush db");
        self.data_clear();
    }

    fn db_flush(&self, force: bool) -> Result<()> {
        let ret;

        if force {
            self.history.switch_set(ReliSwitch::Buffer); // clear data before using buffer

            // update all changes to buffer
            self.station.db_insert();

            // clear db, submit data from all buffers to db, clear buffer
            ret = self.history_flush(ReliSwitch::Buffer);

            self.history.switch_set(ReliSwitch::Ignore);
        } else {
            ret = self.history_flush(ReliSwitch::CacheAll);
            self.history.switch_set(ReliSwitch::Ignore);
        }

        ret
    }

    fn history_flush(&self, switch: ReliSwitch) -> Result<()> {
        let env = Rc::new(create_env(&self.config, &self.hdir, self.b_exist).context(HeedSnafu)?);
        self.env.replace(Some(Rc::clone(&env)));
        self.history.flush(&env, switch);
        self.env.replace(None);
        Ok(())
    }
}
