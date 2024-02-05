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

//! the utils to test the conditions
use nix::{
    fcntl::{open, OFlag},
    sys::{
        stat,
        statvfs::{fstatvfs, FsFlags},
    },
};

use libc::{glob, glob_t, globfree, GLOB_NOSORT};

use crate::{
    cmdline, fd, fs::directory_is_not_empty, mount::is_mount_point, security, sysfs::SysFs, unistd,
};

#[cfg(target_env = "musl")]
use crate::mount::MountInfoParser;

use std::{
    ffi::CString,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    str::FromStr,
    string::String,
};

/// the type of the condition
#[derive(Eq, PartialEq)]
pub enum ConditionType {
    /// check whether the service manager is running on AC Power.
    ACPower,
    /// check the capability
    Capability,
    /// check if the directory is empty
    DirectoryNotEmpty,
    /// check if the file is executable
    FileIsExecutable,
    /// check file is empty
    FileNotEmpty,
    /// conditionalize units on whether the system is booting up for the first time
    FirstBoot,
    /// check the kernel cmdline
    KernelCommandLine,
    /// check need update
    NeedsUpdate,
    /// check path exist
    PathExists,
    /// check if the path exists using glob pattern
    PathExistsGlob,
    /// check if the path is directory
    PathIsDirectory,
    /// check if the path is a mount point
    PathIsMountPoint,
    /// check path is readable and writable
    PathIsReadWrite,
    /// check if the path is symbolic link
    PathIsSymbolicLink,
    /// check the security
    Security,
    /// check whether the service manager is running as the given user.
    User,
}

/// check whether the condition is met.
/// if the condition start with '|'， trigger it and as long as one condition is met, return ok.
/// if the condition start with '!', indicate reverse condition.
/// others indicate usual condition
pub struct Condition {
    c_type: ConditionType,
    trigger: i8,
    revert: i8,
    params: String,
}

impl Condition {
    /// create the condition instance
    pub fn new(c_type: ConditionType, trigger: i8, revert: i8, params: String) -> Self {
        Condition {
            c_type,
            trigger,
            revert,
            params,
        }
    }

    /// return the trigger
    pub fn trigger(&self) -> i8 {
        self.trigger
    }

    /// return the revert
    pub fn revert(&self) -> i8 {
        self.revert
    }

    /// running the condition test
    pub fn test(&self) -> bool {
        // empty self.params means that the condition is not set, so the test is successful
        if self.params.is_empty() {
            return true;
        }
        let result = match self.c_type {
            /* The following functions will return a positive value if check pass. */
            ConditionType::ACPower => self.test_ac_power(),
            ConditionType::Capability => self.test_capability(),
            ConditionType::DirectoryNotEmpty => self.test_directory_not_empty(),
            ConditionType::FileIsExecutable => self.test_file_is_executable(),
            ConditionType::FileNotEmpty => self.test_file_not_empty(),
            ConditionType::FirstBoot => self.test_first_boot(),
            ConditionType::KernelCommandLine => self.test_kernel_command_line(),
            ConditionType::NeedsUpdate => self.test_needs_update(),
            ConditionType::PathExists => self.test_path_exists(),
            ConditionType::PathExistsGlob => self.test_path_exists_glob(),
            ConditionType::PathIsDirectory => self.test_path_is_directory(),
            ConditionType::PathIsMountPoint => self.test_path_is_mount_point(),
            ConditionType::PathIsReadWrite => self.test_path_is_read_write(),
            ConditionType::PathIsSymbolicLink => self.test_path_is_symbolic_link(),
            ConditionType::Security => self.test_security(),
            ConditionType::User => self.test_user(),
        };

        (result > 0) ^ (self.revert() >= 1)
    }

    fn test_ac_power(&self) -> i8 {
        /* params is generated from bool.to_string(), so it should
         * be exactly "true", not "yes"/"on" or other words. */
        let is_true = self.params.eq("true");
        !(is_true ^ SysFs::on_ac_power()) as i8
    }

