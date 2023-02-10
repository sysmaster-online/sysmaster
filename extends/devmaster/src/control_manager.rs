//! control manager
//!
use crate::job_queue::JobQueue;
use crate::worker_manager::WorkerManager;
use libdevice::Device;
use libevent::*;
use std::time::SystemTime;
use std::{
    cell::RefCell,
    io::Read,
    net::TcpListener,
    os::unix::prelude::{AsRawFd, RawFd},
    rc::Rc,
};

/// listening address for control manager
pub const CONTROL_MANAGER_LISTEN_ADDR: &str = "0.0.0.0:1224";

/// control manager
pub struct ControlManager {
    /// listener for devctl messages
    listener: RefCell<TcpListener>,

    /// reference to worker manager
    worker_manager: Rc<WorkerManager>,
    /// reference to job queue
    job_queue: Rc<JobQueue>,
    // events: Rc<Events>,
}

impl ControlManager {
    /// create a control manager instance
    pub fn new(
        listen_addr: String,
        worker_manager: Rc<WorkerManager>,
        job_queue: Rc<JobQueue>,
    ) -> ControlManager {
        ControlManager {
            listener: RefCell::new(TcpListener::bind(listen_addr).unwrap()),
            worker_manager,
            job_queue,
        }
    }

    /// process command from devctl
    pub fn cmd_process(&self, cmd: String) {
        let tokens: Vec<&str> = cmd.split(' ').collect();

        let (cmd_kind, devname) = (tokens[0], tokens[1]);

        match cmd_kind {
            "test" => {
                let seqnum = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    % 1000;

                let mut device = Device::new();
                device.devname = devname.to_string();
                device.seqnum = Some(seqnum);

                self.job_queue.job_queue_insert(device);
                self.job_queue.job_queue_start();
            }
            "kill" => {
                self.worker_manager.clone().start_kill_workers_timer();
            }
            _ => {
                todo!();
            }
        }
    }
}

impl Source for ControlManager {
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
        -50
    }

    /// start dispatching after the event arrives
    fn dispatch(&self, _: &libevent::Events) -> Result<i32, libevent::Error> {
        let (mut stream, _) = self.listener.borrow_mut().accept().unwrap();
        let mut cmd = String::new();
        stream.read_to_string(&mut cmd).unwrap();

        log::debug!("Control Manager: received message \"{cmd}\"");

        self.cmd_process(cmd);

        Ok(0)
    }

    /// Unless you can guarantee all types of token allocation, it is recommended to use the default implementation here
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
