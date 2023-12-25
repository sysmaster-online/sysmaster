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

//!Parse time
#![allow(missing_docs)]
use chrono::DateTime;
use libc::{c_char, strtoll};
use libc::{
    clockid_t, CLOCK_BOOTTIME, CLOCK_BOOTTIME_ALARM, CLOCK_MONOTONIC, CLOCK_REALTIME,
    CLOCK_REALTIME_ALARM,
};
use nix::errno::Errno;
use std::ffi::CStr;
use std::ffi::CString;
use std::mem;

/// USec infinity
pub const USEC_INFINITY: u64 = u64::MAX;

/// NSec infinity
pub const NSEC_INFINITY: u64 = u64::MAX;

/// USec per Sec
pub const USEC_PER_SEC: u64 = 1000000;
/// USec per MSec
pub const USEC_PER_MSEC: u64 = 1000;
/// NSec per Sec
pub const NSEC_PER_SEC: u64 = 1000000000;
/// NSec per USec
pub const NSEC_PER_USEC: u64 = 1000;

/// USec per Minute
pub const USEC_PER_MINUTE: u64 = 60 * USEC_PER_SEC;
/// USec per Month
pub const USEC_PER_MONTH: u64 = 2629800 * USEC_PER_SEC;
/// USec per Hour
pub const USEC_PER_HOUR: u64 = 60 * USEC_PER_MINUTE;
/// USec per Day
pub const USEC_PER_DAY: u64 = 24 * USEC_PER_HOUR;
/// USec per Week
pub const USEC_PER_WEEK: u64 = 7 * USEC_PER_DAY;
/// USec per Year
pub const USEC_PER_YEAR: u64 = 31557600 * USEC_PER_SEC;

/// NSEC per Minute
pub const NSEC_PER_MINUTE: u64 = 60 * NSEC_PER_SEC;

/// parse time
/// default_unit: convert to the specified time unit
pub fn parse_time(t: &str, default_unit: u64) -> Result<u64, Errno> {
    if t.is_empty() {
        return Err(Errno::EINVAL);
    }

    let mut usec = 0;
    let mut something = false;
    let mut p = t.trim();
    let mut cstr_p;

    if p.starts_with("infinity") {
        let (_, right) = p.split_at("infinity".len());
        let s = right.trim().to_string();
        if !s.is_empty() {
            return Err(Errno::EINVAL);
        }
        return Ok(USEC_INFINITY);
    }

    loop {
        let mut multiplier = default_unit;

        p = p.trim_start();
        if p.is_empty() {
            if !something {
                return Err(Errno::EINVAL);
            }
            break;
        }

        /* Don't allow "-0" */
        if p.starts_with('-') {
            return Err(Errno::ERANGE);
        }

        cstr_p = CString::new(p).unwrap();
        let mut endp: *mut c_char = std::ptr::null_mut();
        let (l, e) = unsafe {
            let l = strtoll(cstr_p.as_ptr() as *const c_char, &mut endp, 10);
            Errno::clear();
            let errno = nix::errno::errno();
            if errno > 0 {
                return Err(nix::errno::from_i32(errno));
            }

            (l, CStr::from_ptr(endp).to_str().unwrap())
        };

        if l < 0 {
            return Err(Errno::ERANGE);
        }

        if e.starts_with('.') {
            let (_, e_right) = e.split_at(1);
            p = e_right;
            p = p.trim_start_matches(char::is_numeric);
        } else if e == p {
            return Err(Errno::EINVAL);
        } else {
            p = e;
        }

        let s = p;
        p = p.trim_start();
        extract_multiplier(&mut p, &mut multiplier);

        if s == p && !p.is_empty() {
            /* Don't allow '12.34.56', but accept '12.34 .56' or '12.34s.56' */
            return Err(Errno::EINVAL);
        }

        if l as u64 >= USEC_INFINITY / multiplier {
            return Err(Errno::ERANGE);
        }

        let mut k = l as u64 * multiplier;
        if k >= USEC_INFINITY - usec {
            return Err(Errno::ERANGE);
        }

        usec += k;

        something = true;

        if e.starts_with('.') {
            let mut m = multiplier / 10;
            let (_, e_right) = e.split_at(1);
            let e_right_byte = e_right.as_bytes();

            /* Don't allow "0.-0", "3.+1", "3. 1", "3.sec" or "3.hoge" */
            if e_right_byte.is_empty() || !e_right_byte[0].is_ascii_digit() {
                return Err(Errno::EINVAL);
            }
            for b in e_right_byte.iter() {
                if !b.is_ascii_digit() {
                    break;
                }

                k = (*b as u64 - '0' as u64) * m;
                if k >= USEC_INFINITY - usec {
                    return Err(Errno::ERANGE);
                }
                usec += k;
                m /= 10;
            }
        }
    }
    Ok(usec)
}

