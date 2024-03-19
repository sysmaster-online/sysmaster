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
#![allow(non_snake_case)]
use basic::time::{parse_timer, USEC_INFINITY};
use core::{
    rel::{ReDb, ReDbRwTxn, ReDbTable, ReliSwitch, Reliability},
    Error,
};
use macros::{EnumDisplay, UnitSection};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

struct TimerReDb<K, V>(ReDb<K, V>);
const RELI_DB_HTIMER_CONF: &str = "timerconf";
const RELI_DB_HTIMER_MNG: &str = "timermng";

#[derive(PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize, EnumDisplay)]
pub(super) enum TimerState {
    Dead,
    Waiting,
    Running,
    Elapsed,
    Failed,
    StateMax,
    StateInvalid,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize, EnumDisplay)]
pub enum TimerBase {
    Active,
    Boot,
    Startup,
    UnitActive,
    UnitInactive,
    Calendar,
    BaseMax,
    BaseInvalid,
}

impl Default for TimerBase {
    fn default() -> TimerBase {
        TimerBase::BaseMax
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize, Default)]
struct CalendarComponent {
    start: i32,
    stop: i32,
    repeat: i32,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalendarSpec {
    weekdays_bits: i32,
    end_of_month: bool,
    utc: bool,
    dst: i32,
    timezone: String,

    year: CalendarComponent,
    month: CalendarComponent,
    day: CalendarComponent,
    hour: CalendarComponent,
    minute: CalendarComponent,
    microsecond: CalendarComponent,
}

#[derive(UnitSection, Default, Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SectionTimer {
    #[entry(default = USEC_INFINITY, parser = parse_timer)]
    pub AccuracySec: u64,
    #[entry(default = USEC_INFINITY, parser = parse_timer)]
    pub OnActiveSec: u64,
    #[entry(default = USEC_INFINITY, parser = parse_timer)]
    pub OnBootSec: u64,
    #[entry(default = USEC_INFINITY, parser = parse_timer)]
    pub OnStartupSec: u64,
    #[entry(default = USEC_INFINITY, parser = parse_timer)]
    pub OnUnitActiveSec: u64,
    #[entry(default = USEC_INFINITY, parser = parse_timer)]
    pub OnUnitInactiveSec: u64,
    #[entry(default = USEC_INFINITY, parser = parse_timer)]
    pub OnCalendar: u64,
    #[entry(default = 0, parser = parse_timer)]
    pub RandomizedDelaySec: u64,
    pub Unit: Option<String>,
    #[entry(default = false)]
    pub Persistent: bool,
    #[entry(default = false)]
    pub WakeSystem: bool,
    #[entry(default = true)]
    pub RemainAfterElapse: bool,
}

