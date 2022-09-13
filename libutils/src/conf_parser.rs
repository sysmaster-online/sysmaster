use crate::Error;
use crate::Result;

#[derive(Debug, Eq, PartialEq)]
pub enum Base {
    Binary,
    Decimal,
}

pub fn parse_boolen(item: &str) -> Result<bool> {
    match &item.to_lowercase() as &str {
        "1" | "yes" | "y" | "true" | "t" | "on" => Ok(true),
        "0" | "no" | "n" | "false" | "f" | "off" => Ok(false),
        _ => Err(Error::ParseBoolError(
            "invalid string to boolen".to_string(),
        )),
    }
}

pub fn parse_size(item: &str, base: Base) -> Result<u64> {
    let item = item.trim();
    if item.is_empty() {
        return Err(Error::Other {
            msg: "empty string",
        });
    }

    if item.starts_with('-') {
        return Err(Error::Other {
            msg: "invalue string",
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
            return Err(Error::Other {
                msg: "invalid unit",
            });
        }

        let cur = item[..i].to_string().parse::<f64>()?;

        if cur > (u64::MAX / table[start].1) as f64 {
            return Err(Error::Other {
                msg: "value is out of range",
            });
        }

        ret = (cur as f64 * table[start].1 as f64) as u64;
    }

    Ok(ret)
}

#[cfg(test)]
mod test {

    #[test]
    fn test_parse_size() {
        use crate::conf_parser::{parse_size, Base};
        let ret1 = parse_size("", Base::Binary);
        assert_eq!(ret1.is_err(), true);

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
        assert_eq!(ret1.is_err(), true);
    }

    #[test]
    fn test_parse_boolen() {
        use crate::conf_parser::parse_boolen;

        assert_eq!(parse_boolen("1").unwrap(), true);
        assert_eq!(parse_boolen("y").unwrap(), true);
        assert_eq!(parse_boolen("Y").unwrap(), true);
        assert_eq!(parse_boolen("yes").unwrap(), true);
        assert_eq!(parse_boolen("YES").unwrap(), true);
        assert_eq!(parse_boolen("true").unwrap(), true);
        assert_eq!(parse_boolen("TRUE").unwrap(), true);
        assert_eq!(parse_boolen("on").unwrap(), true);
        assert_eq!(parse_boolen("ON").unwrap(), true);

        assert_eq!(parse_boolen("0").unwrap(), false);
        assert_eq!(parse_boolen("n").unwrap(), false);
        assert_eq!(parse_boolen("N").unwrap(), false);
        assert_eq!(parse_boolen("no").unwrap(), false);
        assert_eq!(parse_boolen("NO").unwrap(), false);
        assert_eq!(parse_boolen("false").unwrap(), false);
        assert_eq!(parse_boolen("FALSE").unwrap(), false);
        assert_eq!(parse_boolen("off").unwrap(), false);
        assert_eq!(parse_boolen("OFF").unwrap(), false);

        assert_eq!(parse_boolen("process").is_err(), true);
        assert_eq!(parse_boolen("in").is_err(), true);
    }
}
