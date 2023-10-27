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

use basic::condition::{Condition, ConditionType};
use std::cell::RefCell;

pub(super) mod condition_keys {
    /* Attention: sort the following options by dictionary order. */
    pub(crate) const CONDITION_AC_POWER: &str = "ConditionACPower";
    pub(crate) const CONDITION_CAPABILITY: &str = "ConditionCapability";
    pub(crate) const CONDITION_DIRECTORY_NOT_EMPTY: &str = "ConditionDirectoryNotEmpty";
    pub(crate) const CONDITION_FILE_IS_EXECUTABLE: &str = "ConditionFileIsExecutable";
    pub(crate) const CONDITION_FILE_NOT_EMPTY: &str = "ConditionFileNotEmpty";
    pub(crate) const CONDITION_FIRST_BOOT: &str = "ConditionFirstBoot";
    pub(crate) const CONDITION_KERNEL_COMMAND_LINE: &str = "ConditionKernelCommandLine";
    pub(crate) const CONDITION_NEEDS_UPDATE: &str = "ConditionNeedsUpdate";
    pub(crate) const CONDITION_PATH_EXISTS: &str = "ConditionPathExists";
    pub(crate) const CONDITION_PATH_EXISTS_GLOB: &str = "ConditionPathExistsGlob";
    pub(crate) const CONDITION_PATH_IS_DIRECTORY: &str = "ConditionPathIsDirectory";
    pub(crate) const CONDITION_PATH_IS_MOUNT_POINT: &str = "ConditionPathIsMountPoint";
    pub(crate) const CONDITION_PATH_IS_READ_WRITE: &str = "ConditionPathIsReadWrite";
    pub(crate) const CONDITION_PATH_IS_SYMBOLIC_LINK: &str = "ConditionPathIsSymbolicLink";
    pub(crate) const CONDITION_SECURITY: &str = "ConditionSecurity";
    pub(crate) const CONDITION_USER: &str = "ConditionUser";
}

pub(super) mod assert_keys {
    /* Attention: sort the following options by dictionary order. */
    pub(crate) const ASSERT_FILE_NOT_EMPTY: &str = "AssertFileNotEmpty";
    pub(crate) const ASSERT_PATH_EXISTS: &str = "AssertPathExists";
}

pub(super) struct UeCondition {
    init_flag: RefCell<i8>,
    conditions: RefCell<Conditions>,
    asserts: RefCell<Asserts>,
}

struct Conditions(Vec<Condition>);

struct Asserts(Vec<Condition>);

impl UeCondition {
    pub fn new() -> UeCondition {
        Self {
            init_flag: RefCell::new(0),
            conditions: RefCell::new(Conditions(Vec::new())),
            asserts: RefCell::new(Asserts(Vec::new())),
        }
    }
    fn new_condition(&self, c_type: ConditionType, params: String) -> Condition {
        let mut trigger = 0;
        let mut revert = 0;

        let mut param_str = params.as_str();
        let mut _s_params = params.strip_prefix('|');
        if let Some(s) = _s_params {
            trigger = 1;
            param_str = s;
            _s_params = param_str.strip_prefix('!');
        } else {
            _s_params = params.strip_prefix('!');
        }

        if let Some(s) = _s_params {
            revert = 1;
            param_str = s;
        }

        Condition::new(c_type, trigger, revert, param_str.to_string())
    }

    #[allow(dead_code)]
    pub(super) fn set_init_flag(&self, flag: i8) {
        *self.init_flag.borrow_mut() = flag;
    }

    pub(super) fn init_flag(&self) -> i8 {
        *self.init_flag.borrow()
    }

