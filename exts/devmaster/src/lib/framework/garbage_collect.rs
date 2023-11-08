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

//! Recycle resources, including idle workers, residual devnodes, etc.
//!

use super::*;
use crate::rules::node::cleanup_prior_dir;
use event::*;
use std::{
    cell::RefCell,
    os::unix::io::RawFd,
    rc::{Rc, Weak},
};

/// max time interval for idle worker
const WORKER_MAX_IDLE_INTERVAL: u64 = 3;

pub(crate) struct GarbageCollect {
    devmaster: Weak<RefCell<Devmaster>>,

    killer: RefCell<Option<Rc<IdleWorkerKiller>>>,
}

impl GarbageCollect {
    pub(crate) fn new(devmaster: &Rc<RefCell<Devmaster>>) -> Self {
        Self {
            devmaster: Rc::downgrade(devmaster),
            killer: RefCell::new(None),
        }
    }

    pub(crate) fn start_killer(&self, e: &Events) {
        let devmaster = self.devmaster.upgrade().unwrap();
        let worker_manager = devmaster.borrow().worker_manager.clone().unwrap();
        let killer = Rc::new(IdleWorkerKiller::new(
            WORKER_MAX_IDLE_INTERVAL,
            worker_manager,
        ));

        if let Err(error) = e.add_source(killer.clone()) {
            log::error!("Failed to add idle worker killer: {}", error);
        }

        if let Err(error) = e.set_enabled(killer.clone(), EventState::OneShot) {
            log::error!("Failed to enable idle worker killer: {}", error);
        }

        self.killer.replace(Some(killer));
    }

    pub(crate) fn close_killer(&self, e: &Events) {
        if let Some(timer) = self.killer.take() {
            if let Err(error) = e.set_enabled(timer.clone(), EventState::Off) {
                log::error!("Failed to disable idle worker killer: {}", error);
            }

            if let Err(error) = e.del_source(timer) {
                log::error!("Failed to delete idle worker killer: {}", error);
            }
        };
    }
}

impl Source for GarbageCollect {
    fn fd(&self) -> RawFd {
        0
    }

    fn event_type(&self) -> EventType {
        EventType::Post
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn priority(&self) -> i8 {
        0
    }

    fn dispatch(&self, e: &Events) -> i32 {
        let devmaster = self.devmaster.upgrade().unwrap();
        let job_queue = devmaster.borrow().job_queue.clone().unwrap();
        let worker_manager = devmaster.borrow().worker_manager.clone().unwrap();

        if !job_queue.is_empty() {
            job_queue.job_queue_start(Some(e));
            return 0;
        }

        if !worker_manager.workers.borrow().is_empty() {
            self.close_killer(e);
            self.start_killer(e);
            return 0;
        } else {
            /*
             * Cleaning up idle wokers is asynchronous, thus when the
             * idle worker killer raised, the workers is not cleaned
             * up right away. This will lead to the post event starting
             * another idle worker killer.
             *
             * That is to say, when the worker manager has cleaned up
             * the workers, there is another redundant idle worker killer
             * underground.
             *
             * To avoid the redundant idle worker killer raising, close
             * it explicitly.
             */
            self.close_killer(e);
        }

        let _ = cleanup_prior_dir();

        0
    }
}

/// kill idle workers
pub(crate) struct IdleWorkerKiller {
    /// time interval
    pub(crate) time: u64,

    /// reference to worker manager
    pub(crate) worker_manager: Weak<WorkerManager>,
}

/// internal methods
impl IdleWorkerKiller {
    /// create a timer instance to recycle workers
    pub(crate) fn new(time: u64, worker_manager: Rc<WorkerManager>) -> IdleWorkerKiller {
        IdleWorkerKiller {
            time,
            worker_manager: Rc::downgrade(&worker_manager),
        }
    }
}

impl Source for IdleWorkerKiller {
    /// timer fd is zero
    fn fd(&self) -> RawFd {
        0
    }

    /// timer type
    fn event_type(&self) -> EventType {
        EventType::TimerMonotonic
    }

    /// epoll type
    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    /// priority of timer source
    fn priority(&self) -> i8 {
        0
    }

    /// relative time
    fn time_relative(&self) -> u64 {
        self.time * 1000000
    }

    /// kill workers if job queue keeps empty for an interval
    fn dispatch(&self, _: &Events) -> i32 {
        log::info!("Cleanup idle workers");
        self.worker_manager.upgrade().unwrap().kill_workers();
        0
    }

    /// token of event source
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
