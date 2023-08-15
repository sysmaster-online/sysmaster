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

//!
use std::{
    cmp, fmt,
    io::{Error, ErrorKind},
    mem,
    str::FromStr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        RwLock,
    },
};

use log::{LevelFilter, Log, Record};

static mut LOGGER_LOCK: Option<RwLock<&dyn Log>> = None;

static STATE: AtomicUsize = AtomicUsize::new(0);

const UNINITIALIZED: usize = 0;
const INITIALIZING: usize = 1;
const INITIALIZED: usize = 2;

///
#[repr(usize)]
#[derive(Copy, Eq, Debug)]
pub enum Level {
    ///
    Error = 1,
    ///
    Warn,
    ///
    Info,
    ///
    Debug,
    ///
    Trace,
}

impl Level {
    ///
    pub fn to_ori_level(&self) -> log::Level {
        match self {
            Level::Error => log::Level::Error,
            Level::Warn => log::Level::Warn,
            Level::Info => log::Level::Info,
            Level::Debug => log::Level::Debug,
            Level::Trace => log::Level::Trace,
        }
    }
}

impl FromStr for Level {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "error" => Ok(Level::Error),
            "warn" => Ok(Level::Warn),
            "info" => Ok(Level::Info),
            "debug" => Ok(Level::Debug),
            "trace" => Ok(Level::Trace),
            _ => Ok(Level::Info),
        }
    }
}

impl Clone for Level {
    #[inline]
    fn clone(&self) -> Level {
        *self
    }
}

impl PartialEq for Level {
    #[inline]
    fn eq(&self, other: &Level) -> bool {
        *self as usize == *other as usize
    }
}

impl PartialEq<LevelFilter> for Level {
    #[inline]
    fn eq(&self, other: &LevelFilter) -> bool {
        *self as usize == *other as usize
    }
}

impl PartialOrd for Level {
    #[inline]
    fn partial_cmp(&self, other: &Level) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }

    #[inline]
    fn lt(&self, other: &Level) -> bool {
        (*self as usize) < *other as usize
    }

    #[inline]
    fn le(&self, other: &Level) -> bool {
        *self as usize <= *other as usize
    }

    #[inline]
    fn gt(&self, other: &Level) -> bool {
        *self as usize > *other as usize
    }

    #[inline]
    fn ge(&self, other: &Level) -> bool {
        *self as usize >= *other as usize
    }
}

impl PartialOrd<LevelFilter> for Level {
    #[inline]
    fn partial_cmp(&self, other: &LevelFilter) -> Option<cmp::Ordering> {
        Some((*self as usize).cmp(&(*other as usize)))
    }

    #[inline]
    fn lt(&self, other: &LevelFilter) -> bool {
        (*self as usize) < *other as usize
    }

    #[inline]
    fn le(&self, other: &LevelFilter) -> bool {
        *self as usize <= *other as usize
    }

    #[inline]
    fn gt(&self, other: &LevelFilter) -> bool {
        *self as usize > *other as usize
    }

    #[inline]
    fn ge(&self, other: &LevelFilter) -> bool {
        *self as usize >= *other as usize
    }
}

impl Ord for Level {
    #[inline]
    fn cmp(&self, other: &Level) -> cmp::Ordering {
        (*self as usize).cmp(&(*other as usize))
    }
}

/// Set the global logger
pub fn set_boxed_logger(logger: Box<dyn Log>) -> Result<(), Error> {
    set_logger_inner(|| Box::leak(logger))
}

fn set_logger_inner<F>(make_logger: F) -> Result<(), Error>
where
    F: FnOnce() -> &'static dyn Log,
{
    let old_state = match STATE.compare_exchange(
        UNINITIALIZED,
        INITIALIZING,
        Ordering::SeqCst,
        Ordering::SeqCst,
    ) {
        Ok(s) | Err(s) => s,
    };
    match old_state {
        INITIALIZED | UNINITIALIZED => {
            unsafe {
                match &LOGGER_LOCK {
                    Some(v) => {
                        let mut a = v.write().unwrap();
                        *a = make_logger();
                        STATE.store(INITIALIZED, Ordering::SeqCst);
                        return Ok(());
                    }
                    None => {}
                }
            }
            unsafe {
                LOGGER_LOCK = Some(RwLock::new(make_logger()));
                STATE.store(INITIALIZED, Ordering::SeqCst);
                Ok(())
            }
        }
        INITIALIZING => {
            while STATE.load(Ordering::SeqCst) == INITIALIZING {
                #[allow(deprecated)]
                std::sync::atomic::spin_loop_hint();
            }
            Err(Error::from(ErrorKind::InvalidData))
        }
        _ => Err(Error::from(ErrorKind::Other)),
    }
}

