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
use crate::serialize::DeserializeWith;
use basic::rlimit_util;
use bitflags::bitflags;
use libc::EPERM;
use nix::sys::stat::Mode;
use nix::unistd::{Group, Uid, User};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::cmp::min;
use std::fs::File;
use std::io::{self, BufRead};
use std::str::FromStr;
use std::{cell::RefCell, collections::HashMap};
use std::{ffi::CString, path::PathBuf, rc::Rc};

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

        if let Err(e) = rlimit_util::setrlimit(resource, self.soft, self.hard) {
            let (_soft, hard) = match e.raw_os_error() {
                Some(code) if code == EPERM => rlimit_util::getrlimit(resource)?,
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
            rlimit_util::setrlimit(resource, new_soft, new_hard)?;
        }
        Ok(())
    }
}

fn parse_rlimit(limit: &str) -> Result<u64, Error> {
    if limit.is_empty() {
        return Err(Error::ConfigureError {
            msg: "empty configure for Limit".to_string(),
        });
    }
    if limit == "infinity" {
        return Ok(rlimit_util::INFINITY);
    }

    let ret = limit.parse::<u64>()?;
    Ok(ret)
}

impl FromStr for Rlimit {
    type Err = Error;
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

impl DeserializeWith for Rlimit {
    type Item = Self;

    fn deserialize_with<'de, D>(de: D) -> Result<Self::Item, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;
        let rlimit = Rlimit::from_str(&s).map_err(de::Error::custom)?;
        Ok(rlimit)
    }
}

/// the exec context that was parse from the unit file.
/// like parsed from Environment field.
pub struct ExecContext {
    envs: RefCell<HashMap<String, String>>,
    env_files: RefCell<Vec<PathBuf>>,
    rlimits: RefCell<HashMap<u8, Rlimit>>,
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
            env_files: RefCell::new(vec![]),
            rlimits: RefCell::new(HashMap::new()),
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

                let content: Vec<&str> = line.split('=').map(|s| s.trim()).collect();
                if content.len() != 2 {
                    continue;
                }

                self.envs
                    .borrow_mut()
                    .insert(content[0].to_string(), content[1].to_string());
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
}

/// the environment that will be set when start a new command
pub struct ExecParameters {
    environment: Rc<EnvData>,
    fds: Vec<i32>,
    notify_sock: Option<PathBuf>,
    root_directory: Option<PathBuf>,
    working_directory: Option<PathBuf>,
    user: Option<User>,
    group: Option<Group>,
    umask: Option<Mode>,
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
            root_directory: None,
            working_directory: None,
            user: None,
            group: None,
            umask: None,
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

    // Just check if the given directory exists
    fn add_directory_common(directory_str: String) -> Result<Option<PathBuf>> {
        if directory_str.is_empty() {
            return Ok(None);
        }
        let directory = PathBuf::from(&directory_str);
        if !directory.is_dir() {
            return Err(Error::InvalidData);
        }
        Ok(Some(directory))
    }

    /// add RootDirectory
    pub fn add_root_directory(&mut self, root_directory_str: String) -> Result<()> {
        self.root_directory = Self::add_directory_common(root_directory_str)?;
        Ok(())
    }

    /// get RootDirectory
    pub fn get_root_directory(&self) -> Option<PathBuf> {
        self.root_directory.clone()
    }

    /// add WorkingDirectory
    pub fn add_working_directory(&mut self, working_directory_str: String) -> Result<()> {
        if working_directory_str.is_empty() {
            return Ok(());
        }

        let mut miss_ok = false;
        if working_directory_str.starts_with('-') {
            miss_ok = true;
        }

        let mut working_directory_str = working_directory_str.trim_start_matches('-').to_string();

        if working_directory_str == *"~".to_string() {
            working_directory_str = std::env::var("HOME").context(VarSnafu)?
        }

        let working_directory = PathBuf::from(&working_directory_str);
        if !working_directory.is_dir() {
            if miss_ok {
                return Ok(());
            } else {
                return Err(Error::InvalidData);
            }
        }

        self.working_directory = Some(working_directory);
        Ok(())
    }

    /// get WorkingDirectory
    pub fn get_working_directory(&self) -> Option<PathBuf> {
        self.working_directory.clone()
    }

    /// add User
    pub fn add_user(&mut self, user_str: String) -> Result<()> {
        // 1. If user_str is empty, treat it as UID 0
        if user_str.is_empty() {
            self.user = User::from_uid(Uid::from_raw(0)).unwrap();
            return Ok(());
        }
        // 2. Try to parse user_str as UID
        if let Ok(user) = basic::user_group_util::parse_uid(&user_str) {
            self.user = Some(user);
            return Ok(());
        }
        // 3. OK, this is not a valid UID, try to parse it as user name
        if let Ok(Some(user)) = User::from_name(&user_str) {
            self.user = Some(user);
            return Ok(());
        }
        Err(Error::InvalidData)
    }

    /// get User
    pub fn get_user(&self) -> Option<User> {
        self.user.clone()
    }

