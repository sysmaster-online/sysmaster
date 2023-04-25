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

//! utilities
//!

use crate::{
    error::{Error, Result},
    rules::FormatSubstitutionType,
};
use lazy_static::lazy_static;
use regex::Regex;

/// check whether the formatters in the value are valid
pub(crate) fn check_value_format(key: &str, value: &str, nonempty: bool) -> Result<()> {
    if nonempty && value.is_empty() {
        return Err(Error::RulesLoadError {
            msg: format!("Ker '{}': value is empty.", key),
        });
    }

    check_format(key, value)
}

/// check whether the formatters in the attribute are valid
pub(crate) fn check_attr_format(key: &str, attr: &str) -> Result<()> {
    if attr.is_empty() {
        return Err(Error::RulesLoadError {
            msg: format!("Ker '{}': attribute is empty.", key),
        });
    }

    check_format(key, attr)
}

/// check whether the format of the value is valid
pub(crate) fn check_format(key: &str, value: &str) -> Result<()> {
    lazy_static! {
        static ref VALUE_RE: Regex =
            Regex::new("(\\$(?P<long>\\w+)|%(?P<short>\\w))(\\{(?P<attr>\\w+)\\})?").unwrap();
    }

    for subst in VALUE_RE.captures_iter(value) {
        let long = subst.name("long");
        let short = subst.name("short");
        let attr = subst.name("attr");
        let subst_type: FormatSubstitutionType = if let Some(long_match) = long {
            long_match
                .as_str()
                .parse::<FormatSubstitutionType>()
                .unwrap_or_default()
        } else if let Some(short_match) = short {
            short_match
                .as_str()
                .parse::<FormatSubstitutionType>()
                .unwrap_or_default()
        } else {
            FormatSubstitutionType::Invalid
        };

        if subst_type == FormatSubstitutionType::Invalid {
            return Err(Error::RulesLoadError {
                msg: format!("Key '{}': invalid substitute formatter type.", key),
            });
        }

        if matches!(
            subst_type,
            FormatSubstitutionType::Attr | FormatSubstitutionType::Env
        ) && attr.is_none()
        {
            return Err(Error::RulesLoadError {
                msg: format!("Key '{}': formatter attribute is missing.", key),
            });
        }

        if matches!(subst_type, FormatSubstitutionType::Result) {
            if let Some(m) = attr {
                let num = m.as_str().parse::<i32>();

                if num.is_err() {
                    return Err(Error::RulesLoadError {
                        msg: format!(
                            "Key '{}': formatter attribute of type \"result\" is not a valid number.",
                            key
                        ),
                    });
                }
            }
        }
    }

    Ok(())
}

/// log key point on device processing
#[macro_export]
macro_rules! device_trace {
    // Match rule that takes any number of arguments
    ($p:tt, $d:expr, $($arg:expr),*) => {
        let action = $d.get_action().unwrap_or_default().to_string();
        let sysname = $d.get_sysname().unwrap_or("no_sysname").to_string();
        let syspath = $d.get_syspath().unwrap_or("no_syspath").to_string();
        let subsystem = $d.get_subsystem().unwrap_or("no_subsystem".to_string());
        // Generate a string with formatted arguments
        let mut s: String = format!("{} {} {} {} {}",$p ,action, sysname, syspath, subsystem);
        $(s.push_str(format!(" {}", $arg).as_str());)*
        log::debug!("{}", s);
    };
}

#[cfg(test)]
mod tests {
    use device::Device;

    use super::check_value_format;

    #[test]
    fn test_check_value_format() {
        // valid value
        check_value_format("", "aaa$devnode{ID_PATH}bbb", false).unwrap();
        check_value_format("", "aaa$tempnode{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$sysfs{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$kernel{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$number{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$driver{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$devpath{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$id{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$major{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$minor{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$parent{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$name{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$links{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$root{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$sys{ID_PATH}ccc", false).unwrap();

        check_value_format(
            "",
            "aaa$devnode{ID_PATH}bbb$env{ID_FSTYPE}ccc$result",
            false,
        )
        .unwrap();

        // formatter type """, ttr" and "env" must take attribute
        check_value_format("", "aaa$attr{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$env{ID_PATH}ccc", false).unwrap();
        check_value_format("", "aaa$attr", false).unwrap_err();
        check_value_format("", "aaa$env", false).unwrap_err();

        // invalid value
        check_value_format("", "aaa$devnode{ID_PATH}bbb", false).unwrap();

        // formatter type """, result" can ignore attribute, thus there should be a delimiter after "result"
        // besides, if it t"", akes an attribute, the attribute must be a valid number.
        check_value_format("", "aaa$resultbbb", false).unwrap_err();
        check_value_format("", "aaa$result bbb", false).unwrap();
        check_value_format("", "aaa$result", false).unwrap();
        check_value_format("", "aaa$result{0}bbb", false).unwrap();
    }

    #[test]
    #[ignore]
    fn test_device_trace() {
        let mut device = Device::from_path("/dev/sda".to_string()).unwrap();

        device_trace!("test", device, "aaa");
    }
}
