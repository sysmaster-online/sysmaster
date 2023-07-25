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

/// The maximum length of a linux path
pub const PATH_LENGTH_MAX: usize = 4096;

/// The maximum length of a linux file name
pub const FILE_LENGTH_MAX: usize = 255;

/// return true if the path of a and b equaled.
pub fn path_equal(a: &str, b: &str) -> bool {
    let p_a = Path::new(a);
    let p_b = Path::new(b);
    p_a == p_b
}

/// check if the path name contains unsafe character
///
/// return true if it doesn't contain unsafe character
pub fn path_name_is_safe(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    for c in s.chars() {
        if c > 0 as char && c < ' ' {
            return false;
        }
        if (c as char).is_ascii_control() {
            return false;
        }
    }
    true
}

/// check if the path length is valid
pub fn path_length_is_valid(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    if s.len() > PATH_LENGTH_MAX {
        return false;
    }
    let mut de_len = 0;
    let mut last_c = '/';
    for c in s.chars() {
        match c {
            '/' => {
                de_len = 0;
            }
            '.' => {
                if last_c == '/' {
                    de_len = 1;
                } else {
                    de_len += 1;
                }
            }
            _ => {
                de_len += 1;
            }
        }
        if de_len > FILE_LENGTH_MAX {
            return false;
        }
        last_c = c;
    }
    true
}

/// Remove redundant inner and trailing slashes and unnecessary dots to simplify path.
/// e.g., //foo//.//bar/ becomes /foo/bar
/// /foo/foo1/../bar becomes /foo/bar
pub fn path_simplify(p: &str) -> Option<String> {
    let mut res = String::new();
    let mut stack: Vec<&str> = Vec::new();
    for f in p.split('/') {
        if f.is_empty() || f == "." {
            continue;
        }
        if f == ".." {
            if let Some(v) = stack.last() {
                if *v != ".." {
                    stack.pop();
                    continue;
                }
            }
            if !p.starts_with('/') {
                stack.push(f);
                continue;
            }
            return None;
        }
        stack.push(f);
    }

    if stack.is_empty() {
        if p.starts_with('/') {
            return Some("/".to_string());
        } else {
            return Some(".".to_string());
        }
    }

    if p.starts_with('/') {
        res += "/";
    }
    res += stack.remove(0);

    for f in stack {
        res += "/";
        res += f;
    }

    return Some(res);
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
    fn test_path_name_is_safe() {
        assert!(!path_name_is_safe(""));
        assert!(path_name_is_safe("/abc"));
        assert!(!path_name_is_safe("/abc\x7f/a"));
        assert!(!path_name_is_safe("/abc\x1f/a"));
        assert!(!path_name_is_safe("/\x0a/a"));
    }

    #[test]
    fn test_path_length_is_valid() {
        assert!(!path_length_is_valid(""));

        let path = "/a/".to_string() + &String::from_iter(vec!['1'; 255]);
        assert!(path_length_is_valid(&path));

        let path = "/a/".to_string() + &String::from_iter(vec!['1'; 256]);
        assert!(!path_length_is_valid(&path));

        let path = "/a/".to_string() + &String::from_iter(vec!['/'; 256]);
        assert!(path_length_is_valid(&path));

        let path = "/a/".to_string() + &String::from_iter(vec!['.'; 255]);
        assert!(path_length_is_valid(&path));

        let mut path = "".to_string();
        for _ in 0..40 {
            path += "/";
            path += &String::from_iter(vec!['1'; 100]);
        }
        assert!(path_length_is_valid(&path));

        let mut path = "".to_string();
        for _ in 0..41 {
            path += "/";
            path += &String::from_iter(vec!['1'; 100]);
        }
        assert!(!path_length_is_valid(&path));
    }

    #[test]
    fn test_path_simplify() {
        assert_eq!(path_simplify("//foo//.//bar/").unwrap(), "/foo/bar");
        assert_eq!(path_simplify(".//foo//.//bar/").unwrap(), "foo/bar");
        assert_eq!(path_simplify("foo//.//bar/").unwrap(), "foo/bar");
        assert_eq!(path_simplify("/a///b/////././././c").unwrap(), "/a/b/c");
        assert_eq!(path_simplify(".//././///././././a").unwrap(), "a");
        assert_eq!(path_simplify("/////////////////").unwrap(), "/");
        assert_eq!(path_simplify(".//////////////////").unwrap(), ".");
        assert_eq!(
            path_simplify("a/b/c../..d/e/f//g").unwrap(),
            "a/b/c../..d/e/f/g"
        );
        assert_eq!(
            path_simplify("aaa/bbbb/.//.//.....").unwrap(),
            "aaa/bbbb/....."
        );

        assert_eq!(path_simplify("a/b/c/../../d").unwrap(), "a/d");
        assert_eq!(path_simplify("a/b/c/../../..").unwrap(), ".");
        assert_eq!(path_simplify("a/b/../../../c/d").unwrap(), "../c/d");
        assert_eq!(path_simplify("../../../a/../").unwrap(), "../../..");
        assert!(path_simplify("/../../../a/../").is_none());
    }
}
