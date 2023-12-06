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

//! worker manager
//!
use crate::{error::*, framework::*, rules::*};
use device::{
    device_monitor::{DeviceMonitor, MonitorNetlinkGroup},
    Device,
};
use event::Source;
use nix::unistd::unlink;
use snafu::ResultExt;
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{self, Display},
    io::{Read, Write},
    os::unix::{
        net::{UnixListener, UnixStream},
        prelude::{AsRawFd, RawFd},
    },
    path::Path,
    rc::{Rc, Weak},
    sync::{mpsc, Arc, RwLock},
    thread::JoinHandle,
};

use super::devmaster::Devmaster;

/// worker manager listen address
pub const WORKER_MANAGER_LISTEN_ADDR: &str = "/run/devmaster/worker";

/// messages sended by manager to workers
pub(crate) enum WorkerMessage {
    Job(Vec<u8>),
    Cmd(String),
}

/// worker manager
pub struct WorkerManager {
    /// max number of workers
    pub(crate) workers_capacity: u32,
    /// container of workers
    pub(crate) workers: RefCell<HashMap<u32, Rc<Worker>>>,
    /// listening socket address
    pub(crate) listen_addr: String,
    /// listening socket
    pub(crate) listener: RefCell<UnixListener>,
    /// reference to job queue
    pub(crate) job_queue: RefCell<Weak<JobQueue>>,

    /// reference to devmaster manager
    pub(crate) devmaster: Weak<RefCell<Devmaster>>,
}

/// worker
#[derive(Debug)]
pub struct Worker {
    /// worker unique id
    id: u32,
    /// channel transfer
    tx: mpsc::Sender<WorkerMessage>,
    /// worker state
    state: RefCell<WorkerState>,
    /// thread handler
    handler: RefCell<Option<JoinHandle<()>>>,
    /// can only bind to unique device job in job queue
    device_job: RefCell<Option<Weak<DeviceJob>>>,
}

/// state of worker
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum WorkerState {
    /// undefined, immediately transfer to idle
    Undef,
    /// idle, wait for a job
    Idle,
    /// running, can only process single device at the same time
    Running,
    /// wait for killing, worker manager will no longer dispatch device job to this worker, waiting for its ack
    Killing,
    /// this worker is already killed, waiting to recycle it from worker manager
    _Killed,
}

impl Display for WorkerState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let state = match self {
            WorkerState::Undef => "Undef",
            WorkerState::Idle => "Idle",
            WorkerState::Running => "Running",
            WorkerState::Killing => "Killing",
            WorkerState::_Killed => "Killed",
        };

        write!(f, "{}", state)
    }
}

/// public methods
impl Worker {
    /// create a new worker, start running the worker thread
    pub(crate) fn new(
        id: u32,
        state: WorkerState,
        listen_addr: String,
        cache: Arc<RwLock<Cache>>,
    ) -> Worker {
        let (tx, rx) = mpsc::channel::<WorkerMessage>();

        // share rules in worker threads. worker should only read rules to avoid lock being poisoned.
        let handler = std::thread::spawn(move || {
            let execute_mgr = exec_mgr::ExecuteManager::new(cache.clone());
            let broadcaster = DeviceMonitor::new(MonitorNetlinkGroup::None, None);

            loop {
                let msg = rx.recv().unwrap_or_else(|error| {
                    log::error!("Worker {}: panic at recv \"{}\"", id, error);
                    panic!();
                });

                match msg {
                    WorkerMessage::Job(dev_nulstr) => {
                        // deserialize the device object from job message
                        let device = Device::from_nulstr(&dev_nulstr).unwrap();

                        log::info!(
                            "Worker {}: received device '{}'",
                            id,
                            device
                                .get_devpath()
                                .context(DeviceSnafu)
                                .log_error("worker received a device without devpath")
                                .unwrap_or_default()
                        );

                        let device = Rc::new(device);
                        let _ = execute_mgr.process_device(device.clone());

                        log::info!("Worker {}: finished job", id);

                        broadcaster.send_device(&device, None).unwrap();

                        let mut tcp_stream = UnixStream::connect(listen_addr.as_str())
                            .unwrap_or_else(|error| {
                                log::error!("Worker {}: failed to connect {}", id, error);
                                panic!();
                            });

                        tcp_stream
                            .write_all(format!("finished {}", id).as_bytes())
                            .unwrap_or_else(|error| {
                                log::error!(
                                    "Worker {}: failed to send ack to manager \"{}\"",
                                    id,
                                    error
                                );
                            });
                    }
                    WorkerMessage::Cmd(cmd) => {
                        log::info!("Worker {} received cmd: {}", id, cmd);
                        match cmd.as_str() {
                            "kill" => {
                                let mut tcp_stream = UnixStream::connect(listen_addr.as_str())
                                    .unwrap_or_else(|error| {
                                        log::error!(
                                            "Worker {}: failed to connect \"{}\"",
                                            id,
                                            error
                                        );
                                        panic!();
                                    });
                                let _ret = tcp_stream
                                .write(format!("killed {}", id).as_bytes())
                                .unwrap_or_else(|error| {
                                    log::error!(
                                        "Worker {}: failed to send killed message to manager \"{}\"", id, error
                                    );
                                    0
                                });
                                log::debug!("Worker {}: is killed", id);
                                break;
                            }
                            _ => {
                                todo!();
                            }
                        }
                    }
                }
            }
        });

        Worker {
            id,
            tx,
            state: RefCell::new(state),
            handler: RefCell::new(Some(handler)),
            device_job: RefCell::new(None),
        }
    }
}

