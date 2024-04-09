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

use std::{
    rc::Rc,
    sync::{Arc, RwLock},
};

use crate::{error::*, log_dev, rules::FormatSubstitutionType, utils::trie::*};
use basic::{error::errno_is_privilege, IN_SET};
use device::{Device, DB_BASE_DIR, DEFAULT_BASE_DIR};
use lazy_static::lazy_static;
use nix::{errno::Errno, unistd::unlink};
use snafu::ResultExt;

pub(crate) const DEVMASTER_LEGAL_CHARS: &str = "/ $%?,";

lazy_static! {
    static ref PLACEHOLDER_LONG_TRIE: Arc<RwLock<Trie<FormatSubstitutionType>>> =
        Trie::from_vec(vec![
            "devnode", "tempnode", "attr", "sysfs", "env", "kernel", "number", "driver", "devpath",
            "id", "major", "minor", "result", "parent", "name", "links", "root", "sys"
        ]);
    static ref PLACEHOLDER_SHORT_TRIE: Arc<RwLock<Trie<FormatSubstitutionType>>> =
        Trie::from_vec(vec![
            "N", "s", "E", "k", "n", "d", "p", "b", "M", "m", "c", "P", "D", "L", "r", "S"
        ]);
}

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
    let mut idx = 0;

    while idx < value.len() {
        let ch = &value[idx..idx + 1];
        if IN_SET!(ch, "%", "$") && idx + 1 < value.len() {
            let subst_type: FormatSubstitutionType;
            if &value[idx + 1..idx + 2]
                == match ch {
                    "%" => "%",
                    "$" => "$",
                    _ => panic!(),
                }
            {
                idx += 2;
                continue;
            }

            match Trie::search_prefix_partial(
                match ch {
                    "$" => PLACEHOLDER_LONG_TRIE.clone(),
                    "%" => PLACEHOLDER_SHORT_TRIE.clone(),
                    _ => panic!(),
                },
                &value[idx + 1..],
            ) {
                Some(node) => {
                    idx += node.read().unwrap().depth;
                    subst_type = node.read().unwrap().value.unwrap_or_default();
                }
                None => {
                    return Err(Error::RulesLoadError {
                        msg: format!("Key '{}': invalid long placeholder.", key),
                    });
                }
            }

            if IN_SET!(
                subst_type,
                FormatSubstitutionType::Attr,
                FormatSubstitutionType::Env,
                FormatSubstitutionType::Result
            ) {
                let mut attr = "".to_string();

                #[derive(PartialEq, Eq, PartialOrd, Ord)]
                enum State {
                    Left,
                    Attr,
                    Right,
                }

                let mut state = State::Left;

                for (i, c) in value[idx + 1..].chars().enumerate() {
                    match state {
                        State::Left => {
                            if c != '{' {
                                break;
                            }
                            state = State::Attr;
                        }
                        State::Attr => {
                            if c == '}' {
                                state = State::Right;
                                idx += i + 1;
                                break;
                            }
                            attr.push(c);
                        }
                        State::Right => {
                            panic!()
                        }
                    }
                }

                if state == State::Attr {
                    return Err(Error::RulesLoadError {
                        msg: format!("Key '{}': unmatched brackets.", key),
                    });
                }

                if subst_type == FormatSubstitutionType::Result {
                    if !attr.is_empty() {
                        let num = if attr.ends_with('+') {
                            attr[0..attr.len() - 1].parse::<i32>()
                        } else {
                            attr.parse::<i32>()
                        };

                        if num.is_err() {
                            return Err(Error::RulesLoadError {
                                msg: format!(
                                    "Key '{}': 'result' placeholder has invalid index.",
                                    key
                                ),
                            });
                        }
                    }
                } else if attr.is_empty() {
                    return Err(Error::RulesLoadError {
                        msg: format!("Key '{}': attribute is missing.", key),
                    });
                }
            }
        }

        idx += 1;
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

    let mut ret = "".to_string();

    let mut whitespace_continue: bool = false;
    for c in s.chars() {
        if c.is_ascii_whitespace() {
            if whitespace_continue {
                continue;
            }

            whitespace_continue = true;
            ret.push('_');
            continue;
        }

        whitespace_continue = false;
        ret.push(c);
    }

    ret
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
    if !s.starts_with('[') {
        return Err(Error::InvalidSubsystemKernel { s: s.to_string() });
    }

    let s = s.strip_prefix('[').unwrap();

    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    enum State {
        Subsystem,
        Sysname,
        Attribute,
    }

    let mut subsystem = "".to_string();
    let mut sysname = "".to_string();
    let mut attribute = "".to_string();
    let mut state = State::Subsystem;

    for ch in s.chars() {
        match state {
            State::Subsystem => {
                if ch == '/' {
                    state = State::Sysname;
                    continue;
                }

                subsystem.push(ch);
            }
            State::Sysname => {
                if ch == ']' {
                    state = State::Attribute;
                    continue;
                }

                sysname.push(ch);
            }
            State::Attribute => {
                attribute.push(ch);
            }
        }
    }

    if subsystem.is_empty() || sysname.is_empty() || state != State::Attribute {
        return Err(Error::InvalidSubsystemKernel { s: s.to_string() });
    }

    let attribute = attribute.trim_start_matches('/').to_string();

    if read && attribute.is_empty() {
        return Err(Error::InvalidSubsystemKernel { s: s.to_string() });
    }

    let device = Device::from_subsystem_sysname(&subsystem, &sysname).map_err(|e| {
        Error::InvalidSubsystemKernel {
            s: format!("{}: {}", s, e),
        }
    })?;

    if read {
        let attr_value = match device.get_sysattr_value(&attribute) {
            Ok(v) => v,
            Err(e) => {
                if e.get_errno() == nix::errno::Errno::ENOENT || errno_is_privilege(e.get_errno()) {
                    "".to_string()
                } else {
                    return Err(Error::InvalidSubsystemKernel { s: s.to_string() });
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

/// The property key should not contain any whitespace,
/// the property value can contain whitespaces.
pub(crate) fn get_property_from_string(s: &str) -> Result<(String, String)> {
    let s = s.trim();

    if s.starts_with('#') {
        return Err(Error::Other {
            msg: format!("ignore commented line '{}'", s),
            errno: Errno::EINVAL,
        });
    }

    let mut key = "".to_string();
    let mut value = "".to_string();

    enum StateMachine {
        Key,
        OpPre,
        OpPost,
        Value,
    }

    let mut state = StateMachine::Key;

    for ch in s.chars() {
        match state {
            StateMachine::Key => {
                if ch.is_ascii_whitespace() {
                    state = StateMachine::OpPre;
                    continue;
                }

                if ch == '=' {
                    state = StateMachine::OpPost;
                    continue;
                }

                key.push(ch);
            }
            StateMachine::OpPre => {
                if ch.is_ascii_whitespace() {
                    continue;
                }

                if ch == '=' {
                    state = StateMachine::OpPost;
                }
            }
            StateMachine::OpPost => {
                if ch.is_ascii_whitespace() {
                    continue;
                }
                state = StateMachine::Value;
                value.push(ch);
            }
            StateMachine::Value => {
                value.push(ch);
            }
        }
    }

    if key.is_empty() || value.is_empty() {
        return Err(Error::Other {
            msg: format!("key or value can not be empty '{}'", s),
            errno: Errno::EINVAL,
        });
    }

    if value.starts_with('"') || value.starts_with('\'') {
        let prefix_c = &value[0..1];
        let suffix_c = &value[value.len() - 1..];

        if prefix_c != suffix_c {
            return Err(Error::Other {
                msg: format!("unmatched quotes: {}", s),
                errno: Errno::EINVAL,
            });
        }

        Ok((key, value[1..value.len() - 1].to_string()))
    } else {
        if value.ends_with(|c| IN_SET!(c, '\"', '\'')) {
            return Err(Error::Other {
                msg: format!("unmatched quotes: {}", s),
                errno: Errno::EINVAL,
            });
        }
        Ok((key, value))
    }
}

/// inherit initial timestamp from old device object
pub(crate) fn initialize_device_usec(dev_new: Rc<Device>, dev_old: Rc<Device>) -> Result<()> {
    let timestamp = dev_old.get_usec_initialized().unwrap_or_else(|_| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |v| v.as_secs())
    });

    dev_new.set_usec_initialized(timestamp);

    Ok(())
}

/// add new tags and remove deleted tags
pub(crate) fn device_update_tag(
    dev_new: Rc<Device>,
    dev_old: Option<Rc<Device>>,
    add: bool,
) -> Result<()> {
    if let Some(dev_old) = dev_old {
        for tag in &dev_old.tag_iter() {
            if let Ok(true) = dev_new.has_tag(tag) {
                continue;
            }

            let _ = dev_new
                .update_tag(tag, false)
                .context(DeviceSnafu)
                .log_error(&format!("failed to remove old tag '{}'", tag));
        }
    }

    for tag in &dev_new.tag_iter() {
        let _ = dev_new
            .update_tag(tag, add)
            .context(DeviceSnafu)
            .log_error(&format!("failed to add new tag '{}'", tag));
    }

    Ok(())
}

/// cleanup device database
pub(crate) fn cleanup_db(dev: Rc<Device>) -> Result<()> {
    let id = dev.get_device_id().context(DeviceSnafu)?;

    let db_path = format!("{}/{}/{}", DEFAULT_BASE_DIR, DB_BASE_DIR, id);

    match unlink(db_path.as_str()) {
        Ok(_) => log_dev!(debug, dev, format!("unlinked '{}'", db_path)),
        Err(e) => {
            if e != nix::Error::ENOENT {
                log_dev!(
                    error,
                    dev,
                    format!("failed to unlink '{}' when cleanup db: {}", db_path, e)
                );
                return Err(Error::Nix { source: e });
            }
        }
    }

    Ok(())
}

/// Check whether every character in string satisfy the condition.
pub(crate) fn str_satisfy<F>(s: &str, f: F) -> bool
where
    F: Copy + FnOnce(char) -> bool,
{
    for ch in s.chars() {
        if !f(ch) {
            return false;
        }
    }

    true
}

/// Find the first component in the path, with the prefix removed.
pub(crate) fn get_first_path_component<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    s.strip_prefix(prefix).map(|suffix| match suffix.find('/') {
        Some(i) => &suffix[0..i],
        None => suffix,
    })
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

        check_value_format("", "aaa$s{xxx}ccc", false).unwrap_err();
        check_value_format("", "aaa$c{0}ccc", false).unwrap_err();
        check_value_format("", "aaa$c{0+}ccc", false).unwrap_err();

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
            get_property_from_string("A = B").unwrap(),
            ("A".to_string(), "B".to_string())
        );
        assert_eq!(
            get_property_from_string("A  =  B").unwrap(),
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
            get_property_from_string("A  =  \"B\"").unwrap(),
            ("A".to_string(), "B".to_string())
        );
        assert_eq!(
            get_property_from_string("A='B'").unwrap(),
            ("A".to_string(), "B".to_string())
        );
        assert_eq!(
            get_property_from_string("A  =  'B'").unwrap(),
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

    #[test]
    fn test_replace_whitespace() {
        assert_eq!(&replace_whitespace("hello world"), "hello_world");
        assert_eq!(&replace_whitespace("hello world "), "hello_world");
        assert_eq!(&replace_whitespace(" hello world"), "_hello_world");
        assert_eq!(&replace_whitespace(""), "");
        assert_eq!(&replace_whitespace("hello   world"), "hello_world");
        assert_eq!(&replace_whitespace("hello world   "), "hello_world");
        assert_eq!(&replace_whitespace("   hello   world"), "_hello_world");
        assert_eq!(&replace_whitespace("hello   world   hi"), "hello_world_hi");
        assert_eq!(&replace_whitespace("        "), "");
        assert_eq!(&replace_whitespace("a        "), "a");
        assert_eq!(&replace_whitespace("        a"), "_a");
    }

    #[test]
    fn test_str_satisfy() {
        assert!(str_satisfy("aaa", |c| c.is_ascii_lowercase()));
        assert!(str_satisfy("123", |c| c.is_ascii_digit()));
    }

    #[test]
    fn test_get_first_path_component() {
        assert_eq!(
            get_first_path_component("/sys/class/device/10002000", "/sys/class/device/").unwrap(),
            "10002000"
        );
        assert_eq!(
            get_first_path_component("/sys/class/device/10002000/aaa", "/sys/class/device/")
                .unwrap(),
            "10002000"
        );
        assert_eq!(
            get_first_path_component("/sys/class/device/10002000/aaa/bbb", "/sys/class/device/")
                .unwrap(),
            "10002000"
        );
    }
}