    fn test_capability(&self) -> i8 {
        let values = match crate::capability::Capability::from_str(&self.params) {
            Err(e) => {
                log::error!("Failed to parse ConditionCapability values: {}, assuming ConditionCapability check failed:{}", self.params,e);
                return 0;
            }
            Ok(v) => v,
        };

        let file = match File::open("/proc/self/status") {
            Err(_) => {
                log::error!(
                    "Failed to open /proc/self/status, assuming ConditionCapability check failed."
                );
                return 0;
            }
            Ok(v) => v,
        };
        let reader = BufReader::new(file);
        let p = "CapBnd:";
        let mut cap_bitmask: u64 = 0;
        for line in reader.lines() {
            let line = match line {
                Err(_) => {
                    log::error!("Failed to read /proc/self/status, assuming ConditionCapability check failed.");
                    return 0;
                }
                Ok(v) => v,
            };
            if !line.starts_with(p) {
                continue;
            }
            match u64::from_str_radix(line.trim_start_matches(p).trim_start(), 16) {
                Err(_) => {
                    log::error!(
                        "Failed to parse CapBnd, assuming ConditionCapability check failed"
                    );
                    return 0;
                }
                Ok(v) => {
                    cap_bitmask = v;
                    break;
                }
            };
        }

        log::debug!("capability {}{}", cap_bitmask, values);
        let res = cap_bitmask & values.bitmask();
        (res != 0) as i8
    }

    fn test_directory_not_empty(&self) -> i8 {
        match directory_is_not_empty(Path::new(&self.params)) {
            Ok(flag) => flag as i8,
            Err(_) => false as i8,
        }
    }

    fn test_file_is_executable(&self) -> i8 {
        let path = Path::new(&self.params);
        if path.is_dir() {
            return 0;
        }
        let s = match stat::stat(path) {
            Err(_) => {
                return 0;
            }
            Ok(v) => v,
        };
        (fd::stat_is_reg(s.st_mode) && (s.st_mode & 111 > 0)) as i8
    }

    fn test_file_not_empty(&self) -> i8 {
        let tmp_path = Path::new(&self.params);
        let result = tmp_path
            .metadata()
            .map(|m| if m.is_file() { m.len() > 0 } else { false })
            .unwrap_or(false);
        result as i8
    }

    fn test_first_boot(&self) -> i8 {
        let ret = cmdline::Cmdline::default().has_param("sysmaster.condition-first-boot");
        if ret {
            return ret as i8;
        }

        let result = self.params.eq("true");

        let existed = Path::new("/run/sysmaster/first-boot").exists();
        (result == existed) as i8
    }

    fn test_kernel_command_line(&self) -> i8 {
        let (params_key, params_value) = match self.params.split_once('=') {
            None => return cmdline::Cmdline::default().has_param(&self.params) as i8,
            Some(values) => (values.0, values.1),
        };

        let value = match cmdline::Cmdline::default().get_param(params_key) {
            None => {
                log::info!("Failed to get cmdline content, assuming ConditionKernelCommandLine check failed.");
                return 0;
            }
            Some(v) => v,
        };
        log::debug!(
            "Found kernel command line value: {}={}, params: {}",
            params_key,
            value,
            self.params
        );
        (value == params_value) as i8
    }

    fn test_needs_update(&self) -> i8 {
        0
    }

    fn test_path_exists(&self) -> i8 {
        let tmp_path = Path::new(&self.params);
        let result = tmp_path.exists();
        result as i8
    }

    fn test_path_exists_glob(&self) -> i8 {
        let pattern = CString::new(self.params.as_str()).unwrap();
        let mut pglob: glob_t = unsafe { std::mem::zeroed() };
        let status = unsafe {
            /* use GLOB_NOSORT to speed up. */
            glob(pattern.as_ptr(), GLOB_NOSORT, None, &mut pglob)
        };
        unsafe { globfree(&mut pglob) };
        (status == 0) as i8
    }

    fn test_path_is_directory(&self) -> i8 {
        Path::new(&self.params).is_dir() as i8
    }

    fn test_path_is_mount_point(&self) -> i8 {
        if self.params.eq("/") {
            return 1;
        }

        i8::from(is_mount_point(Path::new(&self.params)))
    }

