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

//! common utilities

use std::{cell::RefCell, rc::Rc};

use crate::{error::*, log_dev, rules::FormatSubstitutionType};
use basic::error::errno_is_privilege;
use device::{Device, DB_BASE_DIR};
use lazy_static::lazy_static;
use nix::{errno::Errno, unistd::unlink};
use regex::Regex;
use snafu::ResultExt;

pub(crate) const DEVMASTER_LEGAL_CHARS: &str = "/ $%?,";

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

/// Check whether the format of the value is valid.
///
/// The formatter should have explicit trailing delimiter,
/// that is to say the formatter must end with attribute '{xxx}'
/// or non-unicode characters.
pub(crate) fn check_format(key: &str, value: &str) -> Result<()> {
    lazy_static! {
        static ref VALUE_RE: Regex =
            Regex::new("(?P<placeholder>(\\$(?P<long>\\w+)|%(?P<short>\\w))(\\{(?P<attr>[^\\{\\}]+)\\})?)|(?P<escaped>(\\$\\$)|(%%))").unwrap();
    }

    for subst in VALUE_RE.captures_iter(value) {
        if subst.name("escaped").is_some() {
            continue;
        }

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
                let s = m.as_str();
                let num = if s.ends_with('+') {
                    s[0..s.len() - 1].parse::<i32>()
                } else {
                    s.parse::<i32>()
                };

                if num.is_err() {
                    return Err(Error::RulesLoadError {
                        msg: format!("Key '{}': formatter 'result' has invalid index.", key),
                    });
                }
            }
        }
    }

    Ok(())
}

pub(crate) fn valid_devnode_chars(c: char, white_list: &str) -> bool {
    c.is_ascii_alphanumeric() || "#+-.:=@_".contains(c) || white_list.contains(c)
}

pub(crate) fn valid_ifname_chars(c: char) -> bool {
    if c as u32 <= 32 || c as u32 >= 127 || ":/%".contains(c) {
        return false;
    }

    true
}

/// replace invalid chars with '_', except for white list, plain ascii, hex-escaping and valid utf8
/// as Rust strings are always encoded as utf8, we don't need to check whether chars are valid utf8
pub fn replace_chars(s: &str, white_list: &str) -> String {
    let mut ret: String = String::new();
    let l = s.len();
    let mut i = 0;
    loop {
        if i >= l {
            break;
        }

        let c = s.chars().nth(i).unwrap();
        if valid_devnode_chars(c, white_list) {
            ret.push(c);
            i += 1;
            continue;
        }

        if c == '\\' && i + 1 < l && s.chars().nth(i + 1).unwrap() == 'x' {
            ret.push('\\');
            ret.push('x');
            i += 2;
            continue;
        }

        // if whitespace is in white list, replace whitespace with ordinary space
        if c.is_whitespace() && white_list.contains(' ') {
            ret.push(' ');
            i += 1;
            continue;
        }

        ret.push('_');
        i += 1;
    }

    ret
}

pub(crate) fn replace_ifname(s: &str) -> String {
    s.replace(|c| !valid_ifname_chars(c), "_")
}

/// This function replaces excess whitespace in a string with underscores.
/// It uses a regular expression to match one or more whitespace characters.
/// The input string is not modified, and a new string with the replacements is returned.
pub fn replace_whitespace(s: &str) -> String {
    // Remove consecutive spaces after the last non-space character
    let s = s.trim_end_matches(' ');
    // Create a regular expression to match one or more whitespace characters.
    let re = Regex::new(r"\s+").unwrap();
    // Use the regular expression to replace all matches with underscores.
    // The resulting string is converted to a String and returned.
    re.replace_all(s, "_").to_string()
}

/// This function encodes a device node name string into a byte array.
/// The encoded string is stored in the output string buffer.
/// If the input string or output buffer is empty, an error is returned.
pub fn encode_devnode_name(str: &str, str_enc: &mut String) {
    let mut i = 0;

    if str.is_empty() {
        return;
    }

    while let Some(c) = str.chars().nth(i) {
        let seqlen = c.len_utf8();

        // If the character is a multi-byte character, add it to the output buffer.
        if seqlen > 1 {
            str_enc.push(c);
        }
        // If the character is a backslash or not allowed in a device node name, encode it as a hex value.
        else if c == '\\' || !valid_devnode_chars(c, "") {
            str_enc.push_str(&format!("\\x{:02x}", c as u8));
        }
        // Otherwise, add the character to the output buffer.
        else {
            str_enc.push(c);
        }
        i += 1;
    }
}

