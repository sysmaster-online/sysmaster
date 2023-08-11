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

//! the utils can be used to parse unit conf file
use crate::error::*;

/// the base that will be translate from str
#[derive(Debug, Eq, PartialEq)]
pub enum Base {
    /// binary number
    Binary,
    /// Decimal number
    Decimal,
}

/// return true if item is 1, yes, y, true, t or on
/// return false if item is 0, no, n, false, f, or off
pub fn parse_boolean(item: &str) -> Result<bool> {
    match &item.to_lowercase() as &str {
        "1" | "yes" | "y" | "true" | "t" | "on" => Ok(true),
        "0" | "no" | "n" | "false" | "f" | "off" => Ok(false),
        _ => Err(Error::Parse {
            source: "wrong boolean value".into(),
        }),
    }
}

/// parse the item from string to u64
/// the item can be end with E, P， T， G， M， K and B
pub fn parse_size(item: &str, base: Base) -> Result<u64> {
    let item = item.trim();
    if item.is_empty() {
        return Err(Error::Parse {
            source: "empty string".into(),
        });
    }

    if item.starts_with('-') {
        return Err(Error::Parse {
            source: "invalue string".into(),
        });
    }

    let binary_table = [
        (
            'E',
            1024u64 * 1024u64 * 1024u64 * 1024u64 * 1024u64 * 1024u64,
        ),
        ('P', 1024u64 * 1024u64 * 1024u64 * 1024u64 * 1024u64),
        ('T', 1024u64 * 1024u64 * 1024u64 * 1024u64),
        ('G', 1024u64 * 1024u64 * 1024u64),
        ('M', 1024u64 * 1024u64),
        ('K', 1024u64),
        ('B', 1u64),
        (' ', 1u64),
    ];

    let decimal_table = [
        (
            'E',
            1000u64 * 1000u64 * 1000u64 * 1000u64 * 1000u64 * 1000u64,
        ),
        ('P', 1000u64 * 1000u64 * 1000u64 * 1000u64 * 1000u64),
        ('T', 1000u64 * 1000u64 * 1000u64 * 1000u64),
        ('G', 1000u64 * 1000u64 * 1000u64),
        ('M', 1000u64 * 1000u64),
        ('K', 1000u64),
        ('B', 1u64),
        (' ', 1u64),
    ];

    let table = if base == Base::Binary {
        binary_table
    } else {
        decimal_table
    };

    if let Ok(v) = item.parse::<f64>() {
        return Ok(v as u64);
    }

    let mut ret: u64 = 0;
    let mut start: usize = usize::MAX;

    for (i, v) in item.as_bytes().iter().enumerate() {
        if char::from(*v) == ' ' || char::from(*v) == '.' {
            continue;
        };

        if char::from(*v).is_ascii_digit() {
            continue;
        }

        for (index, (key, _factor)) in table.iter().enumerate() {
            if *key == char::from(*v) {
                start = index;
                break;
            }
        }

        if start == usize::MAX {
            return Err(Error::Parse {
                source: "invalid unit".into(),
            });
        }

        let cur = item[..i].to_string().parse::<f64>()?;

        if cur > (u64::MAX / table[start].1) as f64 {
            return Err(Error::Parse {
                source: "value is out of range".into(),
            });
        }

        ret = (cur * table[start].1 as f64) as u64;
    }

    Ok(ret)
}

#[cfg(test)]
mod test {

    #[test]
    fn test_parse_size() {
        use crate::config::{parse_size, Base};
        let ret1 = parse_size("", Base::Binary);
        assert!(ret1.is_err());

        let ret1 = parse_size("100G", Base::Binary).unwrap();
        assert_eq!(ret1, 100 * 1024 * 1024 * 1024);

        let ret1 = parse_size("99", Base::Binary).unwrap();
        assert_eq!(ret1, 99);

        let ret1 = parse_size("99.4", Base::Binary).unwrap();
        assert_eq!(ret1, 99);

        let ret1 = parse_size("4.5K", Base::Binary).unwrap();
        assert_eq!(ret1, 4 * 1024 + 512);

        let ret1 = parse_size("15E", Base::Binary).unwrap();
        assert_eq!(
            ret1,
            15 * 1024u64 * 1024u64 * 1024u64 * 1024u64 * 1024u64 * 1024u64
        );

        let ret1 = parse_size("4.5C", Base::Binary);
        assert!(ret1.is_err());
    }

    #[test]
    fn test_parse_boolean() {
        use crate::config::parse_boolean;

        assert!(parse_boolean("1").unwrap());
        assert!(parse_boolean("y").unwrap());
        assert!(parse_boolean("Y").unwrap());
        assert!(parse_boolean("yes").unwrap());
        assert!(parse_boolean("YES").unwrap());
        assert!(parse_boolean("true").unwrap());
        assert!(parse_boolean("TRUE").unwrap());
        assert!(parse_boolean("on").unwrap());
        assert!(parse_boolean("ON").unwrap());

        assert!(!parse_boolean("0").unwrap());
        assert!(!parse_boolean("n").unwrap());
        assert!(!parse_boolean("N").unwrap());
        assert!(!parse_boolean("no").unwrap());
        assert!(!parse_boolean("NO").unwrap());
        assert!(!parse_boolean("false").unwrap());
        assert!(!parse_boolean("FALSE").unwrap());
        assert!(!parse_boolean("off").unwrap());
        assert!(!parse_boolean("OFF").unwrap());

        assert!(parse_boolean("process").is_err());
        assert!(parse_boolean("in").is_err());
    }
}