/// internal methods
impl Worker {
    /// get the id of the worker
    pub(crate) fn get_id(&self) -> u32 {
        self.id
    }

    /// get the state of the worker
    pub(crate) fn _get_state(&self) -> WorkerState {
        *self.state.borrow()
    }

    /// send message to the worker thread
    pub(crate) fn worker_send_message(&self, msg: WorkerMessage) {
        self.tx.send(msg).unwrap_or_else(|error| {
            log::error!(
                "Worker Manager: failed to send message to worker {}, {}",
                self.id,
                error
            )
        });
    }

    /// bind a worker to a device job
    pub(crate) fn bind(self: &Rc<Worker>, job: &Rc<DeviceJob>) {
        *self.device_job.borrow_mut() = Some(Rc::downgrade(job));
    }

    /// free the device job
    pub(crate) fn job_free(self: &Rc<Worker>) {
        *self.device_job.borrow_mut() = None;
    }
}

/// public methods
impl WorkerManager {
    /// create a worker manager
    pub fn new(
        workers_capacity: u32,
        listen_addr: String,
        devmaster: Weak<RefCell<Devmaster>>,
    ) -> WorkerManager {
        /*
         * The named socket file will not automatically deleted after last devmaster existed.
         * Thus before bind to it, try to unlink it explicitly.
         */
        if Path::new(listen_addr.as_str()).exists() {
            let _ = unlink(listen_addr.as_str());
        }

        let listener = RefCell::new(UnixListener::bind(listen_addr.as_str()).unwrap_or_else(
            |error| {
                log::error!("Worker Manager: failed to bind listener \"{}\"", error);
                panic!();
            },
        ));

        listener
            .borrow()
            .set_nonblocking(true)
            .expect("Cannot set non-blocking");

        WorkerManager {
            workers_capacity,
            workers: RefCell::new(HashMap::new()),
            listen_addr,
            listener,
            job_queue: RefCell::new(Weak::new()),
            devmaster,
        }
    }

    /// set the reference to a job queue instance
    pub fn set_job_queue(&self, job_queue: &Rc<JobQueue>) {
        *self.job_queue.borrow_mut() = Rc::downgrade(job_queue);
    }
}

/// internal methods
impl WorkerManager {
    /// create a new worker object
    /// clone rules and move it to worker thread
    pub(crate) fn create_new_worker(self: &Rc<WorkerManager>) -> Option<u32> {
        for id in 0..self.workers_capacity {
            if !self.workers.borrow().contains_key(&id) {
                self.workers.borrow_mut().insert(
                    id,
                    Rc::new(Worker::new(
                        id,
                        WorkerState::Undef,
                        self.listen_addr.clone(),
                        self.devmaster
                            .upgrade()
                            .unwrap()
                            .as_ref()
                            .borrow()
                            .get_cache(),
                    )),
                );
                log::debug!("Worker Manager: created new worker {}", id);
                self.set_worker_state(id, WorkerState::Idle);
                return Some(id);
            }
        }

        None
    }