/// log key point on device processing
#[macro_export]
macro_rules! device_trace {
    // Match rule that takes any number of arguments
    ($p:tt, $d:expr) => {{
        let action = $d.get_action().unwrap_or_default().to_string();
        let sysname = $d.get_sysname().unwrap_or("no_sysname".to_string());
        let syspath = $d.get_syspath().unwrap_or("no_syspath".to_string());
        let subsystem = $d.get_subsystem().unwrap_or("no_subsystem".to_string());
        format!("{}: {} {} {} {}", $p, action, sysname, syspath, subsystem)
    }};
}

/// resolve [<SUBSYSTEM>/<KERNEL>]<attribute> string
/// if 'read' is true, read the attribute from sysfs device tree and return the value
/// else just return the attribute path under sysfs device tree.
pub(crate) fn resolve_subsystem_kernel(s: &str, read: bool) -> Result<String> {
    lazy_static! {
        static ref PATTERN: Regex = Regex::new(
            "\\[(?P<subsystem>[^/\\[\\]]+)/(?P<sysname>[^/\\[\\]]+)\\](?P<attribute>.*)"
        )
        .unwrap();
    }

    match PATTERN.captures(s) {
        Some(c) => {
            let subsystem = c
                .name("subsystem")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let sysname = c
                .name("sysname")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let attribute = c
                .name("attribute")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default()
                .trim_start_matches('/')
                .to_string();

            if read && attribute.is_empty() {
                return Err(Error::Other {
                    msg: format!("can not read empty sysattr: '{}'", s),
                    errno: nix::errno::Errno::EINVAL,
                });
            }

            let device =
                Device::from_subsystem_sysname(&subsystem, &sysname).map_err(|e| Error::Other {
                    msg: format!("failed to get device: ({})", e),
                    errno: e.get_errno(),
                })?;

            if read {
                let attr_value = match device.get_sysattr_value(&attribute) {
                    Ok(v) => v,
                    Err(e) => {
                        if e.get_errno() == nix::errno::Errno::ENOENT
                            || errno_is_privilege(e.get_errno())
                        {
                            "".to_string()
                        } else {
                            return Err(Error::Other {
                                msg: format!("failed to read sysattr: ({})", e),
                                errno: e.get_errno(),
                            });
                        }
                    }
                };

                log::debug!(
                    "the sysattr value of '[{}/{}]{}' is '{}'",
                    subsystem,
                    sysname,
                    attribute,
                    attr_value
                );
                Ok(attr_value)
            } else {
                let syspath = device.get_syspath().context(DeviceSnafu)?;

                let attr_path = if attribute.is_empty() {
                    syspath
                } else {
                    syspath + "/" + attribute.as_str()
                };
                log::debug!(
                    "resolve path '[{}/{}]{}' as '{}'",
                    subsystem,
                    sysname,
                    attribute,
                    attr_path
                );
                Ok(attr_path)
            }
        }
        None => Err(Error::Other {
            msg: format!(
                "invalid '[<SUBSYSTEM>/<KERNEL>]<attribute>' pattern: ({})",
                s
            ),
            errno: nix::errno::Errno::EINVAL,
        }),
    }
}

pub(crate) fn sysattr_subdir_subst(sysattr: &str) -> Result<String> {
    match sysattr.find("/*/") {
        Some(idx) => {
            let dir = &sysattr[0..idx + 1];
            let tail = &sysattr[idx + 3..];
            for entry in (std::fs::read_dir(dir).map_err(|e| Error::Other {
                msg: format!("failed to read directory '{}': ({})", dir, e),
                errno: nix::errno::Errno::ENOENT,
            })?)
            .flatten()
            {
                if let Ok(md) = entry.metadata() {
                    if md.is_dir() {
                        let file = dir.to_string()
                            + entry.file_name().to_str().unwrap_or_default()
                            + "/"
                            + tail;
                        let path = std::path::Path::new(&file);
                        if path.exists() {
                            return Ok(file);
                        }
                    }
                }
            }
        }
        None => return Ok(sysattr.to_string()),
    }

    Err(Error::Other {
        msg: format!("sysattr is not found: '{}'", sysattr),
        errno: nix::errno::Errno::ENOENT,
    })
}

