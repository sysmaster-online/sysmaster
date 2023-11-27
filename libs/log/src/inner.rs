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
    fmt,
    io::{Error, ErrorKind},
    ops::Deref,
    sync::{
        atomic::{AtomicUsize, Ordering},
        RwLock,
    },
};

use crate::logger::ReInit;
use log::Record;

static mut LOGGER_LOCK: Option<RwLock<&dyn ReInit>> = None;

static STATE: AtomicUsize = AtomicUsize::new(0);

const UNINITIALIZED: usize = 0;
const INITIALIZING: usize = 1;
const INITIALIZED: usize = 2;

/// Set the global logger
pub fn set_boxed_logger(logger: Box<dyn ReInit>) -> Result<(), Error> {
    set_logger_inner(|| Box::leak(logger))
}

fn set_logger_inner<F>(make_logger: F) -> Result<(), Error>
where
    F: FnOnce() -> &'static dyn ReInit,
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

/// Set the global logger
pub(crate) fn reinit() {
    unsafe {
        if let Some(v) = &LOGGER_LOCK {
            let a = v
                .read()
                .expect("failed to reinit global logger as lock failed");
            a.deref().reinit();
        }
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

///
#[macro_export(local_inner_macros)]
macro_rules! log {
    (target: $target:expr, $lvl:expr, $($arg:tt)+) => ({
        let lvl = $lvl;
        if lvl <= $crate::max_level() {
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
        log!(target: $target, $crate::Level::Error, $($arg)+)
    );
    ($($arg:tt)+) => (
        log!($crate::Level::Error, $($arg)+)
    )
}

///
#[macro_export(local_inner_macros)]
macro_rules! warn {
    (target: $target:expr, $($arg:tt)+) => (
        log!(target: $target, $crate::Level::Warn, $($arg)+)
    );
    ($($arg:tt)+) => (
        log!($crate::Level::Warn, $($arg)+)
    )
}

///
#[macro_export(local_inner_macros)]
macro_rules! info {
    (target: $target:expr, $($arg:tt)+) => (
        log!(target: $target, $crate::Level::Info, $($arg)+)
    );
    ($($arg:tt)+) => (
        log!($crate::Level::Info, $($arg)+)
    )
}

///
#[macro_export(local_inner_macros)]
macro_rules! debug {
    (target: $target:expr, $($arg:tt)+) => (
        log!(target: $target, $crate::Level::Debug, $($arg)+)
    );
    ($($arg:tt)+) => (
        log!($crate::Level::Debug, $($arg)+)
    )
}

///
#[macro_export(local_inner_macros)]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)+) => (
        log!(target: $target, $crate::Level::Trace, $($arg)+)
    );
    ($($arg:tt)+) => (
        log!($crate::Level::Trace, $($arg)+)
    )
}

/// flush output stream
#[macro_export(local_inner_macros)]
macro_rules! flush {
    () => {
        $crate::inner::__private_api_flush();
    };
}

///
/* Private, shouldn't be used out of this file. */
pub fn __private_api_log(
    args: fmt::Arguments,
    level: crate::Level,
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
                    .level(level)
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

///
pub fn __private_api_flush() {
    if STATE.load(Ordering::SeqCst) != INITIALIZED {
        return;
    }
    let logger = unsafe { &LOGGER_LOCK };
    match logger {
        Some(v) => {
            v.read().unwrap().flush();
        }
        None => {}
    }
}

pub use crate::debug;
pub use crate::error;
pub use crate::info;
pub use crate::trace;
pub use crate::warn;