/// parse time to sec
pub fn parse_sec(t: &str) -> Result<u64, Errno> {
    parse_time(t, USEC_PER_SEC)
}

///parse time string to sec, include calendar string
pub fn parse_timer(date: &str) -> Result<u64, Errno> {
    let formats = ["%Y-%m-&d", "%Y-%m-%d %H:%M:%M:%S"];
    for format in formats {
        if let Ok(dt) = DateTime::parse_from_str(date, format) {
            return Ok(dt.timestamp_micros() as u64);
        }
    }
    parse_time(date, USEC_PER_SEC)
}

struct Table<'a> {
    suffix: &'a str,
    usec: u64,
}

const TABLE: &[Table] = &[
    Table {
        suffix: "seconds",
        usec: USEC_PER_SEC,
    },
    Table {
        suffix: "sec",
        usec: USEC_PER_SEC,
    },
    Table {
        suffix: "s",
        usec: USEC_PER_SEC,
    },
    Table {
        suffix: "minutes",
        usec: USEC_PER_MINUTE,
    },
    Table {
        suffix: "minute",
        usec: USEC_PER_MINUTE,
    },
    Table {
        suffix: "min",
        usec: USEC_PER_MINUTE,
    },
    Table {
        suffix: "months",
        usec: USEC_PER_MONTH,
    },
    Table {
        suffix: "month",
        usec: USEC_PER_MONTH,
    },
    Table {
        suffix: "M",
        usec: USEC_PER_MONTH,
    },
    Table {
        suffix: "msec",
        usec: USEC_PER_MSEC,
    },
    Table {
        suffix: "ms",
        usec: USEC_PER_MSEC,
    },
    Table {
        suffix: "m",
        usec: USEC_PER_MINUTE,
    },
    Table {
        suffix: "hours",
        usec: USEC_PER_HOUR,
    },
    Table {
        suffix: "hour",
        usec: USEC_PER_HOUR,
    },
    Table {
        suffix: "hr",
        usec: USEC_PER_HOUR,
    },
    Table {
        suffix: "h",
        usec: USEC_PER_HOUR,
    },
    Table {
        suffix: "days",
        usec: USEC_PER_DAY,
    },
    Table {
        suffix: "day",
        usec: USEC_PER_DAY,
    },
    Table {
        suffix: "d",
        usec: USEC_PER_DAY,
    },
    Table {
        suffix: "weeks",
        usec: USEC_PER_WEEK,
    },
    Table {
        suffix: "week",
        usec: USEC_PER_WEEK,
    },
    Table {
        suffix: "w",
        usec: USEC_PER_WEEK,
    },
    Table {
        suffix: "years",
        usec: USEC_PER_YEAR,
    },
    Table {
        suffix: "year",
        usec: USEC_PER_YEAR,
    },
    Table {
        suffix: "y",
        usec: USEC_PER_YEAR,
    },
    Table {
        suffix: "usec",
        usec: 1,
    },
    Table {
        suffix: "us",
        usec: 1,
    },
    Table {
        suffix: "µs",
        usec: 1,
    },
];

fn extract_multiplier(p: &mut &str, multiplier: &mut u64) {
    for table in TABLE {
        if p.starts_with(table.suffix) {
            *multiplier = table.usec;
            let (_, e) = p.split_at(table.suffix.len());
            *p = e;
            return;
        }
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct DualTimestamp {
    pub realtime: u64,
    pub monotonic: u64,
}

#[derive(Clone, Copy, Default)]
pub struct TripleTimestamp {
    pub realtime: u64,
    pub monotonic: u64,
    pub boottime: u64,
}

impl TripleTimestamp {
    pub fn new() -> TripleTimestamp {
        Self {
            realtime: 0,
            monotonic: 0,
            boottime: 0,
        }
    }

    pub fn now(&mut self) -> Self {
        unsafe {
            let mut tp = mem::MaybeUninit::zeroed().assume_init();
            libc::clock_gettime(libc::CLOCK_REALTIME, &mut tp);
            self.realtime = timespec_load(tp);
            libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut tp);
            self.monotonic = timespec_load(tp);
            libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut tp);
            self.boottime = timespec_load(tp);
        }
        *self
    }
}