    fn test_path_is_read_write(&self) -> i8 {
        let path = Path::new(&self.params);
        if !path.exists() {
            return 0;
        }
        let fd = match open(path, OFlag::O_CLOEXEC | OFlag::O_PATH, stat::Mode::empty()) {
            Err(e) => {
                log::error!(
                    "Failed to open {} for checking file system permission: {}",
                    self.params,
                    e
                );
                return 0;
            }
            Ok(v) => v,
        };
        if fd < 0 {
            log::error!("Invalid file descriptor.");
            return 0;
        }
        let flags = match fstatvfs(&fd) {
            Err(e) => {
                log::error!("Failed to get the stat of file system: {}", e);
                return 0;
            }
            Ok(v) => v,
        };
        (!flags.flags().contains(FsFlags::ST_RDONLY)) as i8
    }

    fn test_path_is_symbolic_link(&self) -> i8 {
        match std::fs::symlink_metadata(self.params.clone()) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    1
                } else {
                    0
                }
            }
            Err(_) => -1,
        }
    }

    fn test_security(&self) -> i8 {
        let res = match self.params.as_str() {
            "selinux" => security::selinux_enabled(),
            "apparmor" => security::apparmor_enabled(),
            "tomoyo" => security::tomoyo_enabled(),
            "ima" => security::ima_enabled(),
            "smack" => security::smack_enabled(),
            "audit" => security::audit_enabled(),
            "uefi-secureboot" => security::uefi_secureboot_enabled(),
            "tpm2" => security::tpm2_enabled(),
            _ => false,
        };
        res as i8
    }

    fn test_user(&self) -> i8 {
        // may be UID
        if let Ok(user) = unistd::parse_uid(&self.params) {
            return (user.uid == nix::unistd::getuid() || user.uid == nix::unistd::geteuid()) as i8;
        }

        if self.params.eq("@system") {
            return (unistd::uid_is_system(nix::unistd::getuid())
                || unistd::uid_is_system(nix::unistd::geteuid())) as i8;
        }

        // may be username
        let result = match unistd::parse_name(&self.params) {
            Ok(user) => user.uid == nix::unistd::getuid() || user.uid == nix::unistd::geteuid(),
            _ => false,
        };
        result as i8
    }
}

#[cfg(test)]
mod test {
    use super::{Condition, ConditionType};
    use crate::{
        cmdline,
        fs::write_string_file,
        security::{self},
    };
    use core::panic;
    use std::{any::Any, env, fs, path::Path};
    use tempfile::NamedTempFile;

    fn setup(_type: ConditionType, trigger: i8, revert: i8, params: String) -> Condition {
        Condition::new(_type, trigger, revert, params)
    }

    fn teardown() {}

    fn run_test<T>(test: T, c_type: ConditionType, trigger: i8, revert: i8, params: String)
    where
        T: FnOnce(Condition) + panic::UnwindSafe,
    {
        let c = setup(c_type, trigger, revert, params);
        let result: Result<(), Box<dyn Any + Send>> = std::panic::catch_unwind(|| {
            test(c);
        });
        teardown();
        assert!(result.is_ok());
    }

    #[test]
    fn test_ac_power_false() {
        run_test(
            |x| {
                let result = x.test();
                assert!(!result, "cond_ac power test false");
            },
            ConditionType::ACPower,
            0,
            0,
            String::from("false"),
        );
    }

    #[test]
    fn test_ac_power_true() {
        run_test(
            |c| {
                let result = c.test();
                assert!(result, "cond ac power test true");
            },
            ConditionType::ACPower,
            0,
            0,
            String::from("true"),
        );
    }

    #[test]
    fn test_capability() {
        run_test(
            |c: Condition| {
                let result = c.test();
                assert!(result, "test capability {} true", c.params);
            },
            ConditionType::Capability,
            0,
            0,
            String::from("CHOWN"),
        );
    }

    #[test]
    fn test_directory_not_empty() {
        run_test(
            |c: Condition| {
                let result = c.test();
                assert!(
                    result,
                    "test test_directory_not_empty,directory is:{}",
                    c.params
                );
            },
            ConditionType::DirectoryNotEmpty,
            0,
            0,
            String::from(env::temp_dir().to_str().unwrap()),
        )
    }

    #[test]
    fn test_directory_notempty_empty() {
        let tmp = env::temp_dir();
        let mut path: String = String::from(tmp.to_str().unwrap());
        path.push_str("/empty_test");
        let p: &Path = Path::new(path.as_str());
        fs::create_dir(p).unwrap();
        run_test(
            |c: Condition| {
                let result = c.test();
                fs::remove_dir(p).unwrap();
                assert!(
                    !result,
                    "test test_directory_notempty_empty,directory is: {}",
                    c.params
                );
            },
            ConditionType::DirectoryNotEmpty,
            0,
            0,
            String::from(path.as_str()),
        )
    }

