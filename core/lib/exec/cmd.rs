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
use bitflags::bitflags;
use regex::Regex;
use serde::{
    de::{self, Unexpected},
    Deserialize, Deserializer, Serialize,
};
use std::{collections::VecDeque, path::Path};

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
            argv: vec![String::new()],
            flags: ExecFlag::EXEC_COMMAND_EMPTY,
        }
    }
    ///
    pub fn add_exec_flag(&mut self, flag: ExecFlag) {
        self.flags |= flag;
    }
    ///
    pub fn get_exec_flag(&self) -> ExecFlag {
        self.flags
    }

    /// return the path of the command
    pub fn path(&self) -> &String {
        &self.path
    }

    /// return the arguments of the command
    pub fn argv(&self) -> Vec<&String> {
        self.argv.iter().collect::<Vec<_>>()
    }
}

impl DeserializeWith for ExecCommand {
    type Item = VecDeque<Self>;
    fn deserialize_with<'de, D>(de: D) -> Result<Self::Item, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut s = String::deserialize(de)?;

        let first = match s.as_bytes().first() {
            None => {
                return Err(de::Error::invalid_value(
                    Unexpected::Str(&s),
                    &"The configured value is empty.",
                ));
            }
            Some(v) => *v,
        };

        let exec_flag = match first as char {
            '-' => {
                s = s.trim_start_matches('-').to_string();
                ExecFlag::EXEC_COMMAND_IGNORE_FAILURE
            }
            _ => ExecFlag::EXEC_COMMAND_EMPTY,
        };

        let mut commands = VecDeque::new();

        for cmd in s.trim().split_terminator(';') {
            if cmd.is_empty() {
                continue;
            }

            #[allow(clippy::trim_split_whitespace)]
            let mut command: Vec<String> = Vec::new();
            let re = Regex::new(r"'([^']*)'|\S+").unwrap();
            for cap in re.captures_iter(cmd) {
                if let Some(s) = cap.get(1) {
                    command.push(s.as_str().to_string());
                    continue;
                }

                if let Some(s) = cap.get(0) {
                    command.push(s.as_str().to_string());
                }
            }

            // get the command and leave the command args
            let exec_cmd = command.remove(0);
            let path = Path::new(&exec_cmd);

            if !path.is_absolute() {
                return Err(de::Error::invalid_value(
                    Unexpected::Str(&exec_cmd),
                    &"only accept absolute path",
                ));
            }

            let cmd = path.to_str().unwrap().to_string();
            let mut new_command = ExecCommand::new(cmd, command);
            new_command.add_exec_flag(exec_flag);
            commands.push_back(new_command);
        }

        Ok(commands)
    }
}