impl SectionTimer {
    pub(super) fn set_property(
        &mut self,
        key: &str,
        value: &str,
    ) -> Result<(), core::error::Error> {
        match key {
            "AccuracySec" => self.AccuracySec = parse_timer(value)?,
            "OnActiveSec" => self.OnActiveSec = parse_timer(value)?,
            "OnBootSec" => self.OnBootSec = parse_timer(value)?,
            "OnStartupSec" => self.OnStartupSec = parse_timer(value)?,
            "OnUnitActiveSec" => self.OnUnitActiveSec = parse_timer(value)?,
            "OnUnitInactiveSec" => self.OnUnitInactiveSec = parse_timer(value)?,
            "OnCalendar" => self.OnCalendar = parse_timer(value)?,
            "RandomizedDelaySec" => self.RandomizedDelaySec = parse_timer(value)?,
            "Unit" => {
                self.Unit = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            }
            "Persistent" => self.Persistent = basic::config::parse_boolean(value)?,
            "WakeSystem" => self.WakeSystem = basic::config::parse_boolean(value)?,
            "RemainAfterElapse" => self.RemainAfterElapse = basic::config::parse_boolean(value)?,
            str_key => {
                return Err(Error::NotFound {
                    what: format!("set timer property:{}", str_key),
                });
            }
        }
        Ok(())
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimerValue {
    pub timerbase: TimerBase,
    pub disabled: bool,
    pub value: u64,
    pub calender_spec: CalendarSpec,
    pub next_elapse: u64,
}

impl TimerValue {
    pub fn new(
        timerbase: TimerBase,
        disabled: bool,
        value: u64,
        calender_spec: CalendarSpec,
        next_elapse: u64,
    ) -> TimerValue {
        TimerValue {
            timerbase,
            disabled,
            value,
            calender_spec,
            next_elapse,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum TimerResult {
    Success,
    FailureResources,
    FailureStartLimitHit,
    ResultMax,
    ResultInvalid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TimerReConf {
    timer: SectionTimer,
    unit: String,
}

impl TimerReConf {
    fn new(timer: &SectionTimer, unit: String) -> TimerReConf {
        TimerReConf {
            timer: timer.clone(),
            unit,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TimerReMng {
    state: TimerState,
    result: TimerResult,
    last_trigger_realtime: u64,
    last_trigger_monotonic: u64,
}

impl TimerReMng {
    fn new(
        state: TimerState,
        result: TimerResult,
        last_trigger_realtime: u64,
        last_trigger_monotonic: u64,
    ) -> TimerReMng {
        TimerReMng {
            state,
            result,
            last_trigger_realtime,
            last_trigger_monotonic,
        }
    }
}

pub(super) struct TimerRe {
    // database: multi-instance(N)
    conf: Rc<TimerReDb<String, TimerReConf>>, // RELI_DB_ETIMER_CONF;
    mng: Rc<TimerReDb<String, TimerReMng>>,   // RELI_DB_HTIMER_MNG;
}

impl TimerRe {
    pub(super) fn new(relir: &Rc<Reliability>) -> TimerRe {
        let conf = Rc::new(TimerReDb(ReDb::new(relir, RELI_DB_HTIMER_CONF)));
        let mng = Rc::new(TimerReDb(ReDb::new(relir, RELI_DB_HTIMER_MNG)));
        let rentry = TimerRe { conf, mng };
        rentry.register(relir);
        rentry
    }

    pub(super) fn conf_insert(&self, unit_id: &str, timer: &SectionTimer, unit: String) {
        let conf = TimerReConf::new(timer, unit);
        self.conf.0.insert(unit_id.to_string(), conf);
    }

    pub(super) fn _conf_remove(&self, unit_id: &str) {
        self.conf.0.remove(&unit_id.to_string());
    }

    pub(super) fn conf_get(&self, unit_id: &str) -> Option<(SectionTimer, String)> {
        let conf = self.conf.0.get(&unit_id.to_string());
        conf.map(|c| (c.timer, c.unit))
    }

    pub(super) fn mng_insert(
        &self,
        unit_id: &str,
        state: TimerState,
        result: TimerResult,
        last_trigger_realtime: u64,
        last_trigger_monotonic: u64,
    ) {
        let mng = TimerReMng::new(state, result, last_trigger_realtime, last_trigger_monotonic);
        self.mng.0.insert(unit_id.to_string(), mng);
    }

    pub(super) fn _mng_remove(&self, unit_id: &str) {
        self.mng.0.remove(&unit_id.to_string());
    }

    pub(super) fn mng_get(&self, unit_id: &str) -> Option<(TimerState, TimerResult, u64, u64)> {
        let mng = self.mng.0.get(&unit_id.to_string());
        mng.map(|m| {
            (
                m.state,
                m.result,
                m.last_trigger_realtime,
                m.last_trigger_monotonic,
            )
        })
    }

    fn register(&self, relir: &Reliability) {
        // rel-db: RELI_DB_HTIMER_CONF
        let db = Rc::clone(&self.conf);
        relir.history_db_register(RELI_DB_HTIMER_CONF, db);

        // rel-db: RELI_DB_HTIMER_MNG
        let db = Rc::clone(&self.mng);
        relir.history_db_register(RELI_DB_HTIMER_MNG, db);
    }
}

impl ReDbTable for TimerReDb<String, TimerReConf> {
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

impl ReDbTable for TimerReDb<String, TimerReMng> {
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
