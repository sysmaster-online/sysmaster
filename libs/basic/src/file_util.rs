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

//! the utils of the file operation
//!
use crate::error::*;
use std::fs::OpenOptions;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;

/// read first line from a file
pub fn read_first_line(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path).context(IoSnafu)?;

    let mut buffer = BufReader::new(file);
    let mut first_line = String::with_capacity(1024);
    let _ = buffer.read_line(&mut first_line);

    Ok(first_line)
}

/// write string to file
pub fn write_string_file<P: AsRef<Path>>(path: P, value: String) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).open(&path)?;

    let _ = file.write(value.as_bytes())?;

    Ok(())
}
