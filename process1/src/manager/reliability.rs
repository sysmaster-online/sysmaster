use heed::types::SerdeBincode;
use heed::{Database, Env, RwTxn};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::Hash;
use std::io::Error;
use std::path::Path;

pub(super) const RELI_PATH_DIR: &str = "/run/systemd/reliability";

// last
const RELI_LAST_FILE: &str = "last.mdb";
const RELI_LAST_MAX_DBS: u32 = 7;
const RELI_DB_LUNIT: &str = "unit";
const RELI_DB_LFRAME: &str = "frame";
const RELI_DB_LFQUE: &str = "queue";
const RELI_DB_LFJOB: &str = "job";
const RELI_DB_LFSIGCHLD: &str = "sigchld";
const RELI_DB_LFCGEVENT: &str = "cgroup-event";
const RELI_DB_LFMNGOP: &str = "manager-operation";
static RELI_LAST_DB_NAME: [&str; RELI_LAST_MAX_DBS as usize] = [
    RELI_DB_LUNIT,
    RELI_DB_LFRAME,
    RELI_DB_LFQUE,
    RELI_DB_LFJOB,
    RELI_DB_LFSIGCHLD,
    RELI_DB_LFCGEVENT,
    RELI_DB_LFMNGOP,
];
const RELI_LAST_KEY: u32 = 0; // singleton
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(super) enum ReliLastFrame {
    Queue,
    Job,
    SigChld,
    CgEvent,
    ManagerOp,
    OtherEvent,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(super) enum ReliLastQue {
    Load,
    GcJob,
    GcUnit,
    Clean,
    CgRealize,
    StartWhenUpheld,
    StopWhenBound,
    StopWhenUnneeded,
    Dbus,
}

// entry
const RELI_ENTRY_FILE: &str = "entry.mdb";

struct ReDb<K, V> {
    db: Database<SerdeBincode<K>, SerdeBincode<V>>,
    add: RefCell<HashMap<K, V>>,
    del: RefCell<HashSet<K>>,
}

impl<K, V> ReDb<K, V>
where
    K: 'static + Serialize + Deserialize<'static> + Eq + Hash + Clone,
    V: 'static + Serialize + Deserialize<'static> + Clone,
{
    fn new(env: &Env, name: &str) -> ReDb<K, V> {
        let db = env.create_database(Some(name)).unwrap();
        ReDb {
            db,
            add: RefCell::new(HashMap::new()),
            del: RefCell::new(HashSet::new()),
        }
    }

    fn insert(&self, k: &K, v: &V) {
        // remove "del" + insert "add"
        self.del.borrow_mut().remove(k);
        self.add.borrow_mut().insert(k.clone(), v.clone());
    }

    fn remove(&self, k: &K) {
        // remove "add" + insert "del"
        self.add.borrow_mut().remove(k);
        self.del.borrow_mut().insert(k.clone());
    }

    fn sync(&self, wtxn: &mut RwTxn) {
        // "add" -> db.put + clear "add"
        for (k, v) in self.add.borrow().iter() {
            self.db.put(wtxn, k, v).expect("entry.put");
        }
        self.add.borrow_mut().clear();

        // "del" -> db.delete + clear "del"
        for k in self.del.borrow().iter() {
            self.db.delete(wtxn, k).expect("entry.delete");
        }
        self.del.borrow_mut().clear();
    }
}

fn reli_prepare() -> Result<(), Error> {
    // directory
    let dir = Path::new(RELI_PATH_DIR);
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }

    // last
    let last = dir.join(RELI_LAST_FILE);
    if !last.exists() {
        fs::create_dir_all(&last)?;
    }

    // entry
    let entry = dir.join(RELI_ENTRY_FILE);
    if !entry.exists() {
        fs::create_dir_all(&entry)?;
    }

    Ok(())
}
