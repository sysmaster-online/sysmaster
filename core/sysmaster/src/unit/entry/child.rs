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

use super::base::UeBase;
use core::rel::ReStation;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

pub(super) struct UeChild {
    data: RefCell<UeChildData>,
}

impl ReStation for UeChild {
    // no input, no compensate

    // data
    fn db_map(&self, _reload: bool) {
        self.data.borrow_mut().db_map();
    }

    fn db_insert(&self) {
        self.data.borrow().db_insert();
    }

    // reload: no external connections, no entry
}

impl UeChild {
    pub(super) fn new(baser: &Rc<UeBase>) -> UeChild {
        let child = UeChild {
            data: RefCell::new(UeChildData::new(baser)),
        };
        child.db_insert();
        child
    }

    pub(super) fn get_pids(&self) -> Vec<Pid> {
        return self.data.borrow().get_pids();
    }

    pub(super) fn add_pids(&self, pid: Pid) {
        self.data.borrow_mut().add_pids(pid);
        self.db_update();
    }

    pub(super) fn remove_pids(&self, pid: Pid) {
        self.data.borrow_mut().remove_pids(pid);
        self.db_update();
    }
}

struct UeChildData {
    // associated objects
    base: Rc<UeBase>,

    // owned objects
    pids: HashSet<Pid>,
    _sigchldgen: u64,
}

// the declaration "pub(self)" is for identification only.
impl UeChildData {
    pub(self) fn new(baser: &Rc<UeBase>) -> UeChildData {
        UeChildData {
            base: Rc::clone(baser),
            pids: HashSet::new(),
            _sigchldgen: 0,
        }
    }

    pub(self) fn db_map(&mut self) {
        for pid in self.base.rentry_child_get().iter() {
            self.pids.insert(*pid);
        }
    }

    pub(self) fn get_pids(&self) -> Vec<Pid> {
        let mut res = Vec::new();
        for pid in self.pids.iter() {
            res.push(*pid);
        }
        res
    }

    pub(self) fn add_pids(&mut self, pid: Pid) {
        self.pids.insert(pid);
    }

    pub(self) fn remove_pids(&mut self, pid: Pid) {
        self.pids.remove(&pid);
    }

    pub(self) fn db_insert(&self) {
        let pids: Vec<Pid> = self.pids.iter().copied().collect::<_>();
        self.base.rentry_child_insert(&pids);
    }
}
