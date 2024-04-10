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

//! Cmdline functions
use nix::unistd::Pid;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;

/// The `Cmdline` struct represents a command line with parameters and their corresponding values.
///
/// Properties:
///
/// * `params`: The `params` property is a `HashMap` that stores key-value pairs. The keys are of type
/// `String`, and the values are of type `Option<String>`. The `Option` type allows for the possibility
/// of a value being present (`Some`) or absent (`None`).
#[derive(Debug)]
pub struct Cmdline {
    params: HashMap<String, Option<String>>,
    cmdline: String,
}

impl Cmdline {
    /// The function `read_cmdline` reads a file at a given path, extracts key-value pairs from its
    /// contents, and returns them as a HashMap.
    ///
    /// Arguments:
    ///
    /// * `path`: A string representing the path to the file that needs to be read.
    /// * `cmdline`: A mutable reference to a String that will be updated with the contents of the file
    /// at the specified path.
    ///
    /// Returns:
    ///
    /// The function `read_cmdline` returns a `HashMap<String, Option<String>>`.
    fn read_cmdline(path: String, cmdline: &mut String) -> HashMap<String, Option<String>> {
        let mut params = HashMap::new();
        if let Ok(mut file) = File::open(path) {
            let mut data = String::new();
            if file.read_to_string(&mut data).is_ok() {
                *cmdline = data.replace("\0", " ").trim().to_string();
                for item in cmdline.split_whitespace() {
                    let mut parts = item.splitn(2, '=');
                    let key = parts.next().unwrap_or_default().to_string();
                    let value = parts.next().map(|v| v.to_string());
                    params.insert(key, value);
                }
            }
        }
        params
    }

    /// The function `get_cmdline` returns a clone of the `cmdline` string.
    ///
    /// Returns:
    ///
    /// A String is being returned.
    pub fn get_cmdline(&self) -> String {
        self.cmdline.clone()
    }

    /// The code defines a struct `Cmdline` with methods to read command line parameters from a process
    /// and check if a parameter exists.
    ///
    /// Arguments:
    ///
    /// * `pid`: The `pid` parameter represents the process ID (PID) of a running process. It is used to
    /// identify a specific process in the system.
    ///
    /// Returns:
    ///
    /// In the `new` function, a new instance of the `Cmdline` struct is being returned.
    pub fn new(pid: Pid) -> Self {
        let cmdfile = format!("/proc/{}/cmdline", pid);
        let mut cmdline = String::new();
        let params = Self::read_cmdline(cmdfile, &mut cmdline);

        Cmdline { params, cmdline }
    }

    /// The `get_param` function retrieves a parameter value from a map and returns it as an
    /// `Option<String>`.
    ///
    /// Arguments:
    ///
    /// * `key`: The `key` parameter is a reference to a string that represents the key for which you
    /// want to retrieve a value from the `params` map.
    ///
    /// Returns:
    ///
    /// The function `get_param` returns an `Option<String>`.
    pub fn get_param(&self, key: &str) -> Option<String> {
        self.params.get(key).map(|v| v.clone().unwrap_or_default())
    }

    /// The function `has_param` checks if a given key exists in a map called `params`.
    ///
    /// Arguments:
    ///
    /// * `key`: The `key` parameter is of type `&str`, which means it is a reference to a string slice.
    ///
    /// Returns:
    ///
    /// The `has_param` function returns a boolean value indicating whether the given `key` is present in
    /// the `params` map.
    pub fn has_param(&self, key: &str) -> bool {
        self.params.contains_key(key)
    }

    /// The function `cmdline_item` parses a key-value pair and inserts the values into a
    /// HashSet if the key is "module_blacklist".
    ///
    /// Arguments:
    ///
    /// * `key`: The key parameter is a String that represents the key of a key-value pair in a command
    /// line item.
    /// * `value`: The value parameter is a string that represents the value associated with the key in the
    /// proc cmdline file.
    /// * `data`: A mutable reference to a HashSet of strings.
    pub fn cmdline_item(key: String, value: String, data: &mut HashSet<String>) {
        if key.eq("module_blacklist") {
            if value.is_empty() {
                return;
            }

            let k: Vec<&str> = value.split(',').collect();

            for i in k {
                data.insert(i.to_string());
            }
        }
    }

    /// The `parse` function reads the contents of the `/proc/cmdline` file, splits it into key-value
    /// pairs, and calls a provided function to parse and store the values.
    ///
    /// Arguments:
    ///
    /// * `parse_item`: The `parse_item` parameter is a closure that takes three arguments: a `String`
    /// representing a key, a `String` representing a value, and a mutable reference to `T` (the type of
    /// `data`). The closure is responsible for parsing the key-value pair and updating the `data
    /// * `data`: A mutable reference to the data structure that will be populated with the parsed
    /// key-value pairs.
    pub fn parse<F, T>(parse_item: F, data: &mut T)
    where
        F: Fn(String, String, &mut T),
    {
        let mut cmdline = String::new();
        let line = Self::read_cmdline("/proc/cmdline".to_string(), &mut cmdline);
        if line.is_empty() {
            log::info!("/proc/cmdline is empty!");
            return;
        }

        for i in cmdline.split(' ') {
            let parts = i.split_once('=');
            if let Some((key, value)) = parts {
                parse_item(key.to_string(), value.to_string(), data);
            }
        }
    }
}

impl Default for Cmdline {
    fn default() -> Self {
        let mut cmdline = String::new();
        let params = Self::read_cmdline("/proc/cmdline".to_string(), &mut cmdline);

        Cmdline { params, cmdline }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmdline_new() {
        let cmdline = Cmdline::default();
        println!("{:?}", cmdline);
        assert!(cmdline.has_param("root"));
        assert!(cmdline.get_param("root").is_some());
    }
    #[test]
    fn test_get_cmdline() {
        let mut cmdline = String::new();
        let params = Cmdline::read_cmdline("/proc/cmdline".to_string(), &mut cmdline);
        let cmdline_str = cmdline.clone();
        let cmdline = Cmdline { params, cmdline };
        assert_eq!(cmdline.get_cmdline(), cmdline_str);
    }

    #[test]
    fn test_get_param() {
        let mut cmdline = String::new();
        let params = Cmdline::read_cmdline("/proc/cmdline".to_string(), &mut cmdline);
        let cmdline = Cmdline { params, cmdline };
        assert!(!cmdline.get_param("argv0").is_some());
    }

    #[test]
    fn test_has_param() {
        let mut cmdline = String::new();
        let params = Cmdline::read_cmdline("/proc/cmdline".to_string(), &mut cmdline);
        let cmdline = Cmdline { params, cmdline };
        assert!(!cmdline.has_param("argv0"));
        assert!(!cmdline.has_param("not_a_key"));
    }
}
