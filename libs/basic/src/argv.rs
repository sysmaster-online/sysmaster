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

//! Arguments functions
use std::path::Path;

/// The function checks if the first argument in the command line contains a specific token.
///
/// Arguments:
///
/// * `argv`: A vector of strings representing command line arguments passed to the program. The first
/// element of the vector is expected to be the name of the executable file.
/// * `token`: The `token` parameter is a string that represents a token or keyword that we want to
/// check for in the file name.
///
/// Returns:
///
/// a boolean value.
pub fn invoked_as(argv: Vec<String>, token: &str) -> bool {
    if argv.is_empty() || token.is_empty() {
        return false;
    }

    if let Some(path) = Path::new(&argv[0]).file_name() {
        if let Some(file_name) = path.to_str() {
            return file_name.contains(token);
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use crate::argv::invoked_as;
    #[test]
    fn test_invoked_as() {
        let argv = vec!["/a/bb////aabbcc".to_string()];

        assert!(!invoked_as(argv.clone(), "abc"));
        assert!(invoked_as(argv.clone(), "ab"));
        assert!(!invoked_as(argv, ""));

        let argv = vec![];
        assert!(!invoked_as(argv, "abc"));
    }
}
