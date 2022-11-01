use heed::types::{OwnedType, SerdeBincode};
use heed::{Database, Env, EnvOpenOptions};
use libutils::fd_util;
use nix::errno::Errno;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::io::Error;
use std::path::Path;

const RELI_PENDING_DIR: &str = "pending.mdb";
const RELI_PENDING_MAX_DBS: u32 = 1;
const RELI_DB_PFD: &str = "fd";
#[allow(dead_code)]
static RELI_PENDING_DB_NAME: [&str; RELI_PENDING_MAX_DBS as usize] = [RELI_DB_PFD];

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum ReliPState {
    Retaining,
    Retained,
    Removing,
    Removed,
}

pub(super) struct ReliPending {
    // data
    /* environment */
    env: Env,

    /* database: multi-instance(N) */
    fd: Database<OwnedType<i32>, SerdeBincode<ReliPState>>, // RELI_DB_PFD; key: fd, data: state;
}

impl fmt::Debug for ReliPending {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReliPending")
            .field("env.path", &self.env.path())
            .field("unit.len", &self.fd_len().unwrap_or(0))
            .finish()
    }
}

impl ReliPending {
    pub(super) fn new(dir_str: &str) -> ReliPending {
        // init environment
        let path = Path::new(dir_str).join(RELI_PENDING_DIR);
        let env = EnvOpenOptions::new()
            .max_dbs(RELI_PENDING_MAX_DBS)
            .open(path)
            .unwrap();

        // create db
        let fd = env.create_database(Some(RELI_DB_PFD)).unwrap();

        // return
        ReliPending { env, fd }
    }

    pub(super) fn data_clear(&self) {
        let mut wtxn = self.env.write_txn().expect("pending.write_txn");
        self.fd.clear(&mut wtxn).expect("clear.put");
        wtxn.commit().expect("pending.commit");
    }

    pub(super) fn make_consistent(&self) {
        // release
        let rtxn = self.env.read_txn().expect("pending.read_txn");
        /* fd */
        let iter = self.fd.iter(&rtxn).unwrap();
        for entry in iter {
            let (fd, _) = entry.unwrap();
            fd_util::close(fd);
        }

        // clear data
        self.data_clear();
    }

    pub(super) fn fd_cloexec(&self, fd: i32, cloexec: bool) -> Result<(), Errno> {
        match cloexec {
            true => self.fd_remove(fd),
            false => self.fd_retain(fd),
        }
    }

    pub(super) fn fd_take(&self, fd: i32) -> i32 {
        self.fd_del(fd);
        fd
    }

    fn fd_retain(&self, fd: i32) -> Result<(), Errno> {
        // repeatable protect
        if self.fd_contains(fd) {
            // error
            return Err(Errno::EBADR);
        }

        // mark pending with retaining
        self.fd_add(fd, ReliPState::Retaining);

        // action
        let ret = fd_util::fd_cloexec(fd, false);
        if ret.is_err() {
            self.fd_del(fd);
            return ret;
        }

        // mark pending to retained
        self.fd_add(fd, ReliPState::Retained);

        // return
        Ok(())
    }

    fn fd_remove(&self, fd: i32) -> Result<(), Errno> {
        // close-on-exec
        if fd_util::fd_is_cloexec(fd) {
            // debug
            self.fd_del(fd);
            return Ok(());
        }

        // mark pending with removing
        self.fd_add(fd, ReliPState::Removing);

        // action
        let ret = fd_util::fd_cloexec(fd, true);
        if ret.is_err() {
            self.fd_del(fd);
            return ret;
        }

        // delete mark
        self.fd_del(fd);

        // return
        Ok(())
    }

    fn fd_add(&self, fd: i32, state: ReliPState) {
        let mut wtxn = self.env.write_txn().expect("pending.write_txn");
        self.fd.put(&mut wtxn, &fd, &state).expect("pending.put");
        wtxn.commit().expect("pending.commit");
    }

    fn fd_del(&self, fd: i32) {
        let mut wtxn = self.env.write_txn().expect("pending.write_txn");
        self.fd.delete(&mut wtxn, &fd).expect("pending.delete");
        wtxn.commit().expect("pending.commit");
    }

    #[allow(dead_code)]
    fn fd_state(&self, fd: i32) -> Option<ReliPState> {
        let rtxn = self.env.read_txn().expect("pending.read_txn");
        self.fd.get(&rtxn, &fd).unwrap_or(None)
    }

    fn fd_contains(&self, fd: i32) -> bool {
        let rtxn = self.env.read_txn().expect("pending.read_txn");
        let contains = self.fd.get(&rtxn, &fd).unwrap_or(None);
        contains.is_some()
    }

    fn fd_len(&self) -> heed::Result<u64> {
        let rtxn = self.env.read_txn()?;
        self.fd.len(&rtxn)
    }
}

pub(super) fn prepare(dir_str: &str) -> Result<(), Error> {
    let pending = Path::new(dir_str).join(RELI_PENDING_DIR);
    if !pending.exists() {
        fs::create_dir_all(&pending)?;
    }

    Ok(())
}
