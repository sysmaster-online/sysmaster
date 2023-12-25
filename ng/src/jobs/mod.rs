use once_cell::sync::Lazy;
use std::collections::VecDeque;
use tokio::sync::Mutex;

#[derive(Clone, Copy)]
pub struct Job {
    _id: u32,
}

impl Job {
    pub fn new(_id: u32) -> Job {
        Job { _id }
    }
}

pub type Jobs = Vec<Job>;
pub static _JOBS: Lazy<Mutex<VecDeque<Job>>> = Lazy::new(|| Mutex::new(VecDeque::new()));
