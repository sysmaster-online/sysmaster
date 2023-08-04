use super::base::{self};
use crate::error::*;
use basic::do_entry_or_return_io_error;
use std::fs::{self, File};
use std::path::Path;
use std::thread;
use std::time::Duration;

const RELI_DEBUG_SWITCH_FILE: &str = "switch.debug";
const RELI_DEBUG_EMPTY_FILE: &str = "empty.debug";
const RELI_DEBUG_CLEAR_FILE: &str = "clear.debug";
const RELI_DEBUG_CFIRST_FILE: &str = "clear_first.debug";
const RELI_DEBUG_ENABLE_FILE: &str = "enable.debug";
const RELI_DEBUG_PANIC_FILE: &str = "panic.debug";
const RELI_DEBUG_PFIRST_FILE: &str = "panic_first.debug";
const RELI_DEBUG_SLEEP_FILE: &str = "sleep.debug";

#[derive(Debug)]
pub(super) struct ReliDebug {
    hdir: String, // home-directory
}

impl ReliDebug {
    pub(super) fn new(dir_str: &str) -> ReliDebug {
        ReliDebug {
            hdir: String::from(dir_str),
        }
    }

    pub(super) fn enable(&self) -> bool {
        let enable = Path::new(&self.hdir).join(RELI_DEBUG_ENABLE_FILE);
        enable.exists()
    }

    pub(super) fn clear(&self) -> bool {
        let clear = Path::new(&self.hdir).join(RELI_DEBUG_CLEAR_FILE);
        if clear.exists() {
            let cfirst = Path::new(&self.hdir).join(RELI_DEBUG_CFIRST_FILE);
            if cfirst.exists() {
                // do nothing
                log::info!("reliability debug_clear: non-first time, do nothing.");
                false
            } else {
                File::create(&cfirst).unwrap();
                log::debug!("Successfully created {:?}", cfirst);

                // clear data
                log::info!("reliability debug_clear: first time, clear ...");
                true
            }
        } else {
            false
        }
    }

    pub(super) fn panic(&self) {
        let panic = Path::new(&self.hdir).join(RELI_DEBUG_PANIC_FILE);
        if panic.exists() {
            let pfirst = Path::new(&self.hdir).join(RELI_DEBUG_PFIRST_FILE);
            if pfirst.exists() {
                // do nothing
                log::info!("reliability debug_panic: non-first time, do nothing.");
            } else {
                File::create(&pfirst).unwrap();
                log::debug!("Successfully created {:?}", pfirst);

                // panic
                log::info!("reliability debug_panic: first time, panic ...");
                panic!("first debug_panic.");
            }
        }
    }

    pub(super) fn sleep(&self) {
        let sleep = Path::new(&self.hdir).join(RELI_DEBUG_SLEEP_FILE);
        if sleep.exists() {
            log::info!("reliability debug: sleep...");
            thread::sleep(Duration::from_secs(3600));
        }
    }
}

#[allow(dead_code)]
pub(super) fn enable_switch(enable: bool) -> Result<()> {
    // do the debug action: enable or disable switch flag. effective after restart.
    log::info!("reliability debug: enable[{}] switch.", enable);

    let dir_string = base::reli_dir_get().unwrap();
    let switch = Path::new(&dir_string).join(RELI_DEBUG_SWITCH_FILE);
    // touch switch.debug if enable
    if enable && !switch.exists() {
        do_entry_or_return_io_error!(File::create, switch, "create");
    }
    // remove switch.debug if disable
    if !enable && switch.exists() {
        do_entry_or_return_io_error!(fs::remove_file, switch, "remove");
    }

    Ok(())
}

pub(super) fn switch() -> bool {
    let dir_string = base::reli_dir_get().expect("guaranteed by caller.");
    let switch = Path::new(&dir_string).join(RELI_DEBUG_SWITCH_FILE);
    log::info!("reliability debug: get switch file: {:?}.", switch);
    switch.exists()
}

pub(super) fn empty() -> bool {
    let dir_string = base::reli_dir_get().expect("guaranteed by caller.");
    let empty = Path::new(&dir_string).join(RELI_DEBUG_EMPTY_FILE);
    log::info!("reliability empty: get empty file: {:?}.", empty);
    empty.exists()
}
