use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

mod exec;
mod exec_child;

#[derive(PartialEq, Clone, Eq, Debug)]
pub struct CommandLine {
    pub cmd: String,
    pub args: Vec<String>,
    pub next: Option<Rc<RefCell<CommandLine>>>,
}

impl CommandLine {
    pub fn update_next(&mut self, next: Rc<RefCell<CommandLine>>) {
        self.next = Some(next)
    }
}

impl fmt::Display for CommandLine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Display: {}", self.cmd)
    }
}

#[allow(dead_code)]
pub enum CmdError {
    Timeout,
    NoCmdFound,
    SpawnError,
}

#[allow(dead_code)]
pub struct ExecContext {
    env: Vec<String>,
}

pub struct ExecParameters {
    env_data: Rc<EnvData>,
}

struct EnvData {
    env: RefCell<HashMap<String, String>>,
}

impl EnvData {
    pub fn new() -> EnvData {
        EnvData {
            env: RefCell::new(HashMap::new()),
        }
    }

    pub fn add_env(&self, key: &str, value: String) {
        self.env.borrow_mut().insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.env.borrow().get(key).map(|s| s.to_string())
    }
}

impl ExecParameters {
    pub fn new() -> ExecParameters {
        ExecParameters {
            env_data: Rc::new(EnvData::new()),
        }
    }

    pub fn add_env(&self, key: &str, value: String) {
        self.env_data.add_env(key, value);
    }

    pub fn get_env(&self, key: &str) -> Option<String> {
        self.env_data.get(key)
    }
}

pub use exec::ExecSpawn;
