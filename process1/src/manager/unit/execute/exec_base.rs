use std::{cell::RefCell, collections::HashMap, rc::Rc};

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

pub enum ExecCmdError {
    Timeout,
    NoCmdFound,
    SpawnError,
}

pub struct ExecContext {
    env: Vec<String>,
}

pub struct ExecParameters {
    environment: Rc<EnvData>,
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
}

impl ExecParameters {
    pub fn new() -> ExecParameters {
        ExecParameters {
            environment: Rc::new(EnvData::new()),
        }
    }

    pub fn add_env(&self, key: &str, value: String) {
        self.environment.add_env(key, value);
    }

    pub fn get_env(&self, key: &str) -> Option<String> {
        self.environment.get(key)
    }
}
