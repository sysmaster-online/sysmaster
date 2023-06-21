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

//! the utils of the path operation
//!
use std::path::Path;

/// return true if the path of a and b equaled.
pub fn path_equal(a: &str, b: &str) -> bool {
    let p_a = Path::new(a);
    let p_b = Path::new(b);
    p_a == p_b
}

/// Remove redundant inner and trailing slashes and unnecessary dots to simplify path.
/// e.g., //foo//.//bar/ becomes /foo/bar
/// .//foo//.//bar/ becomes foo/bar
pub fn path_simplify(s: &str) -> String {
    let mut ret = String::new();

    let mut pre = "";

    for com in s.split('/') {
        match com {
            "" => {
                if ret.is_empty() && pre.is_empty() {
                    ret.push('/');
                }
            }
            "." => {
                if pre.is_empty() {
                    pre = ".";
                }
            }
            _ => {
                ret.push_str(com);
                ret.push('/');
                pre = com;
            }
        }
    }
    /* drop the trailing slash */
    if ret.ends_with('/') {
        let _ = ret.pop();
    }

    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_equal() {
        assert!(path_equal("/etc", "/etc"));
        assert!(path_equal("//etc", "/etc"));
        assert!(path_equal("/etc//", "/etc"));
        assert!(!path_equal("/etc", "./etc"));
        assert!(path_equal("/x/./y", "/x/y"));
        assert!(path_equal("/x/././y", "/x/y/./."));
        assert!(!path_equal("/etc", "/var"));
    }

    #[test]
    fn test_path_simplify() {
        assert_eq!(path_simplify("//foo//.//bar/"), "/foo/bar");
        assert_eq!(path_simplify(".//foo//.//bar/"), "foo/bar");
        assert_eq!(path_simplify("foo//.//bar/"), "foo/bar");
    }
}
