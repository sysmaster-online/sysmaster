// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use crate::error::*;
use basic::fs::{path_is_abosolute, path_length_is_valid, path_name_is_safe, path_simplify};
use basic::rlimit;
use bitflags::bitflags;
use libc::EPERM;
use nix::sys::stat::Mode;
use nix::unistd::{Group, Uid, User};
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::str::FromStr;
use std::{cell::RefCell, collections::HashMap};
use std::{ffi::CString, path::PathBuf, rc::Rc};
use unit_parser::prelude::UnitEntry;

/// the Rlimit soft and hard value
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Rlimit {
    soft: u64,
    hard: u64,
}

impl Rlimit {
    fn setrlimit(&self, resource: u8) -> Result<()> {
        log::debug!(
            "set rlimit resource: {:?}, soft: {}, hard: {}",
            resource,
            self.soft,
            self.hard
        );

        if let Err(e) = rlimit::setrlimit(resource, self.soft, self.hard) {
            let (_soft, hard) = match e.raw_os_error() {
                Some(code) if code == EPERM => rlimit::getrlimit(resource)?,
                None => return Err(Error::from(e)),
                Some(_) => return Err(Error::from(e)),
            };

            if hard == self.hard {
                return Err(Error::from(e));
            }

            let new_soft = min(self.soft, hard);
            let new_hard = min(self.hard, hard);
            log::debug!(
                "set new rlimit resource: {:?}, soft: {}, hard: {}",
                resource,
                new_soft,
                new_hard
            );
            rlimit::setrlimit(resource, new_soft, new_hard)?;
        }
        Ok(())
    }
}

impl UnitEntry for Rlimit {
    type Error = crate::error::Error;

    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
        let rlimit = Rlimit::from_str(input.as_ref())?;
        Ok(rlimit)
    }
}

fn parse_rlimit(limit: &str) -> Result<u64, Error> {
    if limit.is_empty() {
        return Err(Error::ConfigureError {
            msg: "empty configure for Limit".to_string(),
        });
    }
    if limit == "infinity" {
        return Ok(rlimit::INFINITY);
    }

    let ret = limit.parse::<u64>()?;
    Ok(ret)
}

impl FromStr for Rlimit {
    type Err = crate::error::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let value: Vec<_> = s.trim().split_terminator(':').collect();
        let soft: u64;
        let hard: u64;
        if value.len() == 1 {
            soft = parse_rlimit(value[0])?;
            hard = soft;
        } else if value.len() == 2 {
            soft = parse_rlimit(value[0])?;
            hard = parse_rlimit(value[1])?;
        } else {
            return Err(Error::ConfigureError {
                msg: "invalid configure for Limit".to_string(),
            });
        }

        if soft > hard {
            return Err(Error::ConfigureError {
                msg: "soft is higher than hard limit".to_string(),
            });
        }

        Ok(Rlimit { soft, hard })
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
///
pub enum PreserveMode {
    ///
    No,
    ///
    Yes,
    ///
    Restart,
}

impl Default for PreserveMode {
    fn default() -> Self {
        Self::No
    }
}

impl UnitEntry for PreserveMode {
    type Error = Error;

    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
        let res = match input.as_ref() {
            "no" => PreserveMode::No,
            "yes" => PreserveMode::Yes,
            "restart" => PreserveMode::Restart,
            _ => {
                log::error!(
                    "Failed to parse RuntimeDirectoryPreserve: {}, assuming no",
                    input.as_ref()
                );
                PreserveMode::No
            }
        };
        Ok(res)
    }
}

fn is_valid_exec_directory(s: &str) -> bool {
    if !path_name_is_safe(s) {
        return false;
    }
    if !path_length_is_valid(s) {
        return false;
    }
    if path_is_abosolute(s) {
        return false;
    }
    true
}

/// WorkingDirectory of ExecContext
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct WorkingDirectory {
    directory: Option<PathBuf>,
    miss_ok: bool,
}

