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

use crate::serialize::DeserializeWith;
use crate::specifier::{
    unit_string_specifier_escape, unit_strings_specifier_escape, UnitSpecifierData,
};
use basic::fs::{path_is_abosolute, path_simplify};
use basic::{fs::parse_absolute_path, Error, Result};
use bitflags::bitflags;
use serde::{
    de::{self, Unexpected},
    Deserialize, Deserializer, Serialize,
};
use std::collections::VecDeque;

bitflags! {
    /// ExecCommand Flags
    #[derive(Serialize, Deserialize)]
    pub struct ExecFlag: u8 {
        ///
        const EXEC_COMMAND_EMPTY = 0;
        ///
        const EXEC_COMMAND_IGNORE_FAILURE   = 1 << 0;
        ///
        const EXEC_COMMAND_FULLY_PRIVILEGED = 1 << 1;
        ///
        const EXEC_COMMAND_NO_SETUID        = 1 << 2;
        ///
        const EXEC_COMMAND_AMBIENT_MAGIC    = 1 << 3;
        ///
        const EXEC_COMMAND_NO_ENV_EXPAND    = 1 << 4;
    }
}

/// the exec command that was parsed from the unit file
#[derive(PartialEq, Clone, Eq, Debug, Serialize, Deserialize)]
pub struct ExecCommand {
    path: String,
    argv: Vec<String>,
    flags: ExecFlag,
}

impl ExecCommand {
    /// create a new instance of the command
    pub fn new(path: String, argv: Vec<String>) -> ExecCommand {
        ExecCommand {
            path,
            argv,
            flags: ExecFlag::EXEC_COMMAND_EMPTY,
        }
    }
    ///
    pub fn empty() -> ExecCommand {
        ExecCommand {
            path: String::new(),
            argv: Vec::new(),
            flags: ExecFlag::EXEC_COMMAND_EMPTY,
        }
    }
    ///
    pub fn add_exec_flag(&mut self, flag: ExecFlag) {
        self.flags |= flag;
    }
    ///
    pub fn append_argv(&mut self, argv: &str) {
        self.argv.push(String::from(argv));
    }
    ///
    pub fn append_many_argv(&mut self, argvs: Vec<&str>) {
        self.argv.extend(argvs.iter().map(|x| x.to_string()))
    }
    ///
    pub fn get_exec_flag(&self) -> ExecFlag {
        self.flags
    }
    ///
    pub fn set_path(&mut self, path: &str) -> Result<()> {
        if !path_is_abosolute(path) {
            return Err(Error::Invalid {
                what: "ExecCmd path should be absolute".to_string(),
            });
        }
        let v = match path_simplify(path) {
            None => {
                return Err(Error::Invalid {
                    what: "Invalid ExecCmd path".to_string(),
                })
            }
            Some(v) => v,
        };
        self.path = v;
        Ok(())
    }
    /// return the path of the command
    pub fn path(&self) -> &String {
        &self.path
    }

    /// return the arguments of the command
    pub fn argv(&self) -> Vec<&String> {
        self.argv.iter().collect::<Vec<_>>()
    }

    /// escape the specifier of ExecCommand
    pub fn specifier_escape_full(
        &mut self,
        max_len: usize,
        unit_specifier_data: &UnitSpecifierData,
    ) {
        if let Ok(ret) = unit_string_specifier_escape(&self.path, max_len, unit_specifier_data) {
            self.path = ret;
        }

        if let Ok(ret) = unit_strings_specifier_escape(&self.argv, max_len, unit_specifier_data) {
            self.argv = ret;
        }
    }
}

///
pub fn parse_exec_command(s: &str) -> Result<Vec<ExecCommand>> {
    match parse_exec(s) {
        Ok(v) => Ok(v),
        Err(e) => {
            log::error!("Failed to parse ExecCommand: {}", e);
            Err(e)
        }
    }
}

