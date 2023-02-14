//! worker manager
//!
use crate::error::Error;
use crate::job_queue::{DeviceJob, JobState};
use crate::JobQueue;
use libdevice::{Device, DeviceMonitor, MonitorNetlinkGroup};
use libevent::{EventState, EventType, Events, Source};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::ops::DerefMut;
use std::os::unix::prelude::{AsRawFd, RawFd};
use std::rc::{Rc, Weak};
use std::sync::mpsc;
use std::thread::JoinHandle;

/// worker manager listen address
pub const WORKER_MANAGER_LISTEN_ADDR: &str = "0.0.0.0:1223";
/// max time interval for idle worker
const WORKER_MAX_IDLE_INTERVAL: u64 = 1;

/// messages sended by manager to workers
pub(crate) enum WorkerMessage {
    Job(Box<Device>),
    Cmd(String),
}

/// worker manager
#[derive(Debug)]
pub struct WorkerManager {
    // events: Rc<libevent::Events>,
    workers_capacity: u32,
    workers: RefCell<HashMap<u32, Rc<Worker>>>,
    listen_addr: String,
    listener: RefCell<TcpListener>,

    kill_idle_workers: RefCell<Option<Rc<WorkerManagerKillWorkers>>>,

    job_queue: RefCell<Weak<JobQueue>>,
    events: Rc<Events>,
}

/// worker
#[derive(Debug)]
pub struct Worker {
    id: u32,
    tx: mpsc::Sender<WorkerMessage>,
    state: RefCell<WorkerState>,
    handler: RefCell<Option<JoinHandle<()>>>,

    device_job: RefCell<Option<Weak<DeviceJob>>>,
}

/// state of worker
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum WorkerState {
    Undef,
    Idle,
    Running,
    Killing, // no longer dispatch device job to this worker, waiting for its ack
    _Killed, // this worker is dead, waiting to recycle it from worker manager
}

impl Display for WorkerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = match self {
            WorkerState::Undef => "Undef",
            WorkerState::Idle => "Idle",
            WorkerState::Running => "Running",
            WorkerState::Killing => "Killing",
            WorkerState::_Killed => "Killed",
        };

        write!(f, "{state}")
    }
}