    /// add Group
    pub fn add_group(&mut self, group_str: String) -> Result<()> {
        // add_user should be called before add_group
        assert!(self.get_user().is_some());
        // 1. Group is not configured, use the primary group of user
        if group_str.is_empty() {
            let gid = self.get_user().unwrap().gid;
            self.group = Group::from_gid(gid).unwrap();
            return Ok(());
        }
        // 2. Try to parse group_str as GID
        if let Ok(group) = basic::user_group_util::parse_gid(&group_str) {
            self.group = Some(group);
            return Ok(());
        }
        // 3. Not a valid GID, parse it as a group name
        if let Ok(Some(group)) = Group::from_name(&group_str) {
            self.group = Some(group);
            return Ok(());
        }
        Err(Error::InvalidData)
    }

    /// get Group
    pub fn get_group(&self) -> Option<Group> {
        self.group.clone()
    }

    /// add UMask
    pub fn add_umask(&mut self, umask_str: String) -> Result<()> {
        let mut umask_str = umask_str;
        if umask_str.is_empty() {
            umask_str = "0022".to_string();
        }
        for c in umask_str.as_bytes() {
            if !(b'0'..b'8').contains(c) {
                return Err(Error::InvalidData);
            }
        }
        let mode = match u32::from_str_radix(&umask_str, 8) {
            Err(_) => {
                return Err(Error::InvalidData);
            }
            Ok(v) => v,
        };
        self.umask = Mode::from_bits(mode);
        log::debug!("Adding umask {:?}", mode);
        Ok(())
    }

    /// get UMask
    pub fn get_umask(&self) -> Option<Mode> {
        self.umask
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use basic::rlimit_util;
    use nix::{
        sys::stat::Mode,
        unistd::{Gid, Uid},
    };

    use crate::exec::base::Rlimit;

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

    #[test]
    fn test_add_user() {
        let mut params = ExecParameters::new();
        assert!(params.add_user("0".to_string()).is_ok());
        assert_eq!(params.get_user().unwrap().name, "root");
        let mut params = ExecParameters::new();
        assert!(params.add_user("root".to_string()).is_ok());
        assert_eq!(params.get_user().unwrap().uid, Uid::from_raw(0));
        let mut params = ExecParameters::new();
        assert!(params.add_user("010123".to_string()).is_err());
        assert!(params.add_user("---".to_string()).is_err());
        assert!(params.add_user("wwwwyyyyyffffff".to_string()).is_err());
    }

    #[test]
    fn test_add_group() {
        let mut params = ExecParameters::new();
        assert!(params.add_user("0".to_string()).is_ok());
        assert!(params.add_group("0".to_string()).is_ok());
        assert_eq!(params.get_group().unwrap().name, "root");

        let mut params = ExecParameters::new();
        assert!(params.add_user("0".to_string()).is_ok());
        assert!(params.add_group("root".to_string()).is_ok());
        assert_eq!(params.get_group().unwrap().gid, Gid::from_raw(0));

        let mut params = ExecParameters::new();
        assert!(params.add_user("0".to_string()).is_ok());
        assert!(params.add_group("010123".to_string()).is_err());
        assert!(params.add_group("---".to_string()).is_err());
        assert!(params.add_group("wwwwyyyyyffffff".to_string()).is_err());
    }

    #[test]
    fn test_add_umask() {
        let mut params = ExecParameters::new();
        assert!(params.add_umask("".to_string()).is_ok());
        assert_eq!(params.get_umask().unwrap(), Mode::from_bits(18).unwrap());
        assert!(params.add_umask("0022".to_string()).is_ok());
        assert_eq!(params.get_umask().unwrap(), Mode::from_bits(18).unwrap());
        params.umask = None;
        assert!(params.add_umask("0o0022".to_string()).is_err());
        assert_eq!(params.get_umask(), None);
        params.umask = None;
        assert!(params.add_umask("0088".to_string()).is_err());
        assert_eq!(params.get_umask(), None);
        assert!(params.add_umask("0011".to_string()).is_ok());
        assert_eq!(params.get_umask().unwrap(), Mode::from_bits(9).unwrap());
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
        assert_eq!(rlimit.soft, rlimit_util::INFINITY);
        assert_eq!(rlimit.hard, rlimit_util::INFINITY);

        let source3 = "infinity:infinity";
        let ret = Rlimit::from_str(source3);
        assert!(ret.is_ok());
        let rlimit = ret.unwrap();
        assert_eq!(rlimit.soft, rlimit_util::INFINITY);
        assert_eq!(rlimit.hard, rlimit_util::INFINITY);

        let source4 = "100:infinity";
        let ret = Rlimit::from_str(source4);
        assert!(ret.is_ok());
        let rlimit = ret.unwrap();
        assert_eq!(rlimit.soft, 100);
        assert_eq!(rlimit.hard, rlimit_util::INFINITY);

        let source5 = "infinity:100";
        let rlimit = Rlimit::from_str(source5);
        assert!(rlimit.is_err());
    }
}
