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

use crate::{
    comm::TimerUnitComm,
    config::TimerConfig,
    rentry::{TimerBase, TimerResult, TimerState},
};
use basic::{
    fs::touch_file,
    machine::Machine,
    time::{
        duml_timestamp_is_set, now_clockid, timespec_load, triple_timestamp_by_clock, usec_add,
        usec_shift_clock, DualTimestamp, TripleTimestamp, USEC_INFINITY,
    },
    IN_SET,
};
use core::{
    error::*,
    rel::ReStation,
    unit::{UnitActiveState, UnitNotifyFlags},
};
use event::{EventState, EventType, Events, Source};
use nix::{
    libc::{
        clockid_t, suseconds_t, time_t, timespec, CLOCK_BOOTTIME_ALARM, CLOCK_MONOTONIC,
        CLOCK_REALTIME,
    },
    sys::stat,
};
use rand::Rng;
use std::{
    cell::RefCell,
    path::Path,
    rc::{Rc, Weak},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub struct TimerMng {
    // associated objects
    comm: Rc<TimerUnitComm>,
    config: Rc<TimerConfig>,

    // owned objects
    state: RefCell<TimerState>,
    result: RefCell<TimerResult>,
    last_trigger: RefCell<DualTimestamp>,
    next_elapse_monotonic_or_boottime: RefCell<u64>,
    next_elapse_reltime: RefCell<u64>,
    stamp_path: String,
    on_timezone_change: bool,
    on_clock_change: bool,
    mt: Rc<MonotonicTimer>,
    rt: Rc<RealtimeTimer>,
}

impl TimerMng {
    pub(crate) fn new(
        commr: &Rc<TimerUnitComm>,
        configr: &Rc<TimerConfig>,
        mtr: &Rc<MonotonicTimer>,
        rtr: &Rc<RealtimeTimer>,
    ) -> TimerMng {
        TimerMng {
            comm: Rc::clone(commr),
            config: Rc::clone(configr),

            state: RefCell::new(TimerState::Dead),
            result: RefCell::new(TimerResult::Success),
            last_trigger: RefCell::new(DualTimestamp::default()),
            next_elapse_monotonic_or_boottime: RefCell::new(0),
            next_elapse_reltime: RefCell::new(0),
            stamp_path: String::new(),
            on_timezone_change: false,
            on_clock_change: false,
            mt: Rc::clone(mtr),
            rt: Rc::clone(rtr),
        }
    }

    fn result(&self) -> TimerResult {
        *self.result.borrow()
    }

    fn set_result(&self, res: TimerResult) {
        *self.result.borrow_mut() = res;
    }

    pub(crate) fn state(&self) -> TimerState {
        *self.state.borrow()
    }

    pub fn get_on_clock_change(&self) -> bool {
        self.on_clock_change
    }

    pub fn get_on_timezone_change(&self) -> bool {
        self.on_timezone_change
    }

    pub fn trigger_notify(&self) {
        {
            let mut v = self.config.value.borrow_mut();
            for i in 0..v.len() {
                if IN_SET!(
                    v[i].timerbase,
                    TimerBase::UnitActive,
                    TimerBase::UnitInactive
                ) {
                    v[i].disabled = false;
                }
            }
        }

        match self.state() {
            TimerState::Waiting | TimerState::Elapsed => self.enter_waiting(false),
            TimerState::Running => {
                let trigger_unit_state = self
                    .comm
                    .um()
                    .current_active_state(&self.config.unit_ref_target());
                if IN_SET!(
                    trigger_unit_state,
                    UnitActiveState::Failed,
                    UnitActiveState::InActive
                ) {
                    self.enter_waiting(false);
                }
            }
            TimerState::StateMax
            | TimerState::StateInvalid
            | TimerState::Dead
            | TimerState::Failed => {}
        }
    }

    fn set_state(&self, state: TimerState) {
        let original_state = self.state();
        *self.state.borrow_mut() = state;
        log::debug!(
            "original state: {:?}, change to: {:?}",
            original_state,
            state
        );

        if self.state() != TimerState::Waiting {
            *self.next_elapse_monotonic_or_boottime.borrow_mut() = USEC_INFINITY;
            *self.next_elapse_reltime.borrow_mut() = USEC_INFINITY;
        }

        if let Some(u) = self.comm.owner() {
            u.notify(
                original_state.to_unit_active_state(),
                state.to_unit_active_state(),
                UnitNotifyFlags::RELOAD_FAILURE,
            )
        }
    }

    pub(crate) fn current_active_state(&self) -> UnitActiveState {
        self.state().to_unit_active_state()
    }

    pub fn reset_failed(&self) {
        if self.state() == TimerState::Failed {
            self.set_state(TimerState::Dead)
        }
        self.set_result(TimerResult::Success)
    }

    pub fn start(&self) -> Result<bool> {
        assert!(IN_SET!(self.state(), TimerState::Dead, TimerState::Failed));

        if !self
            .comm
            .um()
            .load_unit_success(&self.config.unit_ref_target())
        {
            return Ok(false);
        }

        {
            let mut v = self.config.value.borrow_mut();
            for i in 0..v.len() {
                if v[i].timerbase == TimerBase::Active {
                    v[i].disabled = false;
                }
            }
        }

        if !self.stamp_path.is_empty() {
            match stat::stat(Path::new(&self.stamp_path)) {
                Ok(st) => {
                    let system_time = SystemTime::UNIX_EPOCH
                        + Duration::from_secs(st.st_mtime.try_into().unwrap());
                    let duration = system_time.duration_since(UNIX_EPOCH).unwrap();
                    let ts = timespec {
                        tv_sec: duration.as_secs() as time_t,
                        tv_nsec: duration.subsec_nanos() as suseconds_t * 1000,
                    };
                    let ft = timespec_load(ts);
                    let now_ts_reltime = now_clockid(CLOCK_REALTIME);
                    if ft < now_ts_reltime {
                        self.last_trigger.borrow_mut().realtime = ft;
                    } else {
                        log::warn!(
                            "Not using persistent file timestamp {:?} as it is in the feature",
                            ft
                        );
                    }
                }

                Err(e) => {
                    if e == Errno::ENOENT {
                        let r = touch_file(&self.stamp_path, true, None, None, None).unwrap();
                        if !r {
                            log::warn!("Failed to touch file!");
                        }
                    }
                }
            };
        }

        self.set_result(TimerResult::Success);
        self.enter_waiting(false);

        Ok(true)
    }

    pub fn stop(&self) -> Result<bool> {
        assert!(IN_SET!(
            self.state(),
            TimerState::Waiting,
            TimerState::Running,
            TimerState::Elapsed
        ));

        self.enter_dead(TimerResult::Success);

        Ok(true)
    }

    fn timer_monotonic_clock(&self) -> clockid_t {
        if self.config.config_data().borrow().Timer.WakeSystem {
            CLOCK_BOOTTIME_ALARM
        } else {
            CLOCK_MONOTONIC
        }
    }

    fn get_randomized_delay_sec(&self, time: u64) -> u64 {
        if time == 0 {
            return 0;
        }
        rand::thread_rng().gen_range(0..time)
    }

    pub fn enter_waiting(&self, time_change: bool) {
        let mut found_monotonic = false;
        let found_realtime = false;
        let mut leave_around = false;

        let timer_timestamp = self.comm.owner().unwrap().get_unit_timestamp();
        let trigger_unit_timestamp = self
            .comm
            .um()
            .get_unit_timestamp(&self.config.unit_ref_target());
        let tts = TripleTimestamp::new().now();

        let mut v = self.config.value.borrow_mut();
        for i in 0..v.len() {
            if v[i].disabled {
                continue;
            }

            let mut base = 0;
            match v[i].timerbase {
                TimerBase::Active => {
                    if self.state().to_unit_active_state() == UnitActiveState::Active {
                        base = timer_timestamp.borrow().active_exit_timestamp.monotonic;
                    } else {
                        base = tts.monotonic;
                    }
                }
                TimerBase::Boot | TimerBase::Startup => {
                    if v[i].timerbase == TimerBase::Boot
                        && IN_SET!(
                            Machine::detect_container(),
                            Machine::Docker,
                            Machine::Podman,
                            Machine::Containerd
                        )
                    {
                        todo!()
                    }
                }
                TimerBase::UnitActive => {
                    leave_around = true;
                    base = std::cmp::max(
                        trigger_unit_timestamp
                            .borrow()
                            .inactive_exit_timestamp
                            .monotonic,
                        self.last_trigger.borrow().monotonic,
                    );
                    if base == 0 {
                        continue;
                    }
                }
                TimerBase::UnitInactive => {
                    leave_around = true;
                    base = std::cmp::max(
                        trigger_unit_timestamp
                            .borrow()
                            .inactive_enter_timestamp
                            .monotonic,
                        self.last_trigger.borrow().monotonic,
                    );
                    if base == 0 {
                        continue;
                    }
                }
                TimerBase::Calendar => {
                    todo!()
                }
                TimerBase::BaseMax | TimerBase::BaseInvalid => {}
            }

            if v[i].timerbase != TimerBase::Calendar {
                v[i].next_elapse = usec_add(
                    usec_shift_clock(base, CLOCK_MONOTONIC, self.timer_monotonic_clock()),
                    v[i].value,
                );

                if duml_timestamp_is_set(*self.last_trigger.borrow())
                    && !time_change
                    && v[i].next_elapse
                        < triple_timestamp_by_clock(tts, self.timer_monotonic_clock())
                    && IN_SET!(
                        v[i].timerbase,
                        TimerBase::Active,
                        TimerBase::Boot,
                        TimerBase::Startup
                    )
                {
                    v[i].disabled = true;
                    continue;
                }

                if !found_monotonic {
                    *self.next_elapse_monotonic_or_boottime.borrow_mut() = v[i].next_elapse - base;
                } else {
                    *self.next_elapse_monotonic_or_boottime.borrow_mut() = std::cmp::min(
                        *self.next_elapse_monotonic_or_boottime.borrow(),
                        v[i].next_elapse,
                    ) - base;
                }

                found_monotonic = true;
            }
        }

        if !found_monotonic && !found_realtime && !self.on_timezone_change && !self.on_clock_change
        {
            self.enter_elapsed(leave_around);
            return;
        }

        if found_monotonic {
            let events = self.comm.um().events();
            let source = Rc::clone(&self.mt);
            events.del_source(source.clone()).unwrap();

            self.mt.set_time(usec_add(
                *self.next_elapse_monotonic_or_boottime.borrow(),
                self.get_randomized_delay_sec(
                    self.config.config_data().borrow().Timer.RandomizedDelaySec,
                ),
            ));
            events.add_source(source.clone()).unwrap();
            events.set_enabled(source, EventState::OneShot).unwrap();
        }

        if found_realtime {
            let events = self.comm.um().events();
            let source = Rc::clone(&self.rt);
            events.del_source(source.clone()).unwrap();

            self.rt.set_time(usec_add(
                *self.next_elapse_reltime.borrow(),
                self.get_randomized_delay_sec(
                    self.config.config_data().borrow().Timer.RandomizedDelaySec,
                ),
            ));
            events.add_source(source.clone()).unwrap();
            events.set_enabled(source, EventState::OneShot).unwrap();
        }

        self.set_state(TimerState::Waiting);
    }

    pub fn enter_running(&self) {
        if let Some(u) = self.comm.owner() {
            if self.comm.um().has_stop_job(&u.id()) {
                return;
            }

            let ret = self
                .comm
                .um()
                .unit_start_by_job(&self.config.unit_ref_target());
            if ret.is_err() {
                log::warn!(
                    "{}: Failed to queue unit startup job!",
                    &self.config.unit_ref_target()
                );
                self.enter_dead(TimerResult::FailureResources);
            }

            self.last_trigger.borrow_mut().realtime = now_clockid(CLOCK_REALTIME);
            self.last_trigger.borrow_mut().monotonic = now_clockid(CLOCK_MONOTONIC);

            if !self.stamp_path.is_empty() {
                touch_file(&self.stamp_path, true, None, None, None).unwrap();
            }

            self.set_state(TimerState::Running);
        } else {
            self.enter_dead(TimerResult::FailureResources);
        }
    }

    pub fn enter_elapsed(&self, level_around: bool) {
        if level_around || self.config.config_data().borrow().Timer.RemainAfterElapse {
            self.set_state(TimerState::Elapsed)
        } else {
            self.enter_dead(TimerResult::Success)
        }
    }

    pub fn enter_dead(&self, res: TimerResult) {
        log::debug!("timer enter dead state, res {:?}", res);
        if self.result() == TimerResult::Success {
            self.set_result(res);
        }

        let state = if self.result() == TimerResult::Success {
            TimerState::Dead
        } else {
            TimerState::Failed
        };

        self.set_state(state);
    }
}

impl ReStation for TimerMng {
    // input: do nothing

    // compensate: do nothing

    // data
    fn db_map(&self, _reload: bool) {
        let mut lt = self.last_trigger.borrow_mut();
        if let Some((state, result, last_trigger_realtime, last_trigger_monotonic)) =
            self.comm.rentry_mng_get()
        {
            *self.state.borrow_mut() = state;
            *self.result.borrow_mut() = result;
            lt.realtime = last_trigger_realtime;
            lt.monotonic = last_trigger_monotonic;
        }
    }

    fn db_insert(&self) {
        let lt = self.last_trigger.borrow();
        self.comm
            .rentry_mng_insert(self.state(), self.result(), lt.realtime, lt.monotonic);
    }
}

impl TimerState {
    pub(super) fn to_unit_active_state(self) -> UnitActiveState {
        match self {
            TimerState::Dead => UnitActiveState::InActive,
            TimerState::Waiting | TimerState::Running | TimerState::Elapsed => {
                UnitActiveState::Active
            }
            TimerState::Failed => UnitActiveState::Failed,
            TimerState::StateMax | TimerState::StateInvalid => UnitActiveState::DeActivating,
        }
    }
}

pub struct MonotonicTimer {
    monotonic: RefCell<u64>,
    mng: RefCell<Weak<TimerMng>>,
}

impl MonotonicTimer {
    pub fn new(monotonic: u64) -> Self {
        MonotonicTimer {
            monotonic: RefCell::new(monotonic),
            mng: RefCell::new(Weak::new()),
        }
    }

    pub fn attach_mng(&self, mng: Weak<TimerMng>) {
        *self.mng.borrow_mut() = mng;
    }

    pub(self) fn mng(&self) -> Rc<TimerMng> {
        self.mng.borrow().clone().upgrade().unwrap()
    }

    pub fn set_time(&self, usec: u64) {
        *self.monotonic.borrow_mut() = usec;
    }

    pub fn do_dispatch(&self) -> i32 {
        if self.mng().state() != TimerState::Waiting {
            return 0;
        }

        log::debug!("Timer elapsed.");
        self.mng().enter_running();
        0
    }
}

impl Source for MonotonicTimer {
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn event_type(&self) -> EventType {
        if self.mng().config.config_data().borrow().Timer.WakeSystem {
            EventType::TimerBoottimeAlarm
        } else {
            EventType::TimerMonotonic
        }
    }

    fn time_relative(&self) -> u64 {
        *self.monotonic.borrow()
    }

    fn dispatch(&self, _event: &Events) -> i32 {
        self.do_dispatch()
    }
}

pub struct RealtimeTimer {
    realtime: RefCell<u64>,
    mng: RefCell<Weak<TimerMng>>,
}

impl RealtimeTimer {
    pub fn new(realtime: u64) -> Self {
        RealtimeTimer {
            realtime: RefCell::new(realtime),
            mng: RefCell::new(Weak::new()),
        }
    }

    pub fn attach_mng(&self, mng: Weak<TimerMng>) {
        *self.mng.borrow_mut() = mng;
    }

    pub(self) fn mng(&self) -> Rc<TimerMng> {
        self.mng.borrow().clone().upgrade().unwrap()
    }

    pub fn set_time(&self, usec: u64) {
        *self.realtime.borrow_mut() = usec;
    }

    pub fn do_dispatch(&self) -> i32 {
        if self.mng().state() != TimerState::Waiting {
            return 0;
        }

        log::debug!("Timer elapsed.");
        self.mng().enter_running();
        0
    }
}

impl Source for RealtimeTimer {
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn event_type(&self) -> EventType {
        if self.mng().config.config_data().borrow().Timer.WakeSystem {
            EventType::TimerRealtimeAlarm
        } else {
            EventType::TimerRealtime
        }
    }

    fn time_relative(&self) -> u64 {
        *self.realtime.borrow()
    }

    fn dispatch(&self, _event: &Events) -> i32 {
        self.do_dispatch()
    }
}