pub(crate) fn get_property_from_string(s: &str) -> Result<(String, String)> {
    lazy_static! {
        static ref RE_KEY_VALUE: Regex = Regex::new(
            "(?P<key>[^=]*)\\s*=\\s*(?P<value>([^\"']$)|([^\"'].*[^\"']$)|(\".*\"$)|('.*'$))"
        )
        .unwrap();
    }

    let s = s.trim();

    if s.starts_with('#') {
        return Err(Error::Other {
            msg: format!("ignore commented line '{}'", s),
            errno: Errno::EINVAL,
        });
    }

    let capture = RE_KEY_VALUE.captures(s).ok_or(Error::Other {
        msg: format!("failed to parse key and value for '{}'", s),
        errno: Errno::EINVAL,
    })?;

    let key = capture.name("key").unwrap().as_str();
    let value = capture.name("value").unwrap().as_str();

    if key.is_empty() || value.is_empty() {
        return Err(Error::Other {
            msg: format!("key or value can not be empty '{}'", s),
            errno: Errno::EINVAL,
        });
    }

    if value.starts_with('"') || value.starts_with('\'') {
        Ok((key.to_string(), value[1..value.len() - 1].to_string()))
    } else {
        Ok((key.to_string(), value[0..].to_string()))
    }
}

/// inherit initial timestamp from old device object
pub(crate) fn initialize_device_usec(
    dev_new: Rc<RefCell<Device>>,
    dev_old: Rc<RefCell<Device>>,
) -> Result<()> {
    let timestamp = dev_old.borrow().get_usec_initialized().unwrap_or_else(|_| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |v| v.as_secs())
    });

    dev_new
        .borrow()
        .set_usec_initialized(timestamp)
        .map_err(|e| {
            log::error!("failed to set initialization timestamp: {}", e);
            e
        })
        .context(DeviceSnafu)?;

    Ok(())
}

/// add new tags and remove deleted tags
pub(crate) fn device_update_tag(
    dev_new: Rc<RefCell<Device>>,
    dev_old: Option<Rc<RefCell<Device>>>,
    add: bool,
) -> Result<()> {
    if let Some(dev_old) = dev_old {
        for tag in &dev_old.borrow().tag_iter() {
            if let Ok(true) = dev_new.borrow().has_tag(tag) {
                continue;
            }

            let _ = dev_new
                .borrow()
                .update_tag(tag, false)
                .context(DeviceSnafu)
                .log_error(&format!("failed to remove old tag '{}'", tag));
        }
    }

    for tag in &dev_new.borrow().tag_iter() {
        let _ = dev_new
            .borrow()
            .update_tag(tag, add)
            .context(DeviceSnafu)
            .log_error(&format!("failed to add new tag '{}'", tag));
    }

    Ok(())
}

