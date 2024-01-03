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

//! job queue
//!
use crate::{error::*, framework::*};
use basic::fs::touch_file;
use device::device::Device;
use event::Events;
use nix::unistd::unlink;
use snafu::ResultExt;
use std::{
    cell::RefCell,
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd},
    collections::VecDeque,
    rc::{Rc, Weak},
};

/// state of device job
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum JobState {
    /// initial state of device job
    Undef,
    /// the device job is in job queue waiting for dispatching
    Queued,
    /// the device job is being processed by a worker
    Running,
}

/// device job
#[derive(Debug)]
pub struct DeviceJob {
    /// internal device
    pub device: Device,
    /// current state
    pub state: RefCell<JobState>,
    /// if the device is received from monitor, its seqnum should be greater than zero and unique
    pub seqnum: u64,

    /// reference to worker if the device job is being processed
    pub worker: RefCell<Option<Weak<Worker>>>,
}

/// public methods
impl DeviceJob {
    /// create a device job
    pub fn new(device: Device, state: JobState, seqnum: u64) -> DeviceJob {
        DeviceJob {
            device,
            state: RefCell::new(state),
            seqnum,
            worker: RefCell::new(None),
        }
    }
}

/// internal methods
impl DeviceJob {
    /// bind a device job to a unique worker
    pub(crate) fn bind(self: &Rc<DeviceJob>, worker: &Rc<Worker>) {
        *self.worker.borrow_mut() = Some(Rc::downgrade(worker));
        worker.bind(self);
    }

    /// free a device job
    pub(crate) fn job_free(self: &Rc<DeviceJob>) {
        self.worker
            .borrow()
            .as_ref()
            .unwrap()
            .upgrade()
            .unwrap()
            .job_free();
        *self.worker.borrow_mut() = None;
    }

    /// get the state of device job
    pub(crate) fn get_state(&self) -> JobState {
        *self.state.borrow()
    }

    /// set state of device job
    pub(crate) fn set_state(&self, state: JobState) {
        *self.state.borrow_mut() = state;
    }
}

impl Ord for DeviceJob {
    ///
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.seqnum.cmp(&other.seqnum)
    }
}

impl PartialOrd for DeviceJob {
    ///
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for DeviceJob {}

impl PartialEq for DeviceJob {
    ///
    fn eq(&self, other: &Self) -> bool {
        self.seqnum == other.seqnum
    }
}

/// job queue
pub struct JobQueue {
    /// internal container of jobs
    pub(crate) jobs: RefCell<VecDeque<Rc<DeviceJob>>>,

    /// dispatch jobs to worker manager
    pub(crate) devmaster: Weak<RefCell<Devmaster>>,
}

/// public methods
impl JobQueue {
    /// create a job queue
    pub fn new(devmaster: Rc<RefCell<Devmaster>>) -> JobQueue {
        JobQueue {
            jobs: RefCell::new(VecDeque::new()),
            devmaster: Rc::downgrade(&devmaster),
        }
    }
}

/// internal methods
impl JobQueue {
    /// Dispatch job to worker manager.
    /// If events is not none, close the idle worker killer.
    pub(crate) fn job_queue_start(&self, e: Option<&Events>) {
        if self.jobs.borrow().is_empty() {
            log::debug!("Job Queue: job queue is empty");
            return;
        }

        let devmaster = self.devmaster.upgrade().unwrap();
        let worker_manager = devmaster.borrow().worker_manager.clone().unwrap();

        if let Some(e) = e {
            let gc = devmaster.borrow().gc.clone().unwrap();
            gc.close_killer(e);
        }

        for job in self.jobs.borrow().iter() {
            match job.get_state() {
                JobState::Queued => {}
                JobState::Running | JobState::Undef => {
                    continue;
                }
            }

            // check whether device is busy
            // todo!()

            match worker_manager.job_dispatch(job.clone()) {
                Ok(worker) => {
                    job.set_state(JobState::Running);
                    job.bind(&worker);
                    log::debug!(
                        "Job Queue: dispatch job {} to worker {}",
                        job.seqnum,
                        worker.get_id(),
                    );
                }
                Err(error) => {
                    log::debug!("{}", error);
                    return;
                }
            }
        }
    }