///
pub fn parse_working_directory(s: &str) -> Result<WorkingDirectory, basic::Error> {
    if s.is_empty() {
        return Ok(WorkingDirectory::new(None, true));
    }

    let mut miss_ok = false;
    if s.starts_with('-') {
        miss_ok = true;
    }

    let mut s: String = s.trim_start_matches('-').to_string();

    if s == *"~".to_string() {
        s = std::env::var("HOME").map_err(|_| basic::Error::Invalid {
            what: "can't get HOME environment".to_string(),
        })?;
    }

    Ok(WorkingDirectory::new(Some(PathBuf::from(&s)), miss_ok))
}

impl WorkingDirectory {
    ///
    pub fn new(directory: Option<PathBuf>, miss_ok: bool) -> Self {
        Self { directory, miss_ok }
    }

    ///
    pub fn directory(&self) -> Option<PathBuf> {
        self.directory.clone()
    }

    ///
    pub fn miss_ok(&self) -> bool {
        self.miss_ok
    }
}

/// RuntimeDirectory of ExecContext
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RuntimeDirectory {
    directory: Vec<PathBuf>,
}

///
pub fn parse_runtime_directory(s: &str) -> Result<RuntimeDirectory> {
    let mut res = RuntimeDirectory::default();
    for d in s.split_terminator(';') {
        if !is_valid_exec_directory(d) {
            return Err(Error::ConfigureError {
                msg: "invalid runtime directory".to_string(),
            });
        }

        let path = match path_simplify(d) {
            None => {
                return Err(Error::ConfigureError {
                    msg: "invalid runtime directory".to_string(),
                });
            }
            Some(v) => v,
        };

        res.add_directory(Path::new("/run").join(path));
    }

    Ok(res)
}

impl RuntimeDirectory {
    ///
    pub fn add_directory(&mut self, directory: PathBuf) {
        self.directory.push(directory);
    }

    ///
    pub fn directory(&self) -> Vec<PathBuf> {
        self.directory.clone()
    }
}

/// StateDirectory of ExecContext
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct StateDirectory {
    directory: Vec<PathBuf>,
}

///
pub fn parse_state_directory(s: &str) -> Result<StateDirectory> {
    /* Similar with RuntimeDirectory */
    let mut res = StateDirectory::default();
    for d in s.split_terminator(';') {
        if !is_valid_exec_directory(d) {
            return Err(Error::ConfigureError {
                msg: "not valid exec directory".to_string(),
            });
        }

        let path = match path_simplify(d) {
            None => {
                return Err(Error::ConfigureError {
                    msg: "not valid exec directory".to_string(),
                });
            }
            Some(v) => v,
        };

        res.add_directory(Path::new("/var/lib").join(path));
    }

    Ok(res)
}

impl StateDirectory {
    ///
    pub fn add_directory(&mut self, directory: PathBuf) {
        self.directory.push(directory);
    }

    ///
    pub fn directory(&self) -> Vec<PathBuf> {
        self.directory.clone()
    }
}

/// the exec context that was parse from the unit file.
/// like parsed from Environment field.
pub struct ExecContext {
    envs: RefCell<HashMap<String, String>>,
    env_files: RefCell<Vec<PathBuf>>,
    rlimits: RefCell<HashMap<u8, Rlimit>>,
    root_directory: RefCell<Option<PathBuf>>,
    working_directory: RefCell<WorkingDirectory>,
    runtime_directory: RefCell<RuntimeDirectory>,
    state_directory: RefCell<StateDirectory>,

    user: RefCell<Option<User>>,
    group: RefCell<Option<Group>>,
    umask: RefCell<Option<Mode>>,
    selinux_context: RefCell<Option<String>>,
}

impl Default for ExecContext {
    fn default() -> Self {
        ExecContext::new()
    }
}