/// cleanup device database
pub(crate) fn cleanup_db(dev: Rc<RefCell<Device>>) -> Result<()> {
    let id = dev.borrow().get_device_id().context(DeviceSnafu)?;

    let db_path = format!("{}{}", DB_BASE_DIR, id);

    match unlink(db_path.as_str()) {
        Ok(_) => log_dev!(debug, dev.borrow(), format!("unlinked '{}'", db_path)),
        Err(e) => {
            if e != nix::Error::ENOENT {
                log_dev!(
                    error,
                    dev.borrow(),
                    format!("failed to unlink '{}' when cleanup db: {}", db_path, e)
                );
                return Err(Error::Nix { source: e });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use device::Device;

    #[test]
    fn test_check_value_format() {
        // valid long formatter
        check_value_format("", "aaa $devnode    ccc", false).unwrap();
        check_value_format("", "aaa $tempnode   ccc", false).unwrap();
        check_value_format("", "aaa $kernel     ccc", false).unwrap();
        check_value_format("", "aaa $number     ccc", false).unwrap();
        check_value_format("", "aaa $driver     ccc", false).unwrap();
        check_value_format("", "aaa $devpath    ccc", false).unwrap();
        check_value_format("", "aaa $id         ccc", false).unwrap();
        check_value_format("", "aaa $major      ccc", false).unwrap();
        check_value_format("", "aaa $minor      ccc", false).unwrap();
        check_value_format("", "aaa $parent     ccc", false).unwrap();
        check_value_format("", "aaa $name       ccc", false).unwrap();
        check_value_format("", "aaa $links      ccc", false).unwrap();
        check_value_format("", "aaa $root       ccc", false).unwrap();
        check_value_format("", "aaa $sys        ccc", false).unwrap();
        check_value_format("", "aaa $result     ccc", false).unwrap();
        check_value_format("", "aaa $attr{[net/lo]ifindex} ccc", false).unwrap();

        check_value_format("", "aaa$attr{xxx}ccc", false).unwrap();
        check_value_format("", "aaa$sysfs{xxx}ccc", false).unwrap();
        check_value_format("", "aaa$result{0}ccc", false).unwrap();
        check_value_format("", "aaa$result{0+}ccc", false).unwrap();

        // test short formatter
        check_value_format("", "aaa %N   ccc", false).unwrap();
        check_value_format("", "aaa %N   ccc", false).unwrap();
        check_value_format("", "aaa %k   ccc", false).unwrap();
        check_value_format("", "aaa %n   ccc", false).unwrap();
        check_value_format("", "aaa %d   ccc", false).unwrap();
        check_value_format("", "aaa %p   ccc", false).unwrap();
        check_value_format("", "aaa %b   ccc", false).unwrap();
        check_value_format("", "aaa %M   ccc", false).unwrap();
        check_value_format("", "aaa %m   ccc", false).unwrap();
        check_value_format("", "aaa %P   ccc", false).unwrap();
        check_value_format("", "aaa %D   ccc", false).unwrap();
        check_value_format("", "aaa %L   ccc", false).unwrap();
        check_value_format("", "aaa %r   ccc", false).unwrap();
        check_value_format("", "aaa %S   ccc", false).unwrap();
        check_value_format("", "aaa %c   ccc", false).unwrap();
        check_value_format("", "aaa %s{[net/lo]ifindex} ccc", false).unwrap();

        check_value_format("", "aaa$s{xxx}ccc", false).unwrap();
        check_value_format("", "aaa$c{0}ccc", false).unwrap();
        check_value_format("", "aaa$c{0+}ccc", false).unwrap();

        // test multiple formatters
        check_value_format("", "aaa$devnode{xxx}bbb$env{ID_FSTYPE}ccc$result", false).unwrap();

        // 'attr', 'sysfs' and 'env' formatters must follow attribute
        check_value_format("", "aaa$attr", false).unwrap_err();
        check_value_format("", "aaa$sysfs", false).unwrap_err();
        check_value_format("", "aaa$env", false).unwrap_err();

        // test && and %%
        check_value_format("", "$$", false).unwrap();
        check_value_format("", "$$kernel", false).unwrap();
        check_value_format("", "$$1", false).unwrap();
        check_value_format("", "%%", false).unwrap();
        check_value_format("", "%%N", false).unwrap();
        check_value_format("", "%%1", false).unwrap();
    }

    #[test]
    #[ignore]
    fn test_device_trace() {
        let device = Device::from_path("/dev/sda").unwrap();

        device_trace!("test", device);
    }

    #[test]
    #[ignore]
    fn test_resolve_subsystem_kernel() {
        assert!(resolve_subsystem_kernel("[net]", false).is_err());
        assert!(resolve_subsystem_kernel("[net/]", false).is_err());
        assert!(resolve_subsystem_kernel("[net/lo", false).is_err());
        assert!(resolve_subsystem_kernel("[net", false).is_err());
        assert!(resolve_subsystem_kernel("net/lo", false).is_err());

        assert_eq!(
            resolve_subsystem_kernel("[net/lo]", false).unwrap(),
            "/sys/devices/virtual/net/lo"
        );
        assert_eq!(
            resolve_subsystem_kernel("[net/lo]/", false).unwrap(),
            "/sys/devices/virtual/net/lo"
        );
        assert_eq!(
            resolve_subsystem_kernel("[net/lo]hoge", false).unwrap(),
            "/sys/devices/virtual/net/lo/hoge"
        );
        assert_eq!(
            resolve_subsystem_kernel("[net/lo]/hoge", false).unwrap(),
            "/sys/devices/virtual/net/lo/hoge"
        );

        assert!(resolve_subsystem_kernel("[net/lo]", true).is_err());
        assert!(resolve_subsystem_kernel("[net/lo]/", true).is_err());
        assert_eq!(resolve_subsystem_kernel("[net/lo]hoge", true).unwrap(), "");
        assert_eq!(resolve_subsystem_kernel("[net/lo]/hoge", true).unwrap(), "");
        assert_eq!(
            resolve_subsystem_kernel("[net/lo]address", true).unwrap(),
            "00:00:00:00:00:00"
        );
        assert_eq!(
            resolve_subsystem_kernel("[net/lo]/address", true).unwrap(),
            "00:00:00:00:00:00"
        );
    }

    #[test]
    fn test_replace_chars() {
        assert_eq!(replace_chars("abcd!efg", DEVMASTER_LEGAL_CHARS), "abcd_efg");
        assert_eq!(
            replace_chars("abcd\\xefg", DEVMASTER_LEGAL_CHARS),
            "abcd\\xefg"
        );
        assert_eq!(
            replace_chars("abcd\tefg", DEVMASTER_LEGAL_CHARS),
            "abcd efg"
        );
    }

    #[test]
    #[ignore]
    fn test_sysattr_subdir_subst() {
        let device = Device::from_path("/dev/sda").unwrap();
        let syspath = device.get_syspath().unwrap();
        println!(
            "{}",
            sysattr_subdir_subst(&(syspath + "/sda1/*/runtime_status")).unwrap()
        );
    }

    #[test]
    fn test_get_property_from_string() {
        assert_eq!(
            get_property_from_string("A=B").unwrap(),
            ("A".to_string(), "B".to_string())
        );
        assert_eq!(
            get_property_from_string("A=BB").unwrap(),
            ("A".to_string(), "BB".to_string())
        );
        assert_eq!(
            get_property_from_string("A=\"B\"").unwrap(),
            ("A".to_string(), "B".to_string())
        );
        assert_eq!(
            get_property_from_string("A='B'").unwrap(),
            ("A".to_string(), "B".to_string())
        );
        assert_eq!(
            get_property_from_string("A=\"C=D\"").unwrap(),
            ("A".to_string(), "C=D".to_string())
        );
        assert_eq!(
            get_property_from_string("A='C=D'").unwrap(),
            ("A".to_string(), "C=D".to_string())
        );
        assert_eq!(
            get_property_from_string("A=C=D").unwrap(),
            ("A".to_string(), "C=D".to_string())
        );
        assert_eq!(
            get_property_from_string("A=C D").unwrap(),
            ("A".to_string(), "C D".to_string())
        );
        assert!(get_property_from_string("#A=B").is_err());
        assert!(get_property_from_string("=B").is_err());
        assert!(get_property_from_string("A=").is_err());
        assert!(get_property_from_string("A='B\"").is_err());
        assert!(get_property_from_string("A=\"B'").is_err());
        assert!(get_property_from_string("A=\"B").is_err());
        assert!(get_property_from_string("A='B").is_err());
        assert!(get_property_from_string("A=B\"").is_err());
        assert!(get_property_from_string("A=B'").is_err());
    }

    #[test]
    fn test_replace_ifname() {
        assert_eq!(replace_ifname("aaa/bbb"), "aaa_bbb");
        assert_eq!(replace_ifname("aaa:bbb"), "aaa_bbb");
        assert_eq!(replace_ifname("aaa%bbb"), "aaa_bbb");
        assert_eq!(replace_ifname("aaa bbb"), "aaa_bbb");
    }
}
