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
use libdevice::device::Device;
use std::cell::RefCell;
use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::collections::VecDeque;
use std::fmt::{self, Display};
use std::rc::{Rc, Weak};

use crate::worker_manager::{Worker, WorkerManager};

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

impl Display for JobState {
    ///
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = match self {
            JobState::Undef => "Undef",
            JobState::Queued => "Queued",
            JobState::Running => "Running",
        };

        write!(f, "{state}")
    }
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
#[derive(Debug)]
pub struct JobQueue {
    /// internal container of jobs
    jobs: RefCell<VecDeque<Rc<DeviceJob>>>,

    /// dispatch jobs to worker manager
    worker_manager: Rc<WorkerManager>,
}

/// public methods
impl JobQueue {
    /// create a job queue
    pub fn new(worker_manager: Rc<WorkerManager>) -> JobQueue {
        JobQueue {
            jobs: RefCell::new(VecDeque::new()),
            worker_manager,
        }
    }
}

/// internal methods
impl JobQueue {
    /// dispatch job to worker manager
    pub(crate) fn job_queue_start(&self) {
        if self.jobs.borrow().is_empty() {
            log::debug!("Job Queue: job queue is empty");
            return;
        }

        // self.job_queue_show_state();

        for job in self.jobs.borrow().iter() {
            match job.get_state() {
                JobState::Queued => {}
                JobState::Running | JobState::Undef => {
                    continue;
                }
            }

            // check whether device is busy
            // todo!()

            match self.worker_manager.job_dispatch(job.clone()) {
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
            Some(seqnum) => seqnum,
            None => {
                log::debug!(
                    "Job Queue: failed to insert device as it is not received from netlink"
                );
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
            log::debug!("Job Queue: failed to insert reduplicated job {seqnum}");
            return;
        }

        // Keep the ordering
        let idx = self.jobs.borrow().partition_point(|x| x < &job);
        self.jobs.borrow_mut().insert(idx, job);

        log::debug!("Job Queue: insert job {seqnum}");
    }

    // /// cleanup the job queue, if match_state is Undef, cleanup all jobs, otherwise just retain the unmatched jobs
    // pub(crate) fn job_queue_cleanup(&self, match_state: JobState) {
    //     self.jobs.borrow_mut().retain_mut(|job| {
    //         if match_state != JobState::Undef && match_state != job.get_state() {
    //             return true;
    //         }

    //         false
    //     });

    //     log::debug!("Job Queue: cleanup");
    // }

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
    }

    // /// show states of each device job in the job queue
    // pub(crate) fn job_queue_show_state(&self) {
    //     for job in self.jobs.borrow().iter() {
    //         log::debug!("{job:?}");
    //     }
    // }
}
