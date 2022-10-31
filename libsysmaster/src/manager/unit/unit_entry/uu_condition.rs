use libutils::condition::{Condition, ConditionType};
use std::cell::RefCell;

pub(super) const CONDITION_PATH_EXISTS: &str = "ConditionPathExists";
pub(super) const CONDITION_FILE_NOT_EMPTY: &str = "ConditionFileNotEmpty";
pub(super) const CONDITION_NEEDS_UPDATE: &str = "ConditionNeedsUpdate";

pub(super) const ASSERT_PATH_EXISTS: &str = "AssertPathExists";
pub(super) const ASSERT_FILE_NOT_EMPTY: &str = "AssertFileNotEmpty";

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

        let c_type = match condop {
            CONDITION_PATH_EXISTS => ConditionType::PathExists,
            CONDITION_FILE_NOT_EMPTY => ConditionType::FileNotEmpty,
            CONDITION_NEEDS_UPDATE => ConditionType::NeedsUpdate,
            _ => ConditionType::_MAX,
        };

        if c_type == ConditionType::_MAX {
            return;
        }
        let condition = self.new_condition(c_type, _params);
        self.conditions.borrow_mut().0.push(condition);
    }

    pub(super) fn add_assert(&self, assertop: &str, _params: String) {
        if _params.is_empty() {
            return;
        }
        let c_type = match assertop {
            ASSERT_PATH_EXISTS => ConditionType::PathExists,
            ASSERT_FILE_NOT_EMPTY => ConditionType::FileNotEmpty,
            _ => ConditionType::_MAX,
        };

        if c_type == ConditionType::_MAX {
            return;
        }

        let condition = self.new_condition(c_type, _params);
        self.asserts.borrow_mut().0.push(condition);
    }

    fn condition_vec_test(conditions: &Vec<Condition>) -> bool {
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
