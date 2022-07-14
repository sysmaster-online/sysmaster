use cgroup;
use nix::NixPath;
use std::{cell::RefCell, path::PathBuf};
use utils::Error;
use utils::Result;

pub(super) struct UeCgroup {
    data: RefCell<UnitCgroupData>,
}

impl UeCgroup {
    pub(super) fn new() -> UeCgroup {
        UeCgroup {
            data: RefCell::new(UnitCgroupData::new()),
        }
    }

    pub(super) fn setup_cg_path(&self, id: &str) {
        self.data.borrow_mut().setup_cg_path(id);
    }

    pub(super) fn prepare_cg_exec(&self) -> Result<()> {
        self.data.borrow_mut().prepare_cg_exec()
    }

    pub(super) fn cg_path(&self) -> PathBuf {
        let cg_path = self.data.borrow().cg_path();

        return cg_path;
    }
}

struct UnitCgroupData {
    cg_path: PathBuf,
}

impl UnitCgroupData {
    pub(self) fn new() -> UnitCgroupData {
        UnitCgroupData {
            cg_path: PathBuf::from(""),
        }
    }

    pub(self) fn setup_cg_path(&mut self, id: &str) {
        if !self.cg_path.is_empty() {
            return;
        }

        self.set_default_cg_path(id);
    }

    fn set_default_cg_path(&mut self, id: &str) {
        let cg_tree_name = PathBuf::from(cgroup::cg_escape(id));

        self.cg_path = cg_tree_name;
    }

    pub(self) fn prepare_cg_exec(&mut self) -> Result<()> {
        log::debug!("cgroup: prepare cg exec");
        cgroup::cg_create(&self.cg_path).map_err(|_e| Error::Other {
            msg: "prepare cgroup failed",
        })?;

        Ok(())
    }

    pub(self) fn cg_path(&self) -> PathBuf {
        return self.cg_path.clone();
    }
}
