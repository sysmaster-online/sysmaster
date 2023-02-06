//! devmaster daemon
use libdevmaster::*;
use libevent::{EventState, Events};
use libutils::logger::*;
use log::LevelFilter;
use std::rc::Rc;

fn main() {
    init_log_with_console("devmaster", LevelFilter::Info);
    log::info!("daemon start");

    let events = Rc::new(Events::new().unwrap());

    let worker_manager = Rc::new(WorkerManager::new(
        3,
        String::from(WORKER_MANAGER_LISTEN_ADDR),
        events.clone(),
    ));

    worker_manager.set_kill_workers_timer();
    events
        .add_source(worker_manager.get_kill_workers_timer().unwrap())
        .unwrap();

    events.add_source(worker_manager.clone()).unwrap();
    events
        .set_enabled(worker_manager.clone(), EventState::On)
        .unwrap();

    let job_queue = Rc::new(JobQueue::new(worker_manager.clone()));
    worker_manager.set_job_queue(&job_queue);

    let control_manager = Rc::new(ControlManager::new(
        String::from(CONTROL_MANAGER_LISTEN_ADDR),
        worker_manager.clone(),
        job_queue.clone(),
    ));
    events.add_source(control_manager.clone()).unwrap();
    events
        .set_enabled(control_manager.clone(), EventState::On)
        .unwrap();

    let monitor = Rc::new(Monitor::new(job_queue));
    monitor.set_receive_buffer_force(1024 * 1024 * 128);
    events.add_source(monitor.clone()).unwrap();
    events.set_enabled(monitor.clone(), EventState::On).unwrap();

    events.rloop().unwrap();

    events.del_source(worker_manager).unwrap();
    events.del_source(control_manager).unwrap();
    events.del_source(monitor).unwrap();
}