///
#[macro_export]
macro_rules! __log_format_args {
    ($($args:tt)*) => {
        format_args!($($args)*)
    };
}

///
#[macro_export]
macro_rules! __log_module_path {
    () => {
        module_path!()
    };
}

///
#[macro_export]
macro_rules! __log_file {
    () => {
        file!()
    };
}

///
#[macro_export]
macro_rules! __log_line {
    () => {
        line!()
    };
}

static MAX_LOG_LEVEL_FILTER: AtomicUsize = AtomicUsize::new(0);

///
#[inline(always)]
pub fn max_level() -> Level {
    unsafe { mem::transmute(MAX_LOG_LEVEL_FILTER.load(Ordering::Relaxed)) }
}

///
#[inline]
pub fn set_max_level(level: Level) {
    MAX_LOG_LEVEL_FILTER.store(level as usize, Ordering::SeqCst)
}

///
#[macro_export(local_inner_macros)]
macro_rules! log {
    (target: $target:expr, $lvl:expr, $($arg:tt)+) => ({
        let lvl = $lvl;
        if lvl <= $crate::inner::max_level() {
            $crate::inner::__private_api_log(
                __log_format_args!($($arg)+),
                lvl,
                &($target, __log_module_path!(), __log_file!(), __log_line!()),
            );
        }
    });
    ($lvl:expr, $($arg:tt)+) => (log!(target: __log_module_path!(), $lvl, $($arg)+))
}

///
#[macro_export(local_inner_macros)]
macro_rules! error {
    (target: $target:expr, $($arg:tt)+) => (
        log!(target: $target, $crate::log::Level::Error, $($arg)+)
    );
    ($($arg:tt)+) => (
        log!($crate::inner::Level::Error, $($arg)+)
    )
}

///
#[macro_export(local_inner_macros)]
macro_rules! warn {
    (target: $target:expr, $($arg:tt)+) => (
        log!(target: $target, $crate::log::Level::Warn, $($arg)+)
    );
    ($($arg:tt)+) => (
        log!($crate::inner::Level::Warn, $($arg)+)
    )
}

///
#[macro_export(local_inner_macros)]
macro_rules! info {
    (target: $target:expr, $($arg:tt)+) => (
        log!(target: $target, $crate::log::Level::Info, $($arg)+)
    );
    ($($arg:tt)+) => (
        log!($crate::inner::Level::Info, $($arg)+)
    )
}

///
#[macro_export(local_inner_macros)]
macro_rules! debug {
    (target: $target:expr, $($arg:tt)+) => (
        log!(target: $target, $crate::log::Level::Debug, $($arg)+)
    );
    ($($arg:tt)+) => (
        log!($crate::inner::Level::Debug, $($arg)+)
    )
}

///
#[macro_export(local_inner_macros)]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)+) => (
        log!(target: $target, $crate::logger::Level::Trace, $($arg)+)
    );
    ($($arg:tt)+) => (
        log!($crate::inner::Level::Trace, $($arg)+)
    )
}

///
/* Private, shouldn't be used out of this file. */
pub fn __private_api_log(
    args: fmt::Arguments,
    level: Level,
    &(target, module_path, file, line): &(&str, &'static str, &'static str, u32),
) {
    if STATE.load(Ordering::SeqCst) != INITIALIZED {
        return;
    }
    let logger = unsafe { &LOGGER_LOCK };
    match logger {
        Some(v) => {
            v.read().unwrap().log(
                &Record::builder()
                    .args(args)
                    .level(level.to_ori_level())
                    .target(target)
                    .module_path_static(Some(module_path))
                    .file_static(Some(file))
                    .line(Some(line))
                    .build(),
            );
        }
        None => {}
    }
}

pub use crate::debug;
pub use crate::error;
pub use crate::info;
pub use crate::trace;
pub use crate::warn;
