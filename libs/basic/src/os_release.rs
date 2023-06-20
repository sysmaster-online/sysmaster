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

//! Parse info about /etc/os-release
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::error::*;

/// get value by key in /etc/os-release
pub fn get_os_release(key: &str) -> Result<Option<String>> {
    let reader = BufReader::new(File::open("/etc/os-release").context(IoSnafu)?);
    for line in reader.lines().map(std::result::Result::unwrap_or_default) {
        match parse_line(key, &line) {
            Some(value) => return Ok(Some(value.to_string())),
            None => continue,
        }
    }
    Ok(None)
}

fn parse_line<'a>(key: &str, line: &'a str) -> Option<&'a str> {
    if let Some(pos) = line.chars().position(|c| c == '=') {
        let line_key = line[..pos].trim();
        if key == line_key {
            let s = trim_quotes(line[pos + 1..].trim());
            if !s.is_empty() {
                return Some(s);
            }
        }
        None
    } else {
        None
    }
}

fn trim_quotes(str: &str) -> &str {
    if ["\"", "'"]
        .iter()
        .any(|s| str.starts_with(s) && str.ends_with(s))
    {
        str[1..str.len() - 1].trim()
    } else {
        str.trim()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_trim_quotes() {
        let str1 = "\"aaaaa\"";
        assert_eq!(&str1[1..str1.len() - 1], trim_quotes(str1));

        let str2 = "'aaaaa'";
        assert_eq!(&str2[1..str2.len() - 1], trim_quotes(str2));

        let str3 = "\"aaaaa";
        assert_eq!(str3, trim_quotes(str3));

        let str4 = "'aaaaa";
        assert_eq!(str4, trim_quotes(str4));
    }

    #[test]
    fn test_parse_line() {
        let line = "ID=\"openEuler\"";
        assert_eq!(parse_line("ID", line), Some("openEuler"));
        assert_eq!(parse_line("NONE", line), None);

        let line = "ID='openEuler'";
        assert_eq!(parse_line("ID", line), Some("openEuler"));
        assert_eq!(parse_line("NONE", line), None);

        let line = "IDopenEuler";
        assert_eq!(parse_line("ID", line), None);

        let line = "ID=";
        assert_eq!(parse_line("ID", line), None);

        let line = "ID=\"openEuler   \"";
        assert_eq!(parse_line("ID", line), Some("openEuler"));

        let line = "ID=\"openEuler   \"";
        assert_eq!(parse_line("ID", line), Some("openEuler"));

        let line = "ID=\"openEuler\"   ";
        assert_eq!(parse_line("ID", line), Some("openEuler"));

        let line = "ID=openEuler";
        assert_eq!(parse_line("ID", line), Some("openEuler"));

        let line = "ID=openEuler  ";
        assert_eq!(parse_line("ID", line), Some("openEuler"));
    }
}
