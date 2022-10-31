use super::uu_base::UeBase;
use crate::reliability::ReStation;
use libcgroup;
use libutils::Error;
use libutils::Result;
use nix::NixPath;
use std::rc::Rc;
use std::{cell::RefCell, path::PathBuf};

pub(super) struct UeCgroup {
    data: RefCell<UnitCgroupData>,
}

impl ReStation for UeCgroup {
    // no input, no compensate

    // data
    fn db_map(&self) {
        self.data.borrow_mut().db_map();
    }

    fn db_insert(&self) {
        self.data.borrow().db_insert();
    }

    // reload: no external connections, no entry
}

impl UeCgroup {
    pub(super) fn new(baser: &Rc<UeBase>) -> UeCgroup {
        let cgroup = UeCgroup {
            data: RefCell::new(UnitCgroupData::new(baser)),
        };
        cgroup.db_insert();
        cgroup
    }

    pub(super) fn setup_cg_path(&self) {
        self.data.borrow_mut().setup_cg_path();
        self.db_update();
    }

    pub(super) fn prepare_cg_exec(&self) -> Result<()> {
        self.data.borrow_mut().prepare_cg_exec()
    }

    pub(super) fn cg_path(&self) -> PathBuf {
        let cg_path = self.data.borrow().cg_path();

        cg_path
    }
}

struct UnitCgroupData {
    // associated objects
    base: Rc<UeBase>,

    // owned objects
    cg_path: PathBuf,
}

impl UnitCgroupData {
    pub(self) fn new(baser: &Rc<UeBase>) -> UnitCgroupData {
        UnitCgroupData {
            base: Rc::clone(baser),
            cg_path: PathBuf::from(""),
        }
    }

    pub(self) fn db_map(&mut self) {
        if let Some(cg_path) = self.base.rentry_cgroup_get() {
            self.cg_path = cg_path;
        }
    }

    pub(self) fn setup_cg_path(&mut self) {
        if !self.cg_path.is_empty() {
            return;
        }

        self.set_default_cg_path();
    }

    fn set_default_cg_path(&mut self) {
        let cg_tree_name = PathBuf::from(libcgroup::cg_escape(self.base.id()));

        self.cg_path = cg_tree_name;
    }

    pub(self) fn prepare_cg_exec(&mut self) -> Result<()> {
        log::debug!("cgroup: prepare cg exec");
        libcgroup::cg_create(&self.cg_path).map_err(|_e| Error::Other {
            msg: "prepare cgroup failed",
        })?;

        Ok(())
    }

    pub(self) fn cg_path(&self) -> PathBuf {
        self.cg_path.clone()
    }

    pub(self) fn db_insert(&self) {
        self.base.rentry_cgroup_insert(&self.cg_path);
    }
}
