use std::path::Path;

#[derive(Eq, PartialEq)]
pub enum ConditionType {
    PathExists,
    FileNotEmpty,
    NeedsUpdate,
    _MAX,
}

// condtion 判断条件是否满足，如果条件以|开头，表示触发条件，触发条件只要有一个满足，则可以了
//条件以!开始表示反转，
//其他，表示普通条件
pub struct Condition {
    c_type: ConditionType,
    trigger: i8,
    revert: i8,
    params: String,
}

impl Condition {
    pub fn new(c_type: ConditionType, trigger: i8, revert: i8, params: String) -> Self {
        Condition {
            c_type,
            trigger,
            revert,
            params,
        }
    }
    pub fn trigger(&self) -> i8 {
        self.trigger
    }

    pub fn revert(&self) -> i8 {
        self.revert
    }

    pub fn test(&self) -> bool {
        if self.params.is_empty() {
            return false;
        }
        let mut result = match self.c_type {
            ConditionType::PathExists => self.test_path_exists(),
            ConditionType::FileNotEmpty => self.test_file_not_empty(),
            ConditionType::NeedsUpdate => self.test_needs_update(),
            ConditionType::_MAX => todo!(),
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
}

#[cfg(test)]
mod test {
    use crate::logger;
    use tests::get_project_root;

    use super::{Condition, ConditionType};

    #[test]
    fn test_condition_test() {
        logger::init_log_with_console("test_init_lookup_paths", 4);
        let project_root = get_project_root().unwrap();
        let cond_path_not_exists =
            Condition::new(ConditionType::PathExists, 0, 0, "/home/test".to_string());
        let f_result = cond_path_not_exists.test();
        assert!(f_result == false);
        log::debug!("project root {:?}", project_root);
        let cond_path_exists = Condition::new(
            ConditionType::PathExists,
            0,
            0,
            project_root.to_str().unwrap().to_string(),
        );
        let t_result = cond_path_exists.test();
        assert!(t_result, "condtion_path exists is not true");
        let cond_path_exists_revert = Condition::new(
            ConditionType::PathExists,
            0,
            1,
            project_root.to_str().unwrap().to_string(),
        );
        let f_result = cond_path_exists_revert.test();
        assert!(f_result == false, "condtion test path exist revert error");
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
}