pub fn timespec_load(ts: libc::timespec) -> u64 {
    if ts.tv_sec < 0 || ts.tv_nsec < 0 {
        return USEC_INFINITY;
    }

    if (ts.tv_sec as u64) > (USEC_INFINITY - ((ts.tv_nsec as u64) / NSEC_PER_SEC) / USEC_PER_SEC) {
        return USEC_INFINITY;
    }

    (ts.tv_sec as u64) * USEC_PER_SEC + (ts.tv_nsec as u64) / NSEC_PER_USEC
}

pub fn timespec_load_nsec(ts: libc::timespec) -> u64 {
    if ts.tv_sec < 0 || ts.tv_nsec < 0 {
        return NSEC_INFINITY;
    }

    if (ts.tv_sec as u64) >= (NSEC_INFINITY - ((ts.tv_nsec as u64) / NSEC_PER_SEC)) {
        return NSEC_INFINITY;
    }

    (ts.tv_sec as u64) * NSEC_PER_SEC + (ts.tv_nsec as u64)
}

pub fn timestamp_is_set(timestamp: u64) -> bool {
    timestamp > 0 && timestamp != USEC_INFINITY
}

pub fn duml_timestamp_is_set(dt: DualTimestamp) -> bool {
    timestamp_is_set(dt.realtime) || timestamp_is_set(dt.monotonic)
}

pub fn usec_add(a: u64, b: u64) -> u64 {
    if a > USEC_INFINITY - b {
        return USEC_INFINITY;
    }

    a + b
}

pub fn usec_sub_unsigned(a: u64, b: u64) -> u64 {
    if a == USEC_INFINITY {
        return USEC_INFINITY;
    }

    if a < b {
        return 0;
    }

    a - b
}

pub fn usec_sub_signed(a: u64, b: i64) -> u64 {
    if b == i64::MIN {
        return usec_add(a, i64::MAX as u64 + 1);
    }

    if b < 0 {
        usec_add(a, -b as u64);
    }

    usec_sub_unsigned(a, b as u64)
}

pub fn map_clock_id(c: clockid_t) -> clockid_t {
    match c {
        CLOCK_BOOTTIME_ALARM => CLOCK_BOOTTIME,
        CLOCK_REALTIME_ALARM => CLOCK_REALTIME,
        _ => c,
    }
}

pub fn now_clockid(c: clockid_t) -> u64 {
    let now = TripleTimestamp::new().now();
    match c {
        CLOCK_BOOTTIME | CLOCK_BOOTTIME_ALARM => now.boottime,
        CLOCK_REALTIME | CLOCK_REALTIME_ALARM => now.realtime,
        _ => now.monotonic,
    }
}

pub fn usec_shift_clock(x: u64, from: clockid_t, to: clockid_t) -> u64 {
    if x == USEC_INFINITY {
        return USEC_INFINITY;
    }

    if map_clock_id(from) == map_clock_id(to) {
        return x;
    }

    let a = now_clockid(from);
    let b = now_clockid(to);

    if x > a {
        usec_add(b, usec_sub_unsigned(x, a))
    } else {
        usec_sub_unsigned(b, usec_sub_unsigned(a, x))
    }
}

