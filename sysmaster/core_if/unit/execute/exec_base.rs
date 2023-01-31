use bitflags::bitflags;
use std::{cell::RefCell, collections::HashMap, error::Error, ffi::CString, path::PathBuf, rc::Rc};

/// the error
#[derive(Debug)]
pub enum ExecCmdError {
    /// exec command error for timeout
    Timeout,
    /// exec error for not found command
    NoCmdFound,
    /// exec error for fork child error
    SpawnError,
    /// exec error for create cgroup error
    CgroupError(String),
}

/// the exec context that was parse from the unit file.
/// like parsed from Environment field.
pub struct ExecContext {
    envs: RefCell<HashMap<String, String>>,
}

impl Default for ExecContext {
    fn default() -> Self {
        ExecContext::new()
    }
}

impl ExecContext {
    /// create a new instance of exec context
    pub fn new() -> ExecContext {
        ExecContext {
            envs: RefCell::new(HashMap::new()),
        }
    }

    /// insert to the context with key and value
    pub fn insert_env(&self, key: String, value: String) {
        self.envs.borrow_mut().insert(key, value);
    }

    /// return all the environment with hashMap
    pub fn envs(&self) -> Vec<(String, String)> {
        let mut tmp = Vec::new();

        for (key, value) in &*self.envs.borrow() {
            tmp.push((key.to_string(), value.to_string()));
        }
        tmp
    }
}

/// the environment that will be set when start a new command
pub struct ExecParameters {
    environment: Rc<EnvData>,
    fds: Vec<i32>,
    notify_sock: Option<PathBuf>,
    working_directory: Option<PathBuf>,
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
            envs.push(std::ffi::CString::new(format!("{key}={value}")).unwrap());
        }

        envs
    }
}

impl Default for ExecParameters {
    fn default() -> Self {
        ExecParameters::new()
    }
}

impl ExecParameters {
    /// create  a new instance of ExecParameters
    pub fn new() -> ExecParameters {
        ExecParameters {
            environment: Rc::new(EnvData::new()),
            fds: Vec::new(),
            notify_sock: None,
            working_directory: None,
        }
    }

    /// add a new environment with key and value
    pub fn add_env(&self, key: &str, value: String) {
        self.environment.add_env(key, value);
    }

    /// return the value correspond to the key
    pub fn get_env(&self, key: &str) -> Option<String> {
        self.environment.get(key)
    }

    /// return all environments that will be passed to child
    pub fn envs(&self) -> Vec<CString> {
        self.environment.envs()
    }

    /// insert fds that will be passed to child
    pub fn insert_fds(&mut self, fds: Vec<i32>) {
        self.fds = fds
    }

    /// return all the fds that will be passed to child
    pub fn fds(&self) -> Vec<i32> {
        self.fds.to_vec()
    }

    /// set the NOTIFY_SOCKET value
    pub fn set_notify_sock(&mut self, notify_sock: PathBuf) {
        self.notify_sock = Some(notify_sock)
    }

    /// add WorkingDirectory
    pub fn add_working_directory(
        &mut self,
        working_directory_str: String,
    ) -> Result<(), Box<dyn Error>> {
        if working_directory_str.is_empty() {
            return Ok(());
        }

        let mut miss_ok = false;
        if working_directory_str.starts_with('-') {
            miss_ok = true;
        }

        let mut working_directory_str = working_directory_str.trim_start_matches('-').to_string();

        if working_directory_str == *"~".to_string() {
            working_directory_str = match std::env::var("HOME") {
                Err(e) => {
                    return Err(Box::new(e));
                }
                Ok(v) => v,
            };
        }

        let working_directory = PathBuf::from(&working_directory_str);
        if !working_directory.is_dir() {
            if miss_ok {
                return Ok(());
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Specified directory is invalid.",
                )));
            }
        }

        self.working_directory = Some(working_directory);
        Ok(())
    }

    /// get WorkingDirectory
    pub fn get_working_directory(&self) -> Option<PathBuf> {
        self.working_directory.clone()
    }
}

bitflags! {
    /// the for exec the child command
    pub struct ExecFlags: u16 {
        /// the command is a control command
        const CONTROL = 1 << 1;
        /// need pass fds to the command
        const PASS_FDS = 1 << 2;
    }
}

#[cfg(test)]
mod tests {
    use super::ExecParameters;

    #[test]
    fn test_add_working_directory() {
        let mut params = ExecParameters::new();
        assert!(params.add_working_directory("/root".to_string()).is_ok());
        assert_eq!(
            params.get_working_directory().unwrap().to_str(),
            Some("/root")
        );
        let mut params = ExecParameters::new();
        assert!(params
            .add_working_directory("-/root/foooooooobarrrrrr".to_string())
            .is_ok());
        assert_eq!(params.get_working_directory(), None);
        let mut params = ExecParameters::new();
        assert!(params
            .add_working_directory("/root/fooooooooobarrrrrrrrrrrr".to_string())
            .is_err());
        assert_eq!(params.get_working_directory(), None);
        let mut params = ExecParameters::new();
        assert!(params
            .add_working_directory("--------------/usr/lib".to_string())
            .is_ok());
        assert_eq!(
            params.get_working_directory().unwrap().to_str(),
            Some("/usr/lib")
        );
        let mut params = ExecParameters::new();
        assert!(params.add_working_directory("~".to_string()).is_ok());
        assert_eq!(
            params
                .get_working_directory()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            std::env::var("HOME").unwrap()
        );
    }
}