///
pub fn parse_environment(s: &str) -> Result<HashMap<String, String>> {
    #[derive(PartialEq, Clone, Copy)]
    enum ParseState {
        Init,
        Key,
        Value,
        Quotes,
        BackSlash,
        WaitSpace,
        Invalid,
    }

    let mut state = ParseState::Init;
    let mut state_before_back_slash = ParseState::Value;
    let mut key = String::new();
    let mut value = String::new();
    let mut res: HashMap<String, String> = HashMap::new();
    for c in s.chars() {
        match state {
            ParseState::Init => {
                if !key.is_empty() && !value.is_empty() {
                    res.insert(key, value);
                }
                key = String::new();
                value = String::new();
                if c.is_ascii_alphanumeric() || c == '_' {
                    key += &c.to_string();
                    state = ParseState::Key;
                } else if c != ' ' {
                    state = ParseState::Invalid;
                    break;
                }
            }
            ParseState::Key => {
                if c.is_ascii_alphanumeric() || c == '_' {
                    key += &c.to_string();
                } else if c == '=' {
                    state = ParseState::Value;
                } else {
                    /* F-O=foo */
                    state = ParseState::Invalid;
                    break;
                }
            }
            ParseState::Value => {
                /* FOO="foo bar" */
                if c == '\"' {
                    state = ParseState::Quotes;
                    continue;
                }
                /* FOO==\"foo */
                if c == '\\' {
                    state = ParseState::BackSlash;
                    state_before_back_slash = ParseState::Value;
                    continue;
                }
                if c != ' ' {
                    value += &c.to_string();
                    continue;
                }
                state = ParseState::Init;
            }
            ParseState::BackSlash => {
                /* FOO=\"foo or FOO="\"foo bar" */
                value += &c.to_string();
                state = state_before_back_slash;
            }
            ParseState::Quotes => {
                /* We have got the right ", there must a space after. */
                if c == '\"' {
                    state = ParseState::WaitSpace;
                    continue;
                }
                if c == '\\' {
                    state = ParseState::BackSlash;
                    state_before_back_slash = ParseState::Quotes;
                    continue;
                }
                value += &c.to_string();
            }
            ParseState::WaitSpace => {
                if c != ' ' {
                    /* FOO="foo bar"x */
                    state = ParseState::Invalid;
                    break;
                } else {
                    state = ParseState::Init;
                }
            }
            ParseState::Invalid => {
                break;
            }
        }
    }
    if state == ParseState::Invalid {
        log::warn!("Found invalid Environment, breaking");
        return Ok(res);
    }
    if !key.is_empty()
        && !value.is_empty()
        && [ParseState::Init, ParseState::WaitSpace, ParseState::Value].contains(&state)
    {
        res.insert(key, value);
    }
    Ok(res)
}

