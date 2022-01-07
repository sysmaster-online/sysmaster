use super::Source;
use crate::poll::Poll;
use std::{
    collections::{BinaryHeap, HashMap},
    rc::Rc, cell::RefCell,
};

pub struct Events {
    poller: Rc<RefCell<Poll>>,
    exit: bool,
    sources: HashMap<u64, Rc<dyn Source>>,
    pending: BinaryHeap<Rc<dyn Source>>,
}

impl Events {
    pub fn new() -> Self {
        Self {
            poller: Rc::new(RefCell::new(Poll::new().unwrap())),
            exit: false,
            sources: HashMap::new(),
            pending: BinaryHeap::new(),
        }
    }

    pub fn add_source(&mut self, s: Rc<dyn Source>) {
        s.register(self.poller.clone());
        self.sources.insert(s.token(), s);
    }

    /// Wait for the event event through poller
    /// And add the corresponding events to the pengding queue
    pub fn wait(&mut self, timeout: i32) -> bool {
        let events = {
            loop {
                let result = self.poller.borrow().poll(timeout);

                match result {
                    Ok(events) => break events,
                    Err(_err) => return false,
                };
            }
        };

        for event in events.iter() {
            #[allow(unaligned_references)]
            let token = &event.u64;
            let s = self.sources.get(&token).unwrap();
            self.pending.push(s.clone());
        }

        true
    }

    /// Wait for the event event through poller
    /// And add the corresponding events to the pengding queue
    pub fn prepare(&mut self) -> bool {
        false
    }

    /// Fetch the highest priority event processing on the pending queue
    fn dispatch(&mut self) {
        if let Some(first) = self.pending.peek() {
            first.as_ref().dispatch();
            self.pending.pop();
        }
    }

    /// Scheduling once, processing an event
    pub fn run(&mut self, timeout: i32) {
        if self.prepare() || self.pending.is_empty() {
            self.wait(timeout);
        }
        self.dispatch();
    }

    /// Process the event in a loop until exiting actively
    pub fn rloop(&mut self) {
        loop {
            if self.exit == true {
                break;
            }
            self.run(-1i32);
        }
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }

    pub fn now() {
        todo!();
    }
}

impl Default for Events {
    fn default() -> Self {
        Self::new()
    }
}
