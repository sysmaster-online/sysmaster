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

use std::{
    io::Read,
    process::{Command, Stdio},
    time::Duration,
};

use crate::{error::*, rules::FormatSubstitutionType};
use basic::errno_util::errno_is_privilege;
use device::Device;
use lazy_static::lazy_static;
use nix::errno::Errno;
use regex::Regex;
use shell_words::split;
use snafu::ResultExt;
use wait_timeout::ChildExt;

pub(crate) mod loop_device;
pub(crate) mod macros;

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
pub(crate) fn replace_chars(s: &str, white_list: &str) -> String {
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
pub(crate) fn replace_whitespace(s: &str) -> String {
    // Create a regular expression to match one or more whitespace characters.
    let re = Regex::new(r"\s+").unwrap();
    // Use the regular expression to replace all matches with underscores.
    // The resulting string is converted to a String and returned.
    re.replace_all(s, "_").to_string()
}

/// This function encodes a device node name string into a byte array.
/// The encoded string is stored in the output string buffer.
/// If the input string or output buffer is empty, an error is returned.
pub(crate) fn encode_devnode_name(str: &str, str_enc: &mut String) {
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
        let sysname = $d.get_sysname().unwrap_or("no_sysname").to_string();
        let syspath = $d.get_syspath().unwrap_or("no_syspath").to_string();
        let subsystem = $d.get_subsystem().unwrap_or("no_subsystem".to_string());
        format!("{}: {} {} {} {}", $p, action, sysname, syspath, subsystem)
    }};
}