    #[test]
    fn test_directory_noempty_is_not_dir() {
        let tmp_file: NamedTempFile = NamedTempFile::new().unwrap();
        run_test(
            |c: Condition| {
                assert!(
                    !c.test(),
                    "test test_directory_noempty_is_not_dir,directory is: {}",
                    c.params
                );
            },
            ConditionType::DirectoryNotEmpty,
            0,
            0,
            String::from(tmp_file.path().to_str().unwrap()),
        )
    }

    #[test]
    fn test_condition_path_no_exists() {
        run_test(
            |c| {
                assert!(!c.test(), "test_condition_path_no_exists {}", c.params);
            },
            ConditionType::PathExists,
            0,
            0,
            "/home/a_usually_not_existent_file".to_string(),
        );
    }

    #[test]
    fn test_condition_path_exists() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        run_test(
            |c: Condition| {
                let t_result = c.test();
                assert!(t_result, "condition_path is exists {}", c.params);
            },
            ConditionType::PathExists,
            0,
            0,
            String::from(path.to_str().unwrap()),
        );
    }

    #[test]
    fn test_condition_path_exists_revert() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        run_test(
            |c: Condition| {
                assert!(
                    !c.test(),
                    "condition test path exist revert error {}",
                    c.params
                );
            },
            ConditionType::PathExists,
            0,
            1,
            String::from(path.to_str().unwrap()),
        );
    }

    #[test]
    fn test_condition_test_file_not_empty() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        let _ = write_string_file(path, String::from("Hello world"));
        run_test(
            |c: Condition| {
                assert!(c.test(), "cond test file not empty");
            },
            ConditionType::FileNotEmpty,
            0,
            0,
            String::from(path.to_str().unwrap()),
        );
    }

    #[test]
    fn test_condition_test_file_empty() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        run_test(
            |c| {
                assert!(!c.test(), "cond test file empty");
            },
            ConditionType::FileNotEmpty,
            0,
            0,
            String::from(path.to_str().unwrap()),
        );
    }

    #[test]
    fn test_file_is_not_executable() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        run_test(
            |c| {
                assert!(!c.test(), "test_file_is_executable {}", c.params);
            },
            ConditionType::FileIsExecutable,
            0,
            0,
            String::from(path.to_str().unwrap()),
        )
    }
    #[test]
    fn test_security_selinux() {
        run_test(
            |c: Condition| {
                assert_eq!(
                    c.test(),
                    security::selinux_enabled(),
                    "test for security {}",
                    c.params
                );
            },
            ConditionType::Security,
            0,
            0,
            String::from("selinux"),
        );
    }

    #[test]
    fn test_secuirty_appamgr() {
        let policy = String::from("apparmor");
        run_test(
            |c: Condition| {
                assert_eq!(
                    c.test(),
                    security::apparmor_enabled(),
                    "test for security {}",
                    c.params
                );
            },
            ConditionType::Security,
            0,
            0,
            policy,
        );
    }

    #[test]
    fn test_secuirty_tomoyo() {
        let policy = String::from("tomoyo");
        run_test(
            |c: Condition| {
                assert_eq!(
                    c.test(),
                    security::tomoyo_enabled(),
                    "test for security {}",
                    c.params
                );
            },
            ConditionType::Security,
            0,
            0,
            policy,
        );
    }
    #[test]
    fn test_secuirty_ima() {
        let policy = String::from("ima");
        run_test(
            |c: Condition| {
                assert_eq!(
                    c.test(),
                    security::ima_enabled(),
                    "test for security {}",
                    c.params
                );
            },
            ConditionType::Security,
            0,
            0,
            policy,
        );
    }

    #[test]
    fn test_secuirty_smack() {
        let policy = String::from("smack");
        run_test(
            |c: Condition| {
                assert_eq!(
                    c.test(),
                    security::smack_enabled(),
                    "test for security {}",
                    c.params
                );
            },
            ConditionType::Security,
            0,
            0,
            policy,
        );
    }

    #[test]
    fn test_secuirty_audit() {
        let policy = String::from("audit");
        run_test(
            |c: Condition| {
                assert_eq!(
                    c.test(),
                    security::audit_enabled(),
                    "test for security {}",
                    c.params
                );
            },
            ConditionType::Security,
            0,
            0,
            policy,
        );
    }

    #[test]
    fn test_secuirty_uefi_secureboot() {
        let policy = String::from("uefi-secureboot");
        run_test(
            |c: Condition| {
                assert_eq!(
                    c.test(),
                    security::uefi_secureboot_enabled(),
                    "test for security {}",
                    c.params
                );
            },
            ConditionType::Security,
            0,
            0,
            policy,
        );
    }
    #[test]
    fn test_secuirty_tpm2() {
        let policy = String::from("tpm2");
        run_test(
            |c: Condition| {
                assert_eq!(
                    c.test(),
                    security::tpm2_enabled(),
                    "test for security {}",
                    c.params
                );
            },
            ConditionType::Security,
            0,
            0,
            policy,
        );
    }

    #[test]
    fn test_condition_user() {
        if nix::unistd::getuid() != nix::unistd::Uid::from_raw(0) {
            return;
        }

        run_test(
            |c| {
                assert!(c.test(), "cond root username");
            },
            ConditionType::User,
            0,
            0,
            "root".to_string(),
        );

        run_test(
            |c| {
                assert!(c.test(), "cond root userid");
            },
            ConditionType::User,
            0,
            0,
            "0".to_string(),
        );

        let fake_user = "fake";

        run_test(
            |c| {
                assert!(!c.test(), "cond fake username");
            },
            ConditionType::User,
            0,
            0,
            fake_user.to_string(),
        );

        run_test(
            |c| {
                assert!(!c.test(), "cond fake userid");
            },
            ConditionType::User,
            0,
            0,
            "1234".to_string(),
        );

        run_test(
            |c| {
                assert!(c.test(), "cond system username");
            },
            ConditionType::User,
            0,
            0,
            "@system".to_string(),
        );
    }

    #[test]
    fn test_path_is_directory() {
        run_test(
            |c| {
                assert!(c.test(), "test path is directory {}", c.params);
            },
            ConditionType::PathIsDirectory,
            0,
            0,
            String::from(env::temp_dir().to_str().unwrap()),
        );
    }

    #[test]
    fn test_path_is_mount_point() {
        run_test(
            |c| {
                //test mount point is not correct need modify
                assert!(!c.test(), "test_path_is_mount_point {}", c.params);
            },
            ConditionType::PathIsMountPoint,
            0,
            0,
            String::from("/dev/mapper/openeuler-root"),
        );
    }

    #[test]
    fn test_path_is_not_mount_point() {
        run_test(
            |c| {
                assert!(!c.test(), "test_path_is_not_mount_point {}", c.params);
            },
            ConditionType::PathIsMountPoint,
            0,
            0,
            String::from("/home/aaaa"),
        );
    }

    #[test]
    fn test_condition_first_boot() {
        run_test(
            |c| {
                if cmdline::Cmdline::default().has_param("sysmaster.condition-first-boot") {
                    println!(
                        "this test cannot be tested because we cannot modify the kernel parameters"
                    );
                    return;
                }

                let existed = Path::new("/run/sysmaster/first-boot").exists();
                if existed {
                    assert!(c.test(), "file should be existed{}", c.params);
                } else {
                    assert!(!c.test(), "file should not be existed{}", c.params);
                }
            },
            ConditionType::FirstBoot,
            0,
            0,
            String::from("true"),
        );
    }
    #[test]
    fn test_condition_first_boot_false() {
        run_test(
            |c| {
                if cmdline::Cmdline::default().has_param("sysmaster.condition-first-boot") {
                    println!(
                        "this test cannot be tested because we cannot modify the kernel parameters"
                    );
                    return;
                }

                let existed = Path::new("/run/sysmaster/first-boot").exists();
                if existed {
                    assert!(!c.test(), "file should be existed{}", c.params);
                } else {
                    assert!(c.test(), "file should not be existed{}", c.params);
                }
            },
            ConditionType::FirstBoot,
            0,
            0,
            String::from("false"),
        );
    }
}
