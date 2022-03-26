use std::collections::{HashSet};


#[derive(Debug)]
pub struct UeChild {
    pids: HashSet<u64>,
    sigchldgen: u64,
}

impl UeChild {
    pub fn new() -> UeChild {
        UeChild {
            pids: HashSet::<u64>::new(),
            sigchldgen: 0,
        }
    }

    pub fn addPids(&mut self, pid:u64) -> bool{
        self.pids.insert(pid)
    }

    pub fn removePids(&mut self, pid:u64) -> bool{
        self.pids.remove(&pid)
    }
}
