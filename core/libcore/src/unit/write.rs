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

use super::base::UnitBase;
use super::deps::{self, UnitWriteFlags};
use crate::error::*;
use std::fmt::Write as _;
use std::fmt::{self, Arguments};
use std::fs::OpenOptions;
use std::io::Write as _;
use std::rc::Rc;

///
pub fn unit_write_setting(
    unit: Rc<dyn UnitBase>,
    ps: &str,
    flags: UnitWriteFlags,
    _name: &str,
    data: &str,
) -> Result<()> {
    if deps::unit_write_flags_is_noop(flags) {
        return Ok(());
    }

    let mut prefix = String::new();
    let private_section = String::from(ps);
    let transient_file = unit.transient_file();
    let last_section_private = unit.last_section_private();

    if flags.contains(UnitWriteFlags::PRIVATE) {
        if private_section.is_empty() {
            return Err(Error::InvalidData);
        }

        if transient_file.is_none() || last_section_private < 0 {
            //write!(&mut prefix, "[{}]\n", private_section).context(FmtSnafu)?;
            writeln!(&mut prefix, "[{}]", private_section).context(FmtSnafu)?;
        } else if last_section_private == 0 {
            //write!(&mut prefix, "\n[{}]\n", private_section).context(FmtSnafu)?;
            writeln!(&mut prefix, "\n[{}]", private_section).context(FmtSnafu)?;
        }
    } else if !flags.is_empty() {
        // make clippy "collapsible_else_if" happy
        if transient_file.is_none() || last_section_private < 0 {
            //write!(&mut prefix, "[Unit]\n").context(FmtSnafu)?;
            writeln!(&mut prefix, "[Unit]").context(FmtSnafu)?;
        } else if last_section_private > 0 {
            //write!(&mut prefix, "\n[Unit]\n").context(FmtSnafu)?;
            writeln!(&mut prefix, "\n[Unit]").context(FmtSnafu)?;
        }
    } else {
        // guaranteed by unit_write_flags_is_noop.
        unreachable!();
    }

    if let Some(tf) = transient_file {
        let mut file = OpenOptions::new().append(true).open(tf).context(IoSnafu)?;
        write!(file, "{}{}", prefix, data).context(IoSnafu)?;
        if !data.ends_with('\n') {
            //write!(file, "\n").context(IoSnafu)?;
            writeln!(file).context(IoSnafu)?; // append \n
        }

        unit.set_last_section_private(flags.contains(UnitWriteFlags::PRIVATE) as i8);
        return Ok(());
    }

    // not supported now
    Err(Error::NotSupported)
}

///
pub fn unit_write_settingf(
    unit: Rc<dyn UnitBase>,
    ps: &str,
    flags: UnitWriteFlags,
    name: &str,
    args: Arguments<'_>,
) -> Result<()> {
    if deps::unit_write_flags_is_noop(flags) {
        return Ok(());
    }

    let mut data = String::new();
    fmt::write(&mut data, args).context(FmtSnafu)?;
    unit_write_setting(unit, ps, flags, name, &data)
}
