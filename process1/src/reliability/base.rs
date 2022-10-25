use heed::{Env, RoTxn, RwTxn};
use std::env;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;

/// reliability writeable transaction
pub struct ReDbRwTxn<'e, 'p>(pub(super) RwTxn<'e, 'p>);

impl<'e, 'p> ReDbRwTxn<'e, 'p> {
    pub(super) fn new(env: &'e Env) -> heed::Result<ReDbRwTxn> {
        env.write_txn().map(ReDbRwTxn)
    }
}

/// reliability read-only transaction
pub struct ReDbRoTxn<'e>(pub(super) RoTxn<'e>);

impl<'e> ReDbRoTxn<'e> {
    pub(super) fn new(env: &'e Env) -> heed::Result<ReDbRoTxn> {
        env.read_txn().map(ReDbRoTxn)
    }
}

/// reliability data table
pub trait ReDbTable {
    /// clear all data
    fn clear(&self, wtxn: &mut ReDbRwTxn);
    /// export all data to database
    fn export(&self, wtxn: &mut ReDbRwTxn);
    /// import all data from database
    fn import<'a>(&self, rtxn: &'a ReDbRoTxn);
    /// set the ignore flag of data
    fn ignore_set(&self, ignore: bool);
}

const RELI_PATH_DIR: &str = "/run/systemd/reliability";

pub(super) fn reli_dir_get() -> Result<String, Error> {
    // /run/systemd/reliability/
    let ret_run = reli_dir_get_run();
    if ret_run.is_ok() {
        return ret_run; // ok
    }

    // OUT_DIR/../
    let ret_out = reli_dir_get_out();
    if ret_out.is_ok() {
        return ret_out; // ok
    }

    // PROCESS_RELI_PATH
    let ret_customize = reli_dir_get_customize();
    if ret_customize.is_ok() {
        return ret_customize; // ok
    }

    // nothing exists, return failure.
    Err(Error::from(ErrorKind::NotFound))
}

/// prepare the directory for reliability.
/// the reliability path is prepared and searched according to the following priority, from high to low:
/// 1. /run/systemd/reliability/: the real running directory.
/// 2. OUT_DIR/../reliability/: make CI happy, which is target/debug/reliability/ or target/release/reliability/ usually.
/// 3. PROCESS_RELI_PATH: the path customized.
pub fn reli_dir_prepare() -> Result<(), Error> {
    // /run/systemd/reliability/
    let ret_run = reli_dir_prepare_run();
    if ret_run.is_ok() {
        return ret_run; // ok
    }
    let err_run = ret_run.unwrap_err();

    // OUT_DIR/../
    let ret_out = reli_dir_prepare_out();
    if ret_out.is_ok() {
        return ret_out; // ok
    }
    let err_out = ret_out.unwrap_err();

    // PROCESS_RELI_PATH
    let ret_customize = reli_dir_prepare_customize();
    if ret_customize.is_ok() {
        return ret_customize; // ok
    }
    let err_customize = ret_customize.unwrap_err();

    // nothing has been prepared, return failure.
    if err_customize.kind() != ErrorKind::NotFound {
        Err(err_customize)
    } else if err_out.kind() != ErrorKind::NotFound {
        Err(err_out)
    } else {
        Err(err_run)
    }
}

fn reli_dir_prepare_run() -> Result<(), Error> {
    let dir = Path::new(RELI_PATH_DIR);
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }

    log::info!(
        "prepare reliability running directory successfully: {}.",
        RELI_PATH_DIR
    );
    Ok(())
}

fn reli_dir_get_run() -> Result<String, Error> {
    let dir = Path::new(RELI_PATH_DIR);
    if dir.exists() {
        log::info!(
            "get reliability run directory successfully: {}.",
            RELI_PATH_DIR
        );
        Ok(String::from(RELI_PATH_DIR))
    } else {
        Err(Error::from(ErrorKind::NotFound))
    }
}

fn reli_dir_prepare_out() -> Result<(), Error> {
    let dir_string = out_dir_string_get();
    if let Some(d_str) = dir_string {
        let dir = Path::new(&d_str);
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        log::info!("prepare reliability out directory successfully: {}.", d_str);
        Ok(())
    } else {
        Err(Error::from(ErrorKind::NotFound))
    }
}

fn reli_dir_get_out() -> Result<String, Error> {
    let dir_string = out_dir_string_get();
    if let Some(d_str) = dir_string {
        let dir = Path::new(&d_str);
        if dir.exists() {
            log::info!("get reliability out directory successfully: {}.", d_str);
            Ok(d_str)
        } else {
            Err(Error::from(ErrorKind::NotFound))
        }
    } else {
        Err(Error::from(ErrorKind::NotFound))
    }
}

fn reli_dir_prepare_customize() -> Result<(), Error> {
    let dir_string = env::var("PROCESS_LIB_LOAD_PATH").ok();
    if let Some(d_str) = dir_string {
        let dir = Path::new(&d_str);
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        log::info!(
            "prepare reliability customized directory successfully: {}.",
            d_str
        );
        Ok(())
    } else {
        Err(Error::from(ErrorKind::NotFound))
    }
}

fn reli_dir_get_customize() -> Result<String, Error> {
    let dir_string = env::var("PROCESS_LIB_LOAD_PATH").ok();
    if let Some(d_str) = dir_string {
        let dir = Path::new(&d_str);
        if dir.exists() {
            log::info!(
                "get reliability customized directory successfully: {}.",
                d_str
            );
            Ok(d_str)
        } else {
            Err(Error::from(ErrorKind::NotFound))
        }
    } else {
        Err(Error::from(ErrorKind::NotFound))
    }
}

fn out_dir_string_get() -> Option<String> {
    let run = env::var("OUT_DIR").ok();
    let compile: Option<String> = option_env!("OUT_DIR").map(String::from);
    run.or(compile).map(|dir_string| {
        if dir_string.contains("build") {
            let _tmp: Vec<_> = dir_string.split("build").collect();
            format!("{}reliability", _tmp[0])
        } else {
            format!("{}reliability", dir_string)
        }
    })
}