fn parse_exec(s: &str) -> Result<Vec<ExecCommand>> {
    if s.is_empty() {
        return Err(Error::Invalid {
            what: "empty string".to_string(),
        });
    }

    let mut res = Vec::new();
    let mut next_start = 0_usize;
    loop {
        let mut flags = ExecFlag::EXEC_COMMAND_EMPTY;
        /* TODO: separate_argv0 is currently not used. */
        let mut separate_argv0 = false;
        let content = s.split_at(next_start).1;
        if content.is_empty() {
            break;
        }
        /* Take "-/bin/echo good" for example, content is "-bin/echo good" */
        let mut path_start = next_start;
        for f in content.as_bytes() {
            if *f == b' ' {
                path_start += 1;
                continue;
            }
            if *f == b'-' && !flags.intersects(ExecFlag::EXEC_COMMAND_IGNORE_FAILURE) {
                flags |= ExecFlag::EXEC_COMMAND_IGNORE_FAILURE;
            } else if *f == b'@' && !separate_argv0 {
                separate_argv0 = true;
            } else if *f == b':' && !flags.intersects(ExecFlag::EXEC_COMMAND_NO_ENV_EXPAND) {
                flags |= ExecFlag::EXEC_COMMAND_NO_ENV_EXPAND;
            } else if *f == b'+'
                && !flags.intersects(
                    ExecFlag::EXEC_COMMAND_FULLY_PRIVILEGED
                        | ExecFlag::EXEC_COMMAND_NO_SETUID
                        | ExecFlag::EXEC_COMMAND_AMBIENT_MAGIC,
                )
            {
                flags |= ExecFlag::EXEC_COMMAND_FULLY_PRIVILEGED;
            } else if *f == b'!'
                && !flags.intersects(
                    ExecFlag::EXEC_COMMAND_FULLY_PRIVILEGED
                        | ExecFlag::EXEC_COMMAND_NO_SETUID
                        | ExecFlag::EXEC_COMMAND_AMBIENT_MAGIC,
                )
            {
                flags |= ExecFlag::EXEC_COMMAND_NO_SETUID;
            } else if *f == b'!'
                && !flags.intersects(
                    ExecFlag::EXEC_COMMAND_FULLY_PRIVILEGED | ExecFlag::EXEC_COMMAND_AMBIENT_MAGIC,
                )
            {
                flags &= !ExecFlag::EXEC_COMMAND_NO_SETUID;
                flags |= ExecFlag::EXEC_COMMAND_AMBIENT_MAGIC;
            } else {
                break;
            }
            path_start += 1;
        }

        /* content is "/bin/echo good" */
        let content = s.split_at(path_start).1;
        if content.is_empty() {
            return Err(Error::Invalid {
                what: "empty exec command".to_string(),
            });
        }
        /* path is "/bin/echo" */
        let path_str = match content.split_once(' ') {
            None => content,
            Some((v0, _v1)) => v0,
        };

        let path = parse_absolute_path(path_str)?;

        /* content is "good" */
        let mut argv_start = path_start + path_str.len();
        let content = s.split_at(argv_start).1;
        if content.is_empty() {
            res.push(ExecCommand {
                path,
                argv: vec![],
                flags,
            });
            break;
        }

        /* Get the command arg values */
        let mut argv: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut found_semicolon_wait_space = false;
        let mut found_single_quote = false;
        for c in content.chars() {
            argv_start += 1;

            if found_single_quote && c != '\'' {
                cur += &c.to_string();
                continue;
            }
            if c == '\'' {
                if found_single_quote {
                    argv.push(cur);
                    cur = "".to_string();
                    found_single_quote = false;
                    continue;
                }
                found_single_quote = true;
                continue;
            }
            if c == ' ' {
                /* now we find " ; ", break the loop */
                if found_semicolon_wait_space {
                    cur = String::new();
                    break;
                }
                if !cur.is_empty() {
                    argv.push(cur);
                    cur = "".to_string();
                }
                continue;
            }
            if c == ';' {
                /* \; is ; */
                if cur == "\\" {
                    found_semicolon_wait_space = false;
                    cur = ";".to_string();
                } else if !cur.is_empty() {
                    found_semicolon_wait_space = false;
                    cur += ";";
                } else {
                    /* " ;", wait for another space */
                    found_semicolon_wait_space = true;
                    cur += ";";
                }
                continue;
            }
            found_semicolon_wait_space = false;
            cur += &c.to_string();
        }

        if found_single_quote {
            return Err(Error::Invalid {
                what: "no valid exec command, wrong single quote".to_string(),
            });
        }
        /* No more characters after " ;", drop current argv */
        if found_semicolon_wait_space {
            cur = String::new();
        }

        if !cur.is_empty() {
            argv.push(cur);
        }

        res.push(ExecCommand {
            path: path.to_string(),
            argv,
            flags,
        });

        /* argv_start is at the first character after " ; " */
        next_start = argv_start;
    }

    if res.is_empty() {
        return Err(Error::Invalid {
            what: "no valid exec command".to_string(),
        });
    }

    Ok(res)
}

