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

use super::entry::{Job, JobConf};
use super::rentry::{JobKind, JobRe};
use crate::unit::{DataManager, UnitX};
use core::rel::Reliability;
use event::Events;
use std::cell::RefCell;
use std::rc::Rc;

pub(super) struct JobAlloc {
    // associated objects
    reli: Rc<Reliability>,
    rentry: Rc<JobRe>,
    events: Rc<Events>,
    dm: Rc<DataManager>,

    // owned objects
    data: RefCell<JobAllocData>,
}

impl JobAlloc {
    pub(super) fn new(
        relir: &Rc<Reliability>,
        rentryr: &Rc<JobRe>,
        eventsr: &Rc<Events>,
        dmr: &Rc<DataManager>,
    ) -> JobAlloc {
        JobAlloc {
            reli: Rc::clone(relir),
            rentry: Rc::clone(rentryr),
            events: Rc::clone(eventsr),
            dm: Rc::clone(dmr),
            data: RefCell::new(JobAllocData::new()),
        }
    }

    pub(super) fn clear(&self) {
        self.data.borrow_mut().clear();
    }

    pub(super) fn alloc(&self, config: &JobConf) -> Rc<Job> {
        let unit = config.get_unit();
        let kind = config.get_kind();
        self.data.borrow_mut().alloc(
            &self.reli,
            &self.rentry,
            &self.events,
            &self.dm,
            Rc::clone(unit),
            kind,
        )
    }
}

#[derive(Debug)]
struct JobAllocData {
    // status
    last_id: u128,
}

// the declaration "pub(self)" is for identification only.
impl JobAllocData {
    pub(self) fn new() -> JobAllocData {
        JobAllocData { last_id: 0 }
    }

    pub(self) fn clear(&mut self) {
        self.last_id = 0;
    }

    pub(self) fn alloc(
        &mut self,
        relir: &Rc<Reliability>,
        rentryr: &Rc<JobRe>,
        eventsr: &Rc<Events>,
        dmr: &Rc<DataManager>,
        unit: Rc<UnitX>,
        kind: JobKind,
    ) -> Rc<Job> {
        let job = Rc::new(Job::new(
            relir,
            rentryr,
            eventsr,
            dmr,
            self.alloc_id(),
            unit,
            kind,
        ));
        job.get_timer().attach_job(&job);
        job.set_timer();
        job
    }

    fn alloc_id(&mut self) -> u128 {
        let id = self.last_id;
        self.last_id += 1;
        id
    }
}
