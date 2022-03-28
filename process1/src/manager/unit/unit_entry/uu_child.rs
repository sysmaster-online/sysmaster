use std::collections::HashSet;

#[derive(Debug)]
pub(super) struct UeChild {
    pids: HashSet<u64>,
    sigchldgen: u64,
}

impl UeChild {
    pub(super) fn new() -> UeChild {
        UeChild {
            pids: HashSet::<u64>::new(),
            sigchldgen: 0,
        }
    }

    pub(super) fn addPids(&mut self, pid: u64) -> bool {
        self.pids.insert(pid)
    }

    pub(super) fn removePids(&mut self, pid: u64) -> bool {
        self.pids.remove(&pid)
    }
}
