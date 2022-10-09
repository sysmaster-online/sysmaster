use std::{cell::RefCell, path::PathBuf};

pub struct ManagerConfig {
    data: RefCell<ManagerConfigData>,
}

impl ManagerConfig {
    pub(super) fn new() -> ManagerConfig {
        ManagerConfig {
            data: RefCell::new(ManagerConfigData::new()),
        }
    }

    pub(super) fn set_notify_sock(&self, socket: PathBuf) {
        self.data.borrow_mut().set_notify_sock(socket)
    }

    pub(super) fn notify_sock(&self) -> Option<PathBuf> {
        self.data.borrow().notify_sock()
    }
}

pub(self) struct ManagerConfigData {
    notify_sock: Option<PathBuf>,
}

impl ManagerConfigData {
    fn new() -> ManagerConfigData {
        ManagerConfigData { notify_sock: None }
    }

    pub(self) fn set_notify_sock(&mut self, socket: PathBuf) {
        self.notify_sock = Some(socket);
    }

    pub(super) fn notify_sock(&self) -> Option<PathBuf> {
        self.notify_sock.as_ref().cloned()
    }
}