    pub(super) fn add_condition(&self, condop: &str, _params: String) {
        if _params.is_empty() {
            return;
        }
        use condition_keys::*;
        let c_type = match condop {
            CONDITION_AC_POWER => ConditionType::ACPower,
            CONDITION_CAPABILITY => ConditionType::Capability,
            CONDITION_DIRECTORY_NOT_EMPTY => ConditionType::DirectoryNotEmpty,
            CONDITION_FILE_IS_EXECUTABLE => ConditionType::FileIsExecutable,
            CONDITION_FILE_NOT_EMPTY => ConditionType::FileNotEmpty,
            CONDITION_FIRST_BOOT => ConditionType::FirstBoot,
            CONDITION_NEEDS_UPDATE => ConditionType::NeedsUpdate,
            CONDITION_KERNEL_COMMAND_LINE => ConditionType::KernelCommandLine,
            CONDITION_PATH_EXISTS => ConditionType::PathExists,
            CONDITION_PATH_EXISTS_GLOB => ConditionType::PathExistsGlob,
            CONDITION_PATH_IS_DIRECTORY => ConditionType::PathIsDirectory,
            CONDITION_PATH_IS_MOUNT_POINT => ConditionType::PathIsMountPoint,
            CONDITION_PATH_IS_READ_WRITE => ConditionType::PathIsReadWrite,
            CONDITION_SECURITY => ConditionType::Security,
            CONDITION_PATH_IS_SYMBOLIC_LINK => ConditionType::PathIsSymbolicLink,
            CONDITION_USER => ConditionType::User,
            _ => return,
        };
        let condition = self.new_condition(c_type, _params);
        self.conditions.borrow_mut().0.push(condition);
    }

    pub(super) fn add_assert(&self, assertop: &str, _params: String) {
        if _params.is_empty() {
            return;
        }
        use assert_keys::*;
        let c_type = match assertop {
            ASSERT_PATH_EXISTS => ConditionType::PathExists,
            ASSERT_FILE_NOT_EMPTY => ConditionType::FileNotEmpty,
            _ => return,
        };

        let condition = self.new_condition(c_type, _params);
        self.asserts.borrow_mut().0.push(condition);
    }

    fn condition_vec_test(conditions: &[Condition]) -> bool {
        let mut trigger_flag = 0;
        let mut ret = true;
        for cond in conditions {
            let r = cond.test();
            if cond.trigger() == 0 && !r {
                return false;
            }

            if cond.trigger() != 0 && trigger_flag == 0 {
                ret = r;
                if r {
                    trigger_flag = 1;
                };
            }
        }
        ret
    }

    pub(super) fn conditions_test(&self) -> bool {
        let conditions = &self.conditions.borrow().0;
        Self::condition_vec_test(conditions)
    }

    pub(super) fn asserts_test(&self) -> bool {
        let assert_conditions = &self.asserts.borrow().0;
        Self::condition_vec_test(assert_conditions)
    }
}

#[cfg(test)]
mod tests {
    use basic::condition::ConditionType;

    use crate::unit::entry::condition::condition_keys::CONDITION_NEEDS_UPDATE;

    use super::{
        assert_keys::ASSERT_PATH_EXISTS, condition_keys::CONDITION_PATH_EXISTS, UeCondition,
    };

    #[test]
    fn test_new_condition_trigger() {
        let uc = UeCondition::new();
        let c = uc.new_condition(ConditionType::FileNotEmpty, String::from("|!test"));
        assert_eq!(c.trigger(), 1, "condition trigger is {}", c.trigger());
    }
    #[test]
    fn test_new_condition_is_not_trigger() {
        let uc = UeCondition::new();
        let c = uc.new_condition(ConditionType::FileNotEmpty, String::from("!test"));
        assert_eq!(c.trigger(), 0, "condition trigger is {}", c.trigger());
    }

    #[test]
    fn test_new_condition_is_revert() {
        let uc = UeCondition::new();
        let c = uc.new_condition(ConditionType::FileNotEmpty, String::from("!test"));
        assert_eq!(c.revert(), 1, "condition revert is {}", c.revert());
    }
    #[test]
    fn test_new_condition_is_not_revert() {
        let uc = UeCondition::new();
        let c = uc.new_condition(ConditionType::FileNotEmpty, String::from("test"));
        assert_eq!(c.revert(), 0, "condition revert is {}", c.revert());
    }

    #[test]
    fn test_add_condition() {
        let uc = UeCondition::new();
        uc.add_condition(CONDITION_PATH_EXISTS, String::from("test"));
        assert_eq!(uc.conditions.borrow().0.len(), 1);
        uc.add_condition(CONDITION_NEEDS_UPDATE, String::from("True"));
        assert_eq!(uc.conditions.borrow().0.len(), 2);
    }

    #[test]
    fn test_add_assert() {
        let uc = UeCondition::new();
        uc.add_assert(ASSERT_PATH_EXISTS, String::from("assert"));
        assert_eq!(uc.asserts.borrow().0.len(), 1);
    }
}