impl Worker {
    fn new(id: u32, state: WorkerState, tcp_address: String) -> Worker {
        let (tx, rx) = mpsc::channel::<WorkerMessage>();

        let handler = std::thread::spawn(move || loop {
            let msg = rx.recv().unwrap_or_else(|error| {
                log::error!("Worker {id}: panic at recv \"{error}\"");
                panic!();
            });

            let broadcaster = DeviceMonitor::new(MonitorNetlinkGroup::None, None);

            match msg {
                WorkerMessage::Job(device) => {
                    log::info!("Worker {id}: received device {}", device.devpath);

                    Self::worker_process_device(id, device.as_ref());

                    log::info!("Worker {id}: finished job");

                    broadcaster.send_device(device.as_ref(), None);

                    let mut tcp_stream =
                        TcpStream::connect(tcp_address.as_str()).unwrap_or_else(|error| {
                            log::error!("Worker {id}: failed to connect {error}");
                            panic!();
                        });

                    tcp_stream
                        .write_all(format!("finished {id}").as_bytes())
                        .unwrap_or_else(|error| {
                            log::error!("Worker {id}: failed to send ack to manager \"{error}\"");
                        });
                }
                WorkerMessage::Cmd(cmd) => {
                    log::info!("Worker {id} received cmd: {cmd}");
                    match cmd.as_str() {
                        "kill" => {
                            let mut tcp_stream = TcpStream::connect(tcp_address.as_str())
                                .unwrap_or_else(|error| {
                                    log::error!("Worker {id}: failed to connect \"{error}\"");
                                    panic!();
                                });
                            let _ret = tcp_stream
                                .write(format!("killed {id}").as_bytes())
                                .unwrap_or_else(|error| {
                                    log::error!(
                                        "Worker {id}: failed to send killed message to manager \"{error}\""
                                    );
                                    0
                                });
                            log::debug!("Worker {id}: is killed");
                            break;
                        }
                        _ => {
                            todo!();
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

    /// get the id of the worker
    pub(crate) fn get_id(&self) -> u32 {
        self.id
    }

    /// get the state of the worker
    pub(crate) fn _get_state(&self) -> WorkerState {
        *self.state.borrow()
    }

    /// process a device
    fn worker_process_device(id: u32, device: &Device) {
        log::info!("Worker {id}: processing {}", device.devpath);
    }

    /// send message to the worker thread
    fn worker_send_message(&self, msg: WorkerMessage) {
        self.tx.send(msg).unwrap_or_else(|error| {
            log::error!(
                "Worker Manager: failed to send message to worker {}, {error}",
                self.id
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

impl WorkerManager {
    ///
    pub fn new(workers_capacity: u32, listen_addr: String, events: Rc<Events>) -> WorkerManager {
        let listener = RefCell::new(TcpListener::bind(listen_addr.as_str()).unwrap_or_else(
            |error| {
                log::error!("Worker Manager: failed to bind listener \"{error}\"");
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
            kill_idle_workers: RefCell::new(None),
            job_queue: RefCell::new(Weak::new()),
            events,
        }
    }

    /// set the libevent source instance of kill workers timer
    pub fn set_kill_workers_timer(self: &Rc<WorkerManager>) {
        *self.kill_idle_workers.borrow_mut() = Some(Rc::new(WorkerManagerKillWorkers::new(
            WORKER_MAX_IDLE_INTERVAL,
            self.clone(),
        )));
    }

    /// get the libevent source instance of kill workers timer
    pub fn get_kill_workers_timer(
        self: &Rc<WorkerManager>,
    ) -> Option<Rc<WorkerManagerKillWorkers>> {
        if let Some(source) = self.kill_idle_workers.borrow().as_ref() {
            return Some(source.clone());
        }

        None
    }

    /// set the reference to a job queue instance
    pub fn set_job_queue(&self, job_queue: &Rc<JobQueue>) {
        *self.job_queue.borrow_mut() = Rc::downgrade(job_queue);
    }

    /// create a new worker
    pub(crate) fn create_new_worker(self: &Rc<WorkerManager>) -> Option<u32> {
        for id in 0..self.workers_capacity {
            if !self.workers.borrow().contains_key(&id) {
                self.workers.borrow_mut().insert(
                    id,
                    Rc::new(Worker::new(
                        id,
                        WorkerState::Undef,
                        self.listen_addr.clone(),
                    )),
                );
                log::debug!("Worker Manager: created new worker {id}");
                self.set_worker_state(id, WorkerState::Idle);
                return Some(id);
            }
        }

        None
    }

    /// dispatch job to a worker
    pub fn job_dispatch(
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
                worker.worker_send_message(WorkerMessage::Job(Box::new(device_job.device.clone())));
                return Ok(worker.clone());
            }
        }

        if (self.workers.borrow().len() as u32) < self.workers_capacity {
            if let Some(id) = self.create_new_worker() {
                let workers = self.workers.borrow();
                let worker = workers.get(&id).unwrap();
                self.set_worker_state(id, WorkerState::Running);
                worker.worker_send_message(WorkerMessage::Job(Box::new(device_job.device.clone())));
                return Ok(worker.clone());
            }
        }

        Err(Error::WorkerManagerError {
            msg: "failed to get an idle worker for job",
        })
    }

    /// update the state of worker according to the ack
    pub fn worker_response_dispose(&self, ack: String) {
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
                log::debug!("Worker Manager: cleanup worker {id}");

                self.workers
                    .borrow_mut()
                    .deref_mut()
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

                self.job_queue.borrow().upgrade().unwrap().job_queue_start();
            }
            _ => {
                todo!();
            }
        }
    }

    /// set the state of the worker
    fn set_worker_state(&self, id: u32, state: WorkerState) {
        log::debug!("Worker Manager: set worker {id} to state {}", state);
        let workers = self.workers.borrow();
        let worker = workers.get(&id).unwrap();

        *worker.state.borrow_mut() = state;
    }

    /// kill all workers
    fn manager_kill_workers(&self) {
        for (id, worker) in self.workers.borrow().iter() {
            self.set_worker_state(*id, WorkerState::Killing);
            worker.worker_send_message(WorkerMessage::Cmd(String::from("kill")));
        }
    }

    /// start kill workers timer
    pub fn start_kill_workers_timer(self: &Rc<WorkerManager>) {
        self.events
            .set_enabled(self.get_kill_workers_timer().unwrap(), EventState::Off)
            .unwrap();
        self.events
            .set_enabled(self.get_kill_workers_timer().unwrap(), EventState::OneShot)
            .unwrap();
    }

    /// stop kill workers timer
    pub fn stop_kill_workers_timer(self: &Rc<WorkerManager>) {
        self.events
            .set_enabled(self.get_kill_workers_timer().unwrap(), EventState::Off)
            .unwrap();
    }
}

impl Source for WorkerManager {
    /// tcp listener fd
    fn fd(&self) -> RawFd {
        self.listener.borrow().as_raw_fd()
    }

    /// event type
    fn event_type(&self) -> libevent::EventType {
        libevent::EventType::Io
    }

    /// epoll type
    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    /// Set the priority, -127i8 ~ 128i8, the smaller the value, the higher the priority
    fn priority(&self) -> i8 {
        10
    }

    /// start dispatching after the event arrives
    fn dispatch(&self, _: &libevent::Events) -> Result<i32, libevent::Error> {
        let (mut stream, _) = match self.listener.borrow_mut().accept() {
            Ok((s, sa)) => (s, sa),
            Err(e) => {
                // WouldBlock error is expected when a large number of uevents are triggered in a shot interval
                if e.kind() != std::io::ErrorKind::WouldBlock {
                    log::error!("failed to listen worker ack ({})", e.kind());
                }
                return Ok(0);
            }
        };
        let mut ack = String::new();
        stream.read_to_string(&mut ack).unwrap();

        log::debug!("Worker Manager: received message \"{ack}\"");
        self.worker_response_dispose(ack);

        Ok(0)
    }

    /// Unless you can guarantee all types of token allocation, it is recommended to use the default implementation here
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

/// libevent source to kill workers
#[derive(Debug)]
pub struct WorkerManagerKillWorkers {
    /// time interval
    time: u64,

    /// reference to worker manager
    worker_manager: Weak<WorkerManager>,
}

impl WorkerManagerKillWorkers {
    /// create a timer instance to recycle workers
    fn new(time: u64, worker_manager: Rc<WorkerManager>) -> WorkerManagerKillWorkers {
        WorkerManagerKillWorkers {
            time,
            worker_manager: Rc::downgrade(&worker_manager),
        }
    }
}

impl Source for WorkerManagerKillWorkers {
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
        -55
    }

    /// relative time
    fn time_relative(&self) -> u64 {
        self.time * 1000000
    }

    /// kill workers if job queue keeps empty for an interval
    fn dispatch(&self, _: &Events) -> Result<i32, libevent::Error> {
        log::info!("Worker Manager Kill Workers timeout!");
        self.worker_manager
            .upgrade()
            .unwrap()
            .manager_kill_workers();
        Ok(0)
    }

    /// token of event source
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
