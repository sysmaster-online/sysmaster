use bitflags::bitflags;
use std::{cell::RefCell, collections::HashMap, ffi::CString, path::PathBuf, rc::Rc};

#[derive(PartialEq, Clone, Eq, Debug)]
pub struct ExecCommand {
    path: String,
    argv: Vec<String>,
}

impl ExecCommand {
    pub fn new(path: String, argv: Vec<String>) -> ExecCommand {
        ExecCommand { path, argv }
    }

    pub fn path(&self) -> &String {
        &self.path
    }

    pub fn argv(&self) -> Vec<&String> {
        self.argv.iter().map(|argr| argr).collect::<Vec<_>>()
    }
}

#[derive(Debug)]
pub enum ExecCmdError {
    Timeout,
    NoCmdFound,
    SpawnError,
    CgroupError(String),
}

pub struct ExecContext {
    envs: RefCell<HashMap<String, String>>,
}

impl ExecContext {
    pub fn new() -> ExecContext {
        ExecContext {
            envs: RefCell::new(HashMap::new()),
        }
    }

    pub fn insert_env(&self, key: String, value: String) {
        self.envs.borrow_mut().insert(key, value);
    }

    pub fn envs(&self) -> HashMap<String, String> {
        let mut tmp = HashMap::new();

        for (key, value) in &*self.envs.borrow() {
            tmp.insert(key.to_string(), value.to_string());
        }
        tmp
    }
}

pub struct ExecParameters {
    environment: Rc<EnvData>,
    fds: Vec<i32>,
    notify_sock: Option<PathBuf>,
}

struct EnvData {
    env: RefCell<HashMap<String, String>>,
}

impl EnvData {
    fn new() -> EnvData {
        EnvData {
            env: RefCell::new(HashMap::new()),
        }
    }

    fn add_env(&self, key: &str, value: String) {
        self.env.borrow_mut().insert(key.to_string(), value);
    }

    fn get(&self, key: &str) -> Option<String> {
        self.env.borrow().get(key).map(|s| s.to_string())
    }

    fn envs(&self) -> Vec<CString> {
        let mut envs = Vec::new();

        for (key, value) in &*self.env.borrow() {
            envs.push(std::ffi::CString::new(format!("{}={}", key, value)).unwrap());
        }

        envs
    }
}

impl ExecParameters {
    pub fn new() -> ExecParameters {
        ExecParameters {
            environment: Rc::new(EnvData::new()),
            fds: Vec::new(),
            notify_sock: None,
        }
    }

    pub fn add_env(&self, key: &str, value: String) {
        self.environment.add_env(key, value);
    }

    pub fn get_env(&self, key: &str) -> Option<String> {
        self.environment.get(key)
    }

    pub fn envs(&self) -> Vec<CString> {
        self.environment.envs()
    }

    pub fn insert_fds(&mut self, fds: Vec<i32>) {
        self.fds = fds
    }

    pub fn fds(&self) -> Vec<i32> {
        self.fds.iter().map(|v| *v).collect()
    }

    pub fn set_notify_sock(&mut self, notify_sock: PathBuf) {
        self.notify_sock = Some(notify_sock)
    }
}

bitflags! {
    pub struct ExecFlags: u16 {
        const APPLY_SANDBOX = 1 << 0;
        const CONTROL = 1 << 1;

        const PASS_FDS = 1 << 2;
    }
}