    /// encapsulate device into a device job and insert it into job queue
    pub(crate) fn job_queue_insert(&self, device: Device) {
        let seqnum: u64 = match device.get_seqnum() {
            Ok(seqnum) => seqnum,
            Err(_) => {
                log::debug!("Job Queue: failed to insert device as it is not received from uevent");
                return;
            }
        };

        let job = Rc::new(DeviceJob::new(device, JobState::Queued, seqnum));

        // create /run/devmaster/queue
        // todo!()

        // Prevent insertion of devices with the same seqnum
        if self
            .jobs
            .borrow()
            .binary_search_by(|x| x.seqnum.cmp(&seqnum))
            .is_ok()
        {
            log::debug!("Job Queue: failed to insert reduplicated job {}", seqnum);
            return;
        }

        if self.jobs.borrow().is_empty()
            && touch_file("/run/devmaster/queue", true, None, None, None)
                .context(BasicSnafu)
                .log_error("Failed to touch /run/devmaster/queue, stop inserting jobs")
                .is_err()
        {
            return;
        }

        // Keep the ordering
        let idx = self.jobs.borrow().partition_point(|x| x < &job);
        self.jobs.borrow_mut().insert(idx, job);

        log::debug!("Job Queue: insert job {}", seqnum);
    }

    /// free a job from job queue
    pub(crate) fn job_free(&self, job: &Rc<DeviceJob>) {
        job.job_free();

        let idx = match self.jobs.borrow().binary_search(job) {
            Ok(idx) => idx,
            Err(_) => {
                log::debug!("Job Queue: failed to find job {}", job.seqnum);
                return;
            }
        };

        match self.jobs.borrow_mut().remove(idx) {
            Some(job) => {
                log::debug!("Job Queue: succeeded to free job {}", job.seqnum);
            }
            None => {
                log::debug!("Job Queue: failed to free job {}", job.seqnum);
            }
        }

        if self.jobs.borrow().is_empty()
            && unlink("/run/devmaster/queue")
                .context(NixSnafu)
                .log_error("Failed to unlink /run/devmaster/queue")
                .is_ok()
        {
            log::debug!("Job queue is empty, removing /run/devmaster/queue");
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.jobs.borrow().is_empty()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_job_cmp() {
        let j1 = DeviceJob::new(
            Device::from_subsystem_sysname("net", "lo").unwrap(),
            JobState::Queued,
            1000,
        );

        let j2 = DeviceJob::new(
            Device::from_subsystem_sysname("net", "lo").unwrap(),
            JobState::Queued,
            1000,
        );

        assert_eq!(j1, j2);
    }

    #[test]
    fn test_job_queue() {
        let events = Rc::new(Events::new().unwrap());
        let cache = Cache::new(vec![], vec![]);
        let devmaster = Rc::new(RefCell::new(Devmaster {
            events,
            worker_manager: None,
            control_manager: None,
            monitor: None,
            job_queue: None,
            gc: None,
            cache: Arc::new(RwLock::new(cache)),
        }));

        let job_queue = JobQueue::new(devmaster);

        /* Test start job when the queue is empty. */
        job_queue.job_queue_start(None);

        let dev = Device::from_subsystem_sysname("net", "lo").unwrap();
        job_queue.job_queue_insert(dev);

        /* Test insert dulplicate jobs with the same seqnum. */
        let dev = Device::from_subsystem_sysname("net", "lo").unwrap();
        dev.set_seqnum_from_string("1000").unwrap();
        job_queue.job_queue_insert(dev);

        let dev = Device::from_subsystem_sysname("net", "lo").unwrap();
        dev.set_seqnum_from_string("1000").unwrap();
        job_queue.job_queue_insert(dev);
    }
}
