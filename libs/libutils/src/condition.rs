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
//!
use crate::{conf_parser, proc_cmdline, user_group_util};
use std::{path::Path, string::String};

/// the type of the condition
#[derive(Eq, PartialEq)]
pub enum ConditionType {
    /// check path exist
    PathExists,
    /// check file is empty
    FileNotEmpty,
    /// check need update
    NeedsUpdate,
    /// check whether the service manager is running as the given user.
    User,
    /// conditionalize units on whether the system is booting up for the first time
    FirstBoot,
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
        let mut result = match self.c_type {
            ConditionType::PathExists => self.test_path_exists(),
            ConditionType::FileNotEmpty => self.test_file_not_empty(),
            ConditionType::NeedsUpdate => self.test_needs_update(),
            ConditionType::User => self.test_user(),
            ConditionType::FirstBoot => self.test_first_boot(),
        };
        if self.revert() >= 1 {
            result = !result;
        }

        result > 0
    }

    fn test_path_exists(&self) -> i8 {
        let tmp_path = Path::new(&self.params);
        let result = tmp_path.exists();
        result as i8
    }

    fn test_file_not_empty(&self) -> i8 {
        let tmp_path = Path::new(&self.params);
        let result = tmp_path
            .metadata()
            .map(|m| if m.is_file() { m.len() > 0 } else { false })
            .unwrap_or(false);
        result as i8
    }

    fn test_needs_update(&self) -> i8 {
        0
    }

    fn test_user(&self) -> i8 {
        // may be UID
        if let Ok(user) = user_group_util::parse_uid(&self.params) {
            return (user.uid == nix::unistd::getuid() || user.uid == nix::unistd::geteuid()) as i8;
        }

        if self.params.eq("@system") {
            return (user_group_util::uid_is_system(nix::unistd::getuid())
                || user_group_util::uid_is_system(nix::unistd::geteuid()))
                as i8;
        }

        // may be username
        let result = match user_group_util::parse_name(&self.params) {
            Ok(user) => user.uid == nix::unistd::getuid() || user.uid == nix::unistd::geteuid(),
            _ => false,
        };
        result as i8
    }

    fn test_first_boot(&self) -> i8 {
        if let Ok(ret) = proc_cmdline::proc_cmdline_get_bool("sysmaster.condition-first-boot") {
            if ret {
                return ret as i8;
            }
        }

        let result = match conf_parser::parse_boolean(&self.params) {
            Ok(ret) => ret,
            _ => {
                return 0;
            }
        };

        let existed = Path::new("/run/sysmaster/first-boot").exists();
        (result == existed) as i8
    }
}

#[cfg(test)]
mod test {
    use crate::{logger, proc_cmdline};
    use libtests::get_project_root;
    use std::path::Path;

    use super::{Condition, ConditionType};

    #[test]
    fn test_condition_test() {
        logger::init_log_with_console("test_init_lookup_paths", log::LevelFilter::Debug);
        let project_root = get_project_root().unwrap();
        let cond_path_not_exists =
            Condition::new(ConditionType::PathExists, 0, 0, "/home/test".to_string());
        let f_result = cond_path_not_exists.test();
        assert!(!f_result);
        log::debug!("project root {:?}", project_root);
        let cond_path_exists = Condition::new(
            ConditionType::PathExists,
            0,
            0,
            project_root.to_str().unwrap().to_string(),
        );
        let t_result = cond_path_exists.test();
        assert!(t_result, "condition_path exists is not true");
        let cond_path_exists_revert = Condition::new(
            ConditionType::PathExists,
            0,
            1,
            project_root.to_str().unwrap().to_string(),
        );
        let f_result = cond_path_exists_revert.test();
        assert!(!f_result, "condition test path exist revert error");
        let cond_file_not_empty = Condition::new(
            ConditionType::FileNotEmpty,
            0,
            0,
            project_root.to_str().unwrap().to_string() + "/Cargo.lock",
        );
        assert!(cond_file_not_empty.test(), "cond test file not empty");

        let cond_file_empty = Condition::new(
            ConditionType::FileNotEmpty,
            0,
            0,
            project_root.to_str().unwrap().to_string(),
        );
        assert!(!cond_file_empty.test(), "cond test file empty");
    }

    #[test]
    fn test_condition_user() {
        if nix::unistd::getuid() != nix::unistd::Uid::from_raw(0) {
            return;
        }

        let root_user = "root";
        let cond_user_root_username =
            Condition::new(ConditionType::User, 0, 0, root_user.to_string());
        assert!(cond_user_root_username.test(), "cond root username");

        let root_user_num = "0";
        let cond_user_root_username_num =
            Condition::new(ConditionType::User, 0, 0, root_user_num.to_string());
        assert!(cond_user_root_username_num.test(), "cond root username");

        let fake_user = "fake";
        let cond_user_fake_username =
            Condition::new(ConditionType::User, 0, 0, fake_user.to_string());
        assert!(!cond_user_fake_username.test(), "cond fake username");

        let fake_user_num = "1234";
        let cond_user_fake_username_num =
            Condition::new(ConditionType::User, 0, 0, fake_user_num.to_string());
        assert!(!cond_user_fake_username_num.test(), "cond fake username");

        let system_str = "@system";
        let cond_user_system_str =
            Condition::new(ConditionType::User, 0, 0, system_str.to_string());
        assert!(cond_user_system_str.test(), "cond system username");
    }

    #[test]
    fn test_condition_first_boot() {
        if let Ok(ret) = proc_cmdline::proc_cmdline_get_bool("sysmaster.condition-first-boot") {
            if ret {
                println!(
                    "this test cannot be tested because we cannot modify the kernel parameters"
                );
                return;
            }
        }

        let existed = Path::new("/run/sysmaster/first-boot").exists();
        let cond_first_boot_true =
            Condition::new(ConditionType::FirstBoot, 0, 0, String::from("true"));
        let cond_first_boot_false =
            Condition::new(ConditionType::FirstBoot, 0, 0, String::from("false"));
        if existed {
            println!("file is existed");
            assert!(cond_first_boot_true.test(), "file should be existed");
            assert!(!cond_first_boot_false.test(), "file should be existed");
        } else {
            println!("file is no existed");
            assert!(!cond_first_boot_true.test(), "file should not be existed");
            assert!(cond_first_boot_false.test(), "file should not be existed");
        }

        let cond_first_boot_invalid =
            Condition::new(ConditionType::FirstBoot, 0, 0, String::from("invalid"));
        assert!(!cond_first_boot_invalid.test(), "params should be invalid");
    }
}