    /// dispatch job to a worker
    pub(crate) fn job_dispatch(
        self: &Rc<WorkerManager>,
        device_job: Rc<DeviceJob>,
    ) -> Result<Rc<Worker>, Error> {
        log::debug!("Worker Manager: start dispatch job {}", device_job.seqnum);

        if *device_job.state.borrow() == JobState::Running {
            log::debug!(
                "Worker Manager: skip job {} as it is running",
                device_job.seqnum
            );
        }

        for (id, worker) in self.workers.borrow().iter() {
            let state = *worker.state.borrow();
            if state == WorkerState::Idle {
                log::debug!("Worker Manager: find idle worker {}", worker.id);
                self.set_worker_state(*id, WorkerState::Running);

                let (dev_nulstr, _len) = device_job
                    .device
                    .get_properties_nulstr()
                    .context(DeviceSnafu)
                    .log_error("failed to get properties nulstr")?;

                worker.worker_send_message(WorkerMessage::Job(dev_nulstr));
                return Ok(worker.clone());
            }
        }

        if (self.workers.borrow().len() as u32) < self.workers_capacity {
            if let Some(id) = self.create_new_worker() {
                let workers = self.workers.borrow();
                let worker = workers.get(&id).unwrap();
                self.set_worker_state(id, WorkerState::Running);

                let (dev_nulstr, _len) = device_job
                    .device
                    .get_properties_nulstr()
                    .context(DeviceSnafu)
                    .log_error("failed to get properties nulstr")?;

                worker.worker_send_message(WorkerMessage::Job(dev_nulstr));

                return Ok(worker.clone());
            }
        }

        Err(Error::WorkerManagerError {
            msg: "failed to get an idle worker for job",
        })
    }

    /// update the state of worker according to the ack
    pub(crate) fn worker_response_dispose(&self, ack: String) {
        let tokens: Vec<&str> = ack.split(' ').collect();

        if tokens.len() != 2 {
            return;
        }

        let (ack_kind, id) = (
            tokens[0],
            tokens[1]
                .parse::<u32>()
                .expect("Worker respond with invalid id"),
        );

        match ack_kind {
            "killed" => {
                // cleanup the killed worker from the manager
                log::debug!("Worker Manager: cleanup worker {}", id);

                self.workers
                    .borrow_mut()
                    .remove(&id)
                    .unwrap()
                    .handler
                    .take()
                    .unwrap()
                    .join()
                    .unwrap();
            }
            "finished" => {
                let job = &self
                    .workers
                    .borrow()
                    .get(&id)
                    .unwrap()
                    .device_job
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .upgrade()
                    .unwrap();

                self.set_worker_state(id, WorkerState::Idle);
                self.job_queue.borrow().upgrade().unwrap().job_free(job);
            }
            _ => {
                todo!();
            }
        }
    }

    /// set the state of the worker
    pub(crate) fn set_worker_state(&self, id: u32, state: WorkerState) {
        log::debug!("Worker Manager: set worker {} to state {}", id, state);
        let workers = self.workers.borrow();
        let worker = workers.get(&id).unwrap();

        *worker.state.borrow_mut() = state;
    }

    /// kill all workers
    pub(crate) fn kill_workers(&self) {
        for (id, worker) in self.workers.borrow().iter() {
            self.set_worker_state(*id, WorkerState::Killing);
            worker.worker_send_message(WorkerMessage::Cmd(String::from("kill")));
        }
    }
}

impl Source for WorkerManager {
    /// tcp listener fd
    fn fd(&self) -> RawFd {
        self.listener.borrow().as_raw_fd()
    }

    /// event type
    fn event_type(&self) -> event::EventType {
        event::EventType::Io
    }

    /// epoll type
    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    /// Set the priority, -127i8 ~ 128i8, the smaller the value, the higher the priority
    fn priority(&self) -> i8 {
        0
    }

    /// start dispatching after the event arrives
    fn dispatch(&self, _: &event::Events) -> i32 {
        let (mut stream, _) = match self.listener.borrow_mut().accept() {
            Ok((s, sa)) => (s, sa),
            Err(e) => {
                // WouldBlock error is expected when a large number of uevents are triggered in a shot interval
                if e.kind() != std::io::ErrorKind::WouldBlock {
                    log::error!("failed to listen worker ack ({:?})", e.kind());
                }
                return 0;
            }
        };
        let mut ack = String::new();
        stream.read_to_string(&mut ack).unwrap();

        log::debug!("Worker Manager: received message \"{}\"", ack);
        self.worker_response_dispose(ack);

        0
    }

    /// Unless you can guarantee all types of token allocation, it is recommended to use the default implementation here
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
