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
use regex::Regex;
use serde::{
    de::{self, Unexpected},
    Deserialize, Deserializer, Serialize,
};
use std::{collections::VecDeque, path::Path};

/// the exec command that was parsed from the unit file
#[derive(PartialEq, Clone, Eq, Debug, Serialize, Deserialize)]
pub struct ExecCommand {
    path: String,
    argv: Vec<String>,
}

impl ExecCommand {
    /// create a new instance of the command
    pub fn new(path: String, argv: Vec<String>) -> ExecCommand {
        ExecCommand { path, argv }
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
        let s = String::deserialize(de)?;

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

            if path.is_absolute() && !path.exists() {
                return Err(de::Error::invalid_value(
                    Unexpected::Str(&exec_cmd),
                    &"no exist absolute path",
                ));
            }

            let cmd = path.to_str().unwrap().to_string();
            let new_command = ExecCommand::new(cmd, command);
            commands.push_back(new_command);
        }

        Ok(commands)
    }
}