impl ExecContext {
    /// create a new instance of exec context
    pub fn new() -> ExecContext {
        ExecContext {
            envs: RefCell::new(HashMap::new()),
            env_files: RefCell::new(vec![]),
            rlimits: RefCell::new(HashMap::new()),
            working_directory: RefCell::new(WorkingDirectory::default()),
            root_directory: RefCell::new(None),
            runtime_directory: RefCell::new(RuntimeDirectory::default()),
            state_directory: RefCell::new(StateDirectory::default()),
            user: RefCell::new(None),
            group: RefCell::new(None),
            umask: RefCell::new(None),
            selinux_context: RefCell::new(None),
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

    /// insert environment files
    pub fn insert_envs_files(&self, paths: Vec<String>) {
        for path in paths {
            self.env_files.borrow_mut().push(PathBuf::from(path));
        }
    }

    /// load envirenment from file
    pub fn load_env_from_file(&self) -> Result<(), Error> {
        for path in &*self.env_files.borrow() {
            if path.starts_with("-") {
                log::info!("ignore environment file; {:?}", path);
                continue;
            }

            if !path.exists() || !path.is_absolute() {
                continue;
            }

            let f = File::open(path)?;

            for line in io::BufReader::new(f).lines().flatten() {
                if line.trim().starts_with('#') {
                    continue;
                }

                let content = match line.split_once('=') {
                    None => continue,
                    Some(v) => (v.0.trim(), v.1.trim()),
                };

                self.envs
                    .borrow_mut()
                    .insert(content.0.to_string(), content.1.to_string());
            }
        }

        Ok(())
    }

    /// insert configured rlimit to ExecContext
    pub fn insert_rlimit(&self, resource: u8, rlimit: Rlimit) {
        self.rlimits.borrow_mut().insert(resource, rlimit);
    }

    /// set the configured rlimit
    pub fn set_all_rlimits(&self) -> Result<()> {
        for (resource, limit) in &*self.rlimits.borrow() {
            limit.setrlimit(*resource)?;
        }

        Ok(())
    }

    ///
    pub fn set_root_directory(&self, root_diretory: Option<PathBuf>) {
        *self.root_directory.borrow_mut() = root_diretory;
    }

    ///
    pub fn root_directory(&self) -> Option<PathBuf> {
        self.root_directory.borrow().clone()
    }

    ///
    pub fn set_working_directory(&self, working_directory: WorkingDirectory) {
        *self.working_directory.borrow_mut() = working_directory;
    }

    ///
    pub fn working_directory(&self) -> WorkingDirectory {
        self.working_directory.borrow().clone()
    }

    ///
    pub fn set_runtime_directory(&self, runtime_directory: RuntimeDirectory) {
        *self.runtime_directory.borrow_mut() = runtime_directory;
    }

    ///
    pub fn runtime_directory(&self) -> RuntimeDirectory {
        self.runtime_directory.borrow().clone()
    }

    ///
    pub fn set_state_directory(&self, state_directory: StateDirectory) {
        *self.state_directory.borrow_mut() = state_directory;
    }

    ///
    pub fn state_directory(&self) -> StateDirectory {
        self.state_directory.borrow().clone()
    }

    ///
    pub fn set_user(&self, user_str: &str) -> Result<()> {
        if user_str.is_empty() {
            *self.user.borrow_mut() = User::from_uid(Uid::from_raw(0)).unwrap();
            return Ok(());
        }

        /* try to parse as UID */
        if let Ok(user) = basic::unistd::parse_uid(user_str) {
            *self.user.borrow_mut() = Some(user);
            return Ok(());
        }

        /* parse as user name */
        if let Ok(Some(user)) = User::from_name(user_str) {
            *self.user.borrow_mut() = Some(user);
            return Ok(());
        }

        *self.user.borrow_mut() = None;
        Err(Error::ConfigureError {
            msg: "invalid user".to_string(),
        })
    }

    ///
    pub fn user(&self) -> Option<User> {
        self.user.borrow().clone()
    }

    ///
    pub fn set_group(&self, group_str: &str) -> Result<()> {
        /* set_user should be called before add_group */
        assert!(self.user().is_some());

        /* group is not configured, use the primary group of user */
        if group_str.is_empty() {
            let gid = self.user().unwrap().gid;
            *self.group.borrow_mut() = Group::from_gid(gid).unwrap();
            return Ok(());
        }

        /* try to parse group_str as GID */
        if let Ok(group) = basic::unistd::parse_gid(group_str) {
            *self.group.borrow_mut() = Some(group);
            return Ok(());
        }

        /* not a valid GID, parse it as a group name */
        if let Ok(Some(group)) = Group::from_name(group_str) {
            *self.group.borrow_mut() = Some(group);
            return Ok(());
        }

        *self.group.borrow_mut() = None;
        Err(Error::ConfigureError {
            msg: "invalid group".to_string(),
        })
    }

    ///
    pub fn group(&self) -> Option<Group> {
        self.group.borrow().clone()
    }

    ///
    pub fn set_umask(&self, umask_str: &str) -> Result<()> {
        let mut umask_str = umask_str;
        if umask_str.is_empty() {
            umask_str = "0022";
        }
        for c in umask_str.as_bytes() {
            if !(b'0'..b'8').contains(c) {
                *self.umask.borrow_mut() = None;
                return Err(Error::InvalidData);
            }
        }
        let mode = match u32::from_str_radix(umask_str, 8) {
            Err(_) => {
                *self.umask.borrow_mut() = None;
                return Err(Error::InvalidData);
            }
            Ok(v) => v,
        };
        *self.umask.borrow_mut() = Mode::from_bits(mode);
        Ok(())
    }

    ///
    pub fn umask(&self) -> Option<Mode> {
        *self.umask.borrow()
    }

    ///
    pub fn set_selinux_context(&self, selinux_context: Option<String>) {
        *self.selinux_context.borrow_mut() = selinux_context;
    }

    ///
    pub fn selinux_context(&self) -> Option<String> {
        self.selinux_context.borrow().clone()
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
///
pub enum ExecDirectoryType {
    ///
    Runtime = 0,
    ///
    State = 1,
    ///
    Cache = 2,
    ///
    Logs = 3,
    ///
    Config = 4,
}

/// the environment that will be set when start a new command
pub struct ExecParameters {
    environment: Rc<EnvData>,
    fds: Vec<i32>,
    notify_sock: Option<PathBuf>,
    cgroup_path: Option<PathBuf>,
    watchdog_usec: u64,
    flags: ExecFlags,
    nonblock: bool,
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
            cgroup_path: None,
            watchdog_usec: 0,
            flags: ExecFlags::CONTROL,
            nonblock: false,
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

    /// set nonblock
    pub fn set_nonblock(&mut self, nonblock: bool) {
        self.nonblock = nonblock;
    }

    /// get nonblock
    pub fn get_nonblock(&self) -> bool {
        self.nonblock
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

    /// set the software watchdog time
    pub fn set_watchdog_usec(&mut self, usec: u64) {
        self.watchdog_usec = usec;
    }

    /// return the software watchdog time
    pub fn watchdog_usec(&self) -> u64 {
        self.watchdog_usec
    }

    /// set the exec command flags
    pub fn set_exec_flags(&mut self, flags: ExecFlags) {
        self.flags = flags;
    }

    /// return the exec command flags
    pub fn exec_flags(&self) -> ExecFlags {
        self.flags
    }

    /// set the cgroup path
    pub fn set_cgroup_path(&mut self, cgroup_path: &Path) {
        self.cgroup_path = Some(PathBuf::from(cgroup_path));
    }

    /// get the cgroup path
    pub fn cgroup_path(&self) -> Option<PathBuf> {
        self.cgroup_path.clone()
    }
}

bitflags! {
    /// the for exec the child command
    pub struct ExecFlags: u16 {
        /// the command is a control command
        const CONTROL = 1 << 1;
        /// need pass fds to the command
        const PASS_FDS = 1 << 2;
        /// enable software watchdog
        const SOFT_WATCHDOG = 1 << 3;
    }
}

/// parse file mode bits
pub fn parse_mode(s: &str) -> Result<u32, Error> {
    u32::from_str_radix(s, 8).map_err(|_| Error::ConfigureError {
        msg: format!("Invalid SocketMode: {}", s),
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use basic::rlimit;
    use nix::{
        sys::stat::Mode,
        unistd::{Gid, Uid},
    };

    use crate::exec::{base::Rlimit, ExecContext};

    #[test]
    fn test_set_user() {
        let exec_ctx = ExecContext::new();
        assert!(exec_ctx.set_user("0").is_ok());
        assert_eq!(exec_ctx.user().unwrap().name, "root");
        assert!(exec_ctx.set_user("root").is_ok());
        assert_eq!(exec_ctx.user().unwrap().uid, Uid::from_raw(0));
        assert!(exec_ctx.set_user("010123").is_err());
        assert!(exec_ctx.set_user("---").is_err());
        assert!(exec_ctx.set_user("wwwwyyyyyffffff").is_err());
    }

    #[test]
    fn test_set_group() {
        let exec_ctx = ExecContext::new();
        assert!(exec_ctx.set_user("0").is_ok());
        assert!(exec_ctx.set_group("0").is_ok());
        assert_eq!(exec_ctx.group().unwrap().name, "root");

        assert!(exec_ctx.set_user("0").is_ok());
        assert!(exec_ctx.set_group("root").is_ok());
        assert_eq!(exec_ctx.group().unwrap().gid, Gid::from_raw(0));

        assert!(exec_ctx.set_user("0").is_ok());
        assert!(exec_ctx.set_group("010123").is_err());
        assert!(exec_ctx.set_group("---").is_err());
        assert!(exec_ctx.set_group("wwwwyyyyyffffff").is_err());
    }

    #[test]
    fn test_set_umask() {
        let exec_ctx = ExecContext::new();
        assert!(exec_ctx.set_umask("").is_ok());
        assert_eq!(exec_ctx.umask().unwrap(), Mode::from_bits(18).unwrap());
        assert!(exec_ctx.set_umask("0022").is_ok());
        assert_eq!(exec_ctx.umask().unwrap(), Mode::from_bits(18).unwrap());
        assert!(exec_ctx.set_umask("0o0022").is_err());
        assert_eq!(exec_ctx.umask(), None);
        assert!(exec_ctx.set_umask("0088").is_err());
        assert_eq!(exec_ctx.umask(), None);
        assert!(exec_ctx.set_umask("0011").is_ok());
        assert_eq!(exec_ctx.umask().unwrap(), Mode::from_bits(9).unwrap());
    }

    #[test]
    fn test_rlimit_from_str() {
        let source = "100";
        let ret = Rlimit::from_str(source);
        assert!(ret.is_ok());
        let rlimit = ret.unwrap();
        assert_eq!(rlimit.soft, 100);
        assert_eq!(rlimit.hard, 100);

        let source1 = "100:150";
        let ret = Rlimit::from_str(source1);
        assert!(ret.is_ok());
        let rlimit = ret.unwrap();
        assert_eq!(rlimit.soft, 100);
        assert_eq!(rlimit.hard, 150);

        let source2 = "infinity";
        let ret = Rlimit::from_str(source2);
        assert!(ret.is_ok());
        let rlimit = ret.unwrap();
        assert_eq!(rlimit.soft, rlimit::INFINITY);
        assert_eq!(rlimit.hard, rlimit::INFINITY);

        let source3 = "infinity:infinity";
        let ret = Rlimit::from_str(source3);
        assert!(ret.is_ok());
        let rlimit = ret.unwrap();
        assert_eq!(rlimit.soft, rlimit::INFINITY);
        assert_eq!(rlimit.hard, rlimit::INFINITY);

        let source4 = "100:infinity";
        let ret = Rlimit::from_str(source4);
        assert!(ret.is_ok());
        let rlimit = ret.unwrap();
        assert_eq!(rlimit.soft, 100);
        assert_eq!(rlimit.hard, rlimit::INFINITY);

        let source5 = "infinity:100";
        let rlimit = Rlimit::from_str(source5);
        assert!(rlimit.is_err());
    }

    use crate::exec::base::parse_working_directory;
    use std::path::PathBuf;
    #[test]
    fn test_parse_working_directory() {
        assert_eq!(
            parse_working_directory("/root").unwrap().directory(),
            Some(PathBuf::from("/root"))
        );
        assert_eq!(
            parse_working_directory("-/root/foooooooobarrrrrr")
                .unwrap()
                .directory(),
            Some(PathBuf::from("/root/foooooooobarrrrrr"))
        );
        assert_eq!(
            parse_working_directory("--------------/usr/lib")
                .unwrap()
                .directory(),
            Some(PathBuf::from("/usr/lib"))
        );
        assert_eq!(
            parse_working_directory("~").unwrap().directory(),
            Some(PathBuf::from(std::env::var("HOME").unwrap()))
        );
        assert_eq!(parse_working_directory("").unwrap().directory(), None);
    }

    use super::parse_mode;
    #[test]
    fn test_parse_mode() {
        assert_eq!(parse_mode("777").unwrap(), 0o777);
        assert_eq!(parse_mode("644").unwrap(), 0o644);
        assert!(parse_mode("-777").is_err());
        assert!(parse_mode("787").is_err());
        assert!(parse_mode("777aa").is_err());
        assert!(parse_mode("aaaaa").is_err());
        assert!(parse_mode("777 aa").is_err());
    }
}