#[allow(unused_variables)]
pub fn triple_timestamp_by_clock(ts: TripleTimestamp, clock: clockid_t) -> u64 {
    match clock {
        CLOCK_BOOTTIME | CLOCK_BOOTTIME_ALARM => ts.boottime,
        CLOCK_MONOTONIC => ts.monotonic,
        CLOCK_REALTIME | CLOCK_REALTIME_ALARM => ts.realtime,
        _ => USEC_INFINITY,
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct UnitTimeStamp {
    pub inactive_exit_timestamp: DualTimestamp,
    pub active_enter_timestamp: DualTimestamp,
    pub active_exit_timestamp: DualTimestamp,
    pub inactive_enter_timestamp: DualTimestamp,
    pub state_change_timestamp: DualTimestamp,
}

fn map_clock_usec_internal(from: u64, from_base: u64, to_base: u64) -> u64 {
    if from >= from_base {
        let delta = from - from_base;

        if to_base >= USEC_INFINITY - delta {
            return USEC_INFINITY;
        }

        to_base + delta
    } else {
        let delta = from_base - from;
        if to_base <= delta {
            return 0;
        }

        to_base - delta
    }
}

pub fn map_clock_usec(from: u64, from_clock: clockid_t, to_clock: clockid_t) -> u64 {
    if map_clock_id(from_clock) == map_clock_id(to_clock) {
        return from;
    }

    if from == USEC_INFINITY {
        return from;
    }

    map_clock_usec_internal(from, now_clockid(from_clock), now_clockid(to_clock))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sec() {
        let u = parse_sec("5s").unwrap();
        assert_eq!(u, 5 * USEC_PER_SEC);

        let u = parse_sec("5s500ms").unwrap();
        assert_eq!(u, 5 * USEC_PER_SEC + 500 * USEC_PER_MSEC);

        let u = parse_sec(" 5s 500ms  ").unwrap();
        assert_eq!(u, 5 * USEC_PER_SEC + 500 * USEC_PER_MSEC);

        let u = parse_sec(" 5.5s  ").unwrap();
        assert_eq!(u, 5 * USEC_PER_SEC + 500 * USEC_PER_MSEC);

        let u = parse_sec(" 5.5s 0.5ms ").unwrap();
        assert_eq!(u, 5 * USEC_PER_SEC + 500 * USEC_PER_MSEC + 500);

        let u = parse_sec(" .22s ").unwrap();
        assert_eq!(u, 220 * USEC_PER_MSEC);

        let u = parse_sec("0.5min").unwrap();
        assert_eq!(u, 30 * USEC_PER_SEC);

        let u = parse_sec(" .50y ").unwrap();
        assert_eq!(u, USEC_PER_YEAR / 2);

        let u = parse_sec("2.5").unwrap();
        assert_eq!(u, 2500 * USEC_PER_MSEC);

        let u = parse_sec(".7").unwrap();
        assert_eq!(u, 700 * USEC_PER_MSEC);

        let u = parse_sec("23us").unwrap();
        assert_eq!(u, 23);

        let u = parse_sec("23µs").unwrap();
        assert_eq!(u, 23);

        let u = parse_sec("infinity").unwrap();
        assert_eq!(u, USEC_INFINITY);

        let u = parse_sec(" infinity ").unwrap();
        assert_eq!(u, USEC_INFINITY);

        let u = parse_sec("+3.1s").unwrap();
        assert_eq!(u, 3100 * USEC_PER_MSEC);

        let u = parse_sec("3.1s.2").unwrap();
        assert_eq!(u, 3300 * USEC_PER_MSEC);

        let u = parse_sec("3.1 .2").unwrap();
        assert_eq!(u, 3300 * USEC_PER_MSEC);

        let u = parse_sec("3.1 sec .2 sec").unwrap();
        assert_eq!(u, 3300 * USEC_PER_MSEC);

        let u = parse_sec("3.1 sec 1.2 sec").unwrap();
        assert_eq!(u, 4300 * USEC_PER_MSEC);

        assert!(parse_sec(" xyz ").is_err());
        assert!(parse_sec("").is_err());
        assert!(parse_sec(" . ").is_err());
        assert!(parse_sec(" 5. ").is_err());
        assert!(parse_sec(".s ").is_err());
        assert!(parse_sec("-5s ").is_err());
        assert!(parse_sec("-0.3s ").is_err());
        assert!(parse_sec("-0.0s ").is_err());
        assert!(parse_sec("-0.-0s ").is_err());
        assert!(parse_sec("0.-0s ").is_err());
        assert!(parse_sec("3.-0s ").is_err());
        assert!(parse_sec(" infinity .7").is_err());
        assert!(parse_sec(".3 infinity").is_err());
        assert!(parse_sec("3.+1s").is_err());
        assert!(parse_sec("3. 1s").is_err());
        assert!(parse_sec("3.s").is_err());
        assert!(parse_sec("12.34.56").is_err());
        assert!(parse_sec("12..34").is_err());
        assert!(parse_sec("..1234").is_err());
        assert!(parse_sec("1234..").is_err());
    }
    #[test]
    fn test_parse_time() {
        let u = parse_time("5", 1).unwrap();
        assert_eq!(u, 5);

        let u = parse_time("5", USEC_PER_MSEC).unwrap();
        assert_eq!(u, 5 * USEC_PER_MSEC);

        let u = parse_time("5", USEC_PER_SEC).unwrap();
        assert_eq!(u, 5 * USEC_PER_SEC);

        let u = parse_time("5s", 1).unwrap();
        assert_eq!(u, 5 * USEC_PER_SEC);

        let u = parse_time("5s", USEC_PER_SEC).unwrap();
        assert_eq!(u, 5 * USEC_PER_SEC);

        let u = parse_time("5s", USEC_PER_MSEC).unwrap();
        assert_eq!(u, 5 * USEC_PER_SEC);

        assert_eq!(parse_time("11111111111111y", 1), Err(Errno::ERANGE));

        let u = parse_time("1.1111111111111y", 1).unwrap();
        assert_eq!(u, 35063999999997);
    }
}