/// resolve [<SUBSYSTEM>/<KERNEL>]<attribute> string
/// if 'read' is true, read the attribute from sysfs device tree and return the value
/// else just return the attribute path under sysfs device tree.
pub(crate) fn resolve_subsystem_kernel(s: &String, read: bool) -> Result<String> {
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
                    msg: format!("can not read empty sysattr: '{s}'",),
                    errno: nix::errno::Errno::EINVAL,
                });
            }

            let mut device = Device::from_subsystem_sysname(subsystem.clone(), sysname.clone())
                .map_err(|e| Error::Other {
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
                    "the sysattr value of '[{subsystem}/{sysname}]{attribute}' is '{attr_value}'"
                );
                Ok(attr_value)
            } else {
                let syspath = device.get_syspath().context(DeviceSnafu)?.to_string();

                let attr_path = if attribute.is_empty() {
                    syspath
                } else {
                    syspath + "/" + attribute.as_str()
                };
                log::debug!("resolve path '[{subsystem}/{sysname}]{attribute}' as '{attr_path}'");
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

/// if the command is not absolute path, try to find it under lib directory first.
pub(crate) fn spawn(cmd_str: &String, timeout: Duration) -> Result<(String, i32)> {
    lazy_static! {
        static ref LIB_DIRS: Vec<String> =
            vec!["/lib/udev/".to_string(), "/lib/devmaster/".to_string()];
    }

    let cmd_tokens = split(cmd_str).map_err(|e| Error::Other {
        msg: format!(
            "failed to split command '{}' into shell tokens: ({})",
            cmd_str, e
        ),
        errno: nix::errno::Errno::EINVAL,
    })?;

    if cmd_tokens.is_empty() {
        return Err(Error::Other {
            msg: "failed to spawn empty command.".to_string(),
            errno: nix::errno::Errno::EINVAL,
        });
    }

    let mut cmd = if !cmd_tokens[0].starts_with('/') {
        LIB_DIRS
            .iter()
            .map(|lib| lib.clone() + cmd_tokens[0].as_str())
            .find(|path| std::fs::metadata(path).is_ok())
            .map_or_else(|| Command::new(&cmd_tokens[0]), Command::new)
    } else {
        Command::new(&cmd_tokens[0])
    };

    let cmd = cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    for arg in &cmd_tokens[1..] {
        cmd.arg(arg);
    }

    let mut child = cmd.spawn().map_err(|e| Error::Other {
        msg: format!("failed to spawn command '{:?}': {}", cmd, e),
        errno: nix::errno::Errno::EINVAL,
    })?;
    let pid = child.id();

    match child.wait_timeout(timeout).map_err(|e| Error::Other {
        msg: format!("failed to kill child process {} '{:?}': ({})", pid, cmd, e),
        errno: nix::errno::Errno::EINVAL,
    })? {
        Some(status) => {
            log::debug!("Process {} exited with status {:?}", pid, status);
            // status.code()
            let mut stdout = child.stdout.take().unwrap();
            let mut stderr = child.stderr.take().unwrap();
            let retno = status.code().unwrap();

            let mut stdout_buf = String::new();
            stdout
                .read_to_string(&mut stdout_buf)
                .map_err(|e| Error::Other {
                    msg: format!(
                        "failed to read stdout for child process {} '{:?}': ({})",
                        pid, cmd, e
                    ),
                    errno: nix::errno::Errno::EINVAL,
                })?;

            let mut stderr_buf = String::new();
            stderr
                .read_to_string(&mut stderr_buf)
                .map_err(|e| Error::Other {
                    msg: format!(
                        "failed to read stderr for child process {} '{:?}': ({})",
                        pid, cmd, e
                    ),
                    errno: nix::errno::Errno::EINVAL,
                })?;

            if !stderr_buf.is_empty() {
                log::debug!(
                    "stderr from child process {} {:?}: {}",
                    pid,
                    cmd,
                    stderr_buf
                );
            }

            Ok((stdout_buf, retno))
        }
        None => {
            child.kill().map_err(|e| Error::Other {
                msg: format!("failed to kill child process {} '{:?}': ({})", pid, cmd, e),
                errno: nix::errno::Errno::EINVAL,
            })?;
            Err(Error::Other {
                msg: format!("child process {} '{:?}' timed out", pid, cmd),
                errno: nix::errno::Errno::EINVAL,
            })
        }
    }
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

    if value.starts_with(['"', '\'']) {
        Ok((key.to_string(), value[1..value.len() - 1].to_string()))
    } else {
        Ok((key.to_string(), value[0..].to_string()))
    }
}

#[cfg(test)]
mod tests {
    use basic::logger::init_log_to_console;
    use device::Device;
    use log::LevelFilter;

    use super::*;

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

        device_trace!("test", device);
    }

    #[test]
    #[ignore]
    fn test_resolve_subsystem_kernel() {
        assert!(resolve_subsystem_kernel(&"[net]".to_string(), false).is_err());
        assert!(resolve_subsystem_kernel(&"[net/]".to_string(), false).is_err());
        assert!(resolve_subsystem_kernel(&"[net/lo".to_string(), false).is_err());
        assert!(resolve_subsystem_kernel(&"[net".to_string(), false).is_err());
        assert!(resolve_subsystem_kernel(&"net/lo".to_string(), false).is_err());

        assert_eq!(
            resolve_subsystem_kernel(&"[net/lo]".to_string(), false).unwrap(),
            "/sys/devices/virtual/net/lo"
        );
        assert_eq!(
            resolve_subsystem_kernel(&"[net/lo]/".to_string(), false).unwrap(),
            "/sys/devices/virtual/net/lo"
        );
        assert_eq!(
            resolve_subsystem_kernel(&"[net/lo]hoge".to_string(), false).unwrap(),
            "/sys/devices/virtual/net/lo/hoge"
        );
        assert_eq!(
            resolve_subsystem_kernel(&"[net/lo]/hoge".to_string(), false).unwrap(),
            "/sys/devices/virtual/net/lo/hoge"
        );

        assert!(resolve_subsystem_kernel(&"[net/lo]".to_string(), true).is_err());
        assert!(resolve_subsystem_kernel(&"[net/lo]/".to_string(), true).is_err());
        assert_eq!(
            resolve_subsystem_kernel(&"[net/lo]hoge".to_string(), true).unwrap(),
            ""
        );
        assert_eq!(
            resolve_subsystem_kernel(&"[net/lo]/hoge".to_string(), true).unwrap(),
            ""
        );
        assert_eq!(
            resolve_subsystem_kernel(&"[net/lo]address".to_string(), true).unwrap(),
            "00:00:00:00:00:00"
        );
        assert_eq!(
            resolve_subsystem_kernel(&"[net/lo]/address".to_string(), true).unwrap(),
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
        let device = Device::from_path("/dev/sda".to_string()).unwrap();
        let syspath = device.get_syspath().unwrap();
        println!(
            "{}",
            sysattr_subdir_subst(&(syspath.to_string() + "/sda1/*/runtime_status")).unwrap()
        );
    }

    #[test]
    #[ignore]
    fn test_spawn() {
        init_log_to_console("test_spawn", LevelFilter::Debug);

        println!(
            "{}",
            spawn(&"echo hello world".to_string(), Duration::from_secs(1),)
                .unwrap()
                .0
        );

        println!(
            "{}",
            spawn(&"/bin/echo hello world".to_string(), Duration::from_secs(1),)
                .unwrap()
                .0
        );

        println!(
            "{}",
            spawn(&"sleep 2".to_string(), Duration::from_secs(1),).unwrap_err()
        );

        println!(
            "{}",
            spawn(&"sleep 1".to_string(), Duration::from_secs(10),)
                .unwrap()
                .0
        );

        println!(
            "{}",
            spawn(
                &"sh -c '/bin/echo test shell'".to_string(),
                Duration::from_secs(1),
            )
            .unwrap()
            .0
        );

        // scsi_id is provided by udev and is located in /lib/udev/
        println!(
            "{}",
            spawn(
                &"scsi_id --export --whitelisted -d /dev/sda".to_string(),
                Duration::from_secs(1),
            )
            .unwrap()
            .0
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
