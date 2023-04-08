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

//! encapsulate all sub-managers of framework
//!

use std::{cell::RefCell, rc::Rc};

use event::{EventState, Events};

use crate::rules::{rule_load::DEFAULT_RULES_DIRS, Rules};

use super::{
    control_manager::{ControlManager, CONTROL_MANAGER_LISTEN_ADDR},
    job_queue::JobQueue,
    uevent_monitor::UeventMonitor,
    worker_manager::{WorkerManager, WORKER_MANAGER_LISTEN_ADDR},
};

/// encapsulate all submanagers
pub struct Devmaster {
    /// reference to events
    events: Rc<Events>,

    /// reference to worker manager
    worker_manager: Option<Rc<WorkerManager>>,
    /// reference to control manager
    control_manager: Option<Rc<ControlManager>>,
    /// reference to monitor
    monitor: Option<Rc<UeventMonitor>>,

    /// rules
    rules: Rules,
}

impl Devmaster {
    /// generate a devmaster object
    pub fn new(events: Rc<Events>) -> Rc<RefCell<Devmaster>> {
        let rules = Rules::new(&[]);

        let ret = Rc::new(RefCell::new(Devmaster {
            events: events.clone(),
            worker_manager: None,
            control_manager: None,
            monitor: None,
            rules,
        }));

        // initialize submanagers
        let worker_manager = Rc::new(WorkerManager::new(
            3,
            String::from(WORKER_MANAGER_LISTEN_ADDR),
            events.clone(),
            Rc::downgrade(&ret),
        ));
        let job_queue = Rc::new(JobQueue::new(worker_manager.clone()));
        let control_manager = Rc::new(ControlManager::new(
            String::from(CONTROL_MANAGER_LISTEN_ADDR),
            worker_manager.clone(),
            job_queue.clone(),
        ));
        let monitor = Rc::new(UeventMonitor::new(job_queue.clone()));

        // configure worker manager and monitor
        worker_manager.set_job_queue(&job_queue);
        worker_manager.set_kill_workers_timer();
        monitor.set_receive_buffer_force(1024 * 1024 * 128);

        events
            .add_source(worker_manager.get_kill_workers_timer().unwrap())
            .unwrap();
        events.add_source(worker_manager.clone()).unwrap();
        events.add_source(control_manager.clone()).unwrap();
        events.add_source(monitor.clone()).unwrap();

        events
            .set_enabled(worker_manager.clone(), EventState::On)
            .unwrap();
        events
            .set_enabled(control_manager.clone(), EventState::On)
            .unwrap();
        events.set_enabled(monitor.clone(), EventState::On).unwrap();

        ret.as_ref().borrow_mut().worker_manager = Some(worker_manager);
        ret.as_ref().borrow_mut().control_manager = Some(control_manager);
        ret.as_ref().borrow_mut().monitor = Some(monitor);

        ret
    }

    /// run the events loop
    pub fn run(&self) {
        self.events.rloop().unwrap();
    }

    /// clean up the events source
    pub fn exit(&self) {
        self.events
            .del_source(self.worker_manager.clone().unwrap())
            .unwrap();
        self.events
            .del_source(self.control_manager.clone().unwrap())
            .unwrap();
        self.events
            .del_source(self.monitor.clone().unwrap())
            .unwrap();
    }

    /// get a clone of rules
    pub(crate) fn get_rules(&self) -> Rules {
        self.rules.clone()
    }
}