impl DeserializeWith for ExecCommand {
    type Item = VecDeque<Self>;
    fn deserialize_with<'de, D>(de: D) -> Result<Self::Item, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;
        match parse_exec(&s) {
            Ok(v) => {
                let mut res = VecDeque::new();
                for cmd in v {
                    res.push_back(cmd);
                }
                Ok(res)
            }
            Err(e) => {
                log::error!("Failed to parse ExecCommand: {}", e);
                return Err(de::Error::invalid_value(Unexpected::Str(&s), &""));
            }
        }
    }
}

mod tests {
    #[test]
    fn test_exec() {
        use crate::exec::{cmd::parse_exec, ExecCommand, ExecFlag};
        /* One command */
        assert_eq!(
            parse_exec("/bin/echo").unwrap(),
            Vec::from([ExecCommand {
                path: "/bin/echo".to_string(),
                argv: vec![],
                flags: ExecFlag::EXEC_COMMAND_EMPTY
            }])
        );

        assert_eq!(
            parse_exec("!!/bin/echo").unwrap(),
            Vec::from([ExecCommand {
                path: "/bin/echo".to_string(),
                argv: vec![],
                flags: ExecFlag::EXEC_COMMAND_AMBIENT_MAGIC
            }])
        );

        assert_eq!(
            parse_exec("-!/bin/echo").unwrap(),
            Vec::from([ExecCommand {
                path: "/bin/echo".to_string(),
                argv: vec![],
                flags: ExecFlag::EXEC_COMMAND_IGNORE_FAILURE | ExecFlag::EXEC_COMMAND_NO_SETUID
            }])
        );

        assert_eq!(
            parse_exec("/bin/echo good1 good2").unwrap(),
            Vec::from([ExecCommand {
                path: "/bin/echo".to_string(),
                argv: vec!["good1".to_string(), "good2".to_string()],
                flags: ExecFlag::EXEC_COMMAND_EMPTY
            }])
        );

        assert_eq!(
            parse_exec("/bin/echo good1;good2").unwrap(),
            Vec::from([ExecCommand {
                path: "/bin/echo".to_string(),
                argv: vec!["good1;good2".to_string()],
                flags: ExecFlag::EXEC_COMMAND_EMPTY
            }])
        );

        assert_eq!(
            parse_exec("/bin/echo ;/bin/echo good").unwrap(),
            Vec::from([ExecCommand {
                path: "/bin/echo".to_string(),
                argv: vec![";/bin/echo".to_string(), "good".to_string()],
                flags: ExecFlag::EXEC_COMMAND_EMPTY
            }])
        );

        assert_eq!(
            parse_exec("/bin/echo good1\\;good2").unwrap(),
            Vec::from([ExecCommand {
                path: "/bin/echo".to_string(),
                argv: vec!["good1\\;good2".to_string()],
                flags: ExecFlag::EXEC_COMMAND_EMPTY
            }])
        );

        assert_eq!(
            parse_exec("/bin/echo \\;").unwrap(),
            Vec::from([ExecCommand {
                path: "/bin/echo".to_string(),
                argv: vec![";".to_string()],
                flags: ExecFlag::EXEC_COMMAND_EMPTY
            }])
        );

        assert_eq!(
            parse_exec("/bin/echo good \\; /bin/echo good1 good2").unwrap(),
            Vec::from([ExecCommand {
                path: "/bin/echo".to_string(),
                argv: vec![
                    "good".to_string(),
                    ";".to_string(),
                    "/bin/echo".to_string(),
                    "good1".to_string(),
                    "good2".to_string()
                ],
                flags: ExecFlag::EXEC_COMMAND_EMPTY
            }])
        );

        assert_eq!(
            parse_exec("/bin/echo good ;").unwrap(),
            Vec::from([ExecCommand {
                path: "/bin/echo".to_string(),
                argv: vec!["good".to_string()],
                flags: ExecFlag::EXEC_COMMAND_EMPTY
            }])
        );

        /* Many commands */
        assert_eq!(
            parse_exec("/bin/echo good ; /bin/echo good1 good2 ; /bin/echo").unwrap(),
            Vec::from([
                ExecCommand {
                    path: "/bin/echo".to_string(),
                    argv: vec!["good".to_string()],
                    flags: ExecFlag::EXEC_COMMAND_EMPTY
                },
                ExecCommand {
                    path: "/bin/echo".to_string(),
                    argv: vec!["good1".to_string(), "good2".to_string()],
                    flags: ExecFlag::EXEC_COMMAND_EMPTY
                },
                ExecCommand {
                    path: "/bin/echo".to_string(),
                    argv: vec![],
                    flags: ExecFlag::EXEC_COMMAND_EMPTY
                }
            ])
        );

        /* Error command */
        assert!(parse_exec("echo good \\; /bin/echo good1 good2").is_err());
        assert!(parse_exec("; /bin/echo good1 good2").is_err());
        assert!(parse_exec("--/bin/echo good").is_err());
        assert!(parse_exec("-+!@  /bin/echo good1 good2").is_err());
        assert!(parse_exec("/bin/echo\x7f good1 good2").is_err());

        let path = "/a/".to_string() + &String::from_iter(vec!['1'; 255]);
        assert!(parse_exec(&path).is_ok());

        let path = "/a/".to_string() + &String::from_iter(vec!['1'; 256]);
        assert!(parse_exec(&path).is_err());

        let mut path = "".to_string();
        for _ in 0..41 {
            path += "/";
            path += &String::from_iter(vec!['1'; 100]);
        }
        assert!(parse_exec(&path).is_err());

        assert!(parse_exec("/bin/echo good ; ; ; ;").is_err());
        assert!(parse_exec("/bin/echo 'good1 good2").is_err());
        assert!(parse_exec("/bin/echo 'good good1' 'good2").is_err());
        assert_eq!(
            parse_exec("/bin/echo 'good good1' good2").unwrap(),
            Vec::from([ExecCommand {
                path: "/bin/echo".to_string(),
                argv: vec!["good good1".to_string(), "good2".to_string()],
                flags: ExecFlag::EXEC_COMMAND_EMPTY
            }])
        );
        assert_eq!(
            parse_exec("/bin/echo 'good good1' 'good2'").unwrap(),
            Vec::from([ExecCommand {
                path: "/bin/echo".to_string(),
                argv: vec!["good good1".to_string(), "good2".to_string()],
                flags: ExecFlag::EXEC_COMMAND_EMPTY
            }])
        );
    }
}
