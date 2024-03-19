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

use super::super::job::JobManager;
use super::entry::UnitX;
use super::rentry::{self, UnitLoadState};
use super::submanager::UnitSubManagers;
use super::uload::UnitLoad;
use basic::fs::RUN_TRANSIENT_PATH;
use cmdproto::proto::transient_unit_comm::UnitProperty;
use core::error::*;
use core::rel::Reliability;
use core::unit::{self, UnitType, UnitWriteFlags};
use nix::sys::stat::{self, Mode};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct UnitBus {
    // associated objects
    reli: Rc<Reliability>,
    load: Rc<UnitLoad>,
    jm: Rc<JobManager>,
    sms: Rc<UnitSubManagers>,
    // owned objects
}

impl UnitBus {
    pub(super) fn new(
        relir: &Rc<Reliability>,
        loadr: &Rc<UnitLoad>,
        jmr: &Rc<JobManager>,
        smsr: &Rc<UnitSubManagers>,
    ) -> UnitBus {
        UnitBus {
            reli: Rc::clone(relir),
            load: Rc::clone(loadr),
            jm: Rc::clone(jmr),
            sms: Rc::clone(smsr),
        }
    }

    pub(super) fn transient_unit_from_message(
        &self,
        properties: &[UnitProperty],
        name: &str,
    ) -> Result<Rc<UnitX>> {
        let unit_type = rentry::unit_name_to_type(name);
        if unit_type == UnitType::UnitTypeInvalid {
            return Err(Error::InvalidData);
        }

        if !self.sms.can_transient(unit_type) {
            return Err(Error::InvalidData);
        }

        // the first incomplete loading
        let unit = match self.load.load_unit(name) {
            None => {
                return Err(Error::UnitActionENoent);
            }
            Some(v) => v,
        };

        // prevent duplicate actions
        if !self.unit_is_pristine(&unit) {
            return Err(Error::UnitActionEAlready);
        }

        // set configuration file of the transient unit
        self.unit_make_transient(&unit, properties)?;

        // the second real loading
        self.load.load_update(&unit);

        Ok(unit)
    }

    fn unit_is_pristine(&self, unit: &Rc<UnitX>) -> bool {
        let load_state = unit.load_state();
        let load = load_state == UnitLoadState::Loaded || load_state == UnitLoadState::NotFound;
        let exist = !load
            || !unit.load_paths().is_empty()
            || unit.merged_into().is_some()
            || self.jm.has_job(unit);
        !exist
    }

    fn unit_make_transient(&self, unit: &Rc<UnitX>, properties: &[UnitProperty]) -> Result<()> {
        self.reli.set_last_unit(&unit.id());
        let ret = self.unit_make_transient_body(unit, properties);
        self.reli.clear_last_unit();
        if ret.is_err() {
            unit.remove_transient();
        }
        ret
    }

    fn unit_make_transient_body(
        &self,
        unit: &Rc<UnitX>,
        properties: &[UnitProperty],
    ) -> Result<()> {
        let name = unit.id();
        let path = get_transient_file_path(&name);

        unit.make_transient(Some(path)); // record first
        create_transient_file(&name)?; // create file
        self.unit_set_properties(unit, properties, UnitWriteFlags::RUNTIME)?; // write file

        Ok(())
    }

    fn unit_set_properties(
        &self,
        unit: &Rc<UnitX>,
        properties: &[UnitProperty],
        flags: UnitWriteFlags,
    ) -> Result<()> {
        for property in properties {
            self.unit_set_property(unit, &property.key, &property.value, flags)?;
        }

        Ok(())
    }

    fn unit_set_property(
        &self,
        unit: &Rc<UnitX>,
        key: &str,
        value: &str,
        flags: UnitWriteFlags,
    ) -> Result<()> {
        let mut ret = unit.set_sub_property(key, value, flags);
        if let Err(Error::NotFound { what: _ }) = ret {
            if unit.transient() && unit.load_state() == UnitLoadState::Stub {
                ret = self.unit_set_transient_property(unit, key, value, flags);
            }
        }

        if let Err(Error::NotFound { what: _ }) = ret {
            ret = self.unit_set_live_property(unit, key, value, flags);
        }

        ret
    }

    fn unit_set_transient_property(
        &self,
        unit: &Rc<UnitX>,
        key: &str,
        value: &str,
        flags: UnitWriteFlags,
    ) -> Result<()> {
        let ps = self.sms.private_section(unit.unit_type());
        match key {
            "RefuseManualStart"
            | "RefuseManualStop"
            | "DefaultDependencies"
            | "OnSuccessJobMode"
            | "OnFailureJobMode"
            | "IgnoreOnIsolate"
            | "JobTimeoutSec"
            | "JobTimeoutAction"
            | "StartLimitIntervalSec"
            | "StartLimitBurst"
            | "StartLimitAction"
            | "FailureAction"
            | "SuccessAction"
            | "ConditionACPower"
            | "ConditionCapability"
            | "ConditionDirectoryNotEmpty"
            | "ConditionFileIsExecutable"
            | "ConditionFileNotEmpty"
            | "ConditionFirstBoot"
            | "ConditionKernelCommandLine"
            | "ConditionNeedsUpdate"
            | "ConditionPathExists"
            | "ConditionPathExistsGlob"
            | "ConditionPathIsDirectory"
            | "ConditionPathIsMountPoint"
            | "ConditionPathIsReadWrite"
            | "ConditionPathIsSymbolicLink"
            | "ConditionSecurity"
            | "ConditionUser"
            | "AssertPathExists"
            | "Documentation" => self.unit_write_property(unit, &ps, key, value, flags, false),
            "Wants" | "Requires" | "BindsTo" | "Requisite" | "PartOf" | "OnFailure"
            | "OnSuccess" | "Before" | "After" | "Conflicts" => {
                self.unit_write_property(unit, &ps, key, value, flags, false)
            }
            str_key => Err(Error::NotFound {
                what: format!("set transient property:{}", str_key),
            }),
        }
    }

    fn unit_set_live_property(
        &self,
        unit: &Rc<UnitX>,
        key: &str,
        value: &str,
        flags: UnitWriteFlags,
    ) -> Result<()> {
        let ps = self.sms.private_section(unit.unit_type());
        match key {
            "Description" => self.unit_write_property(unit, &ps, key, value, flags, true),
            str_key => Err(Error::NotFound {
                what: format!("set live property:{}", str_key),
            }),
        }
    }

    fn unit_write_property(
        &self,
        unit: &Rc<UnitX>,
        ps: &str,
        key: &str,
        value: &str,
        flags: UnitWriteFlags,
        update: bool,
    ) -> Result<()> {
        if unit::unit_write_flags_is_noop(flags) {
            return Ok(());
        }

        if update {
            unit.set_property(key, value)?;
        }
        unit.write_settingf(ps, flags, key, format_args!("{}={}", key, value))
    }
}

fn create_transient_file(name: &str) -> Result<()> {
    // create '/run/sysmaster/transient' with mode 750
    let old_mask = stat::umask(Mode::from_bits_truncate(!0o750));
    let ret = create_transient_dir_body();
    let _ = stat::umask(old_mask);
    if let Err(e) = ret {
        log::error!("create transient directory failed: {}", e);
        return Err(e);
    }

    // create '/run/sysmaster/transient/unit.service' with mode 640
    let old_mask = stat::umask(Mode::from_bits_truncate(!0o640));
    let ret = create_transient_file_body(name);
    let _ = stat::umask(old_mask);
    if let Err(e) = ret {
        log::error!("create transient file failed:dir{:?}, {}", name, e);
        return Err(e);
    }

    Ok(())
}

fn create_transient_dir_body() -> Result<()> {
    let dir = Path::new(RUN_TRANSIENT_PATH);
    if !dir.exists() {
        fs::create_dir_all(dir).context(IoSnafu)?;
        log::info!(
            "create transient directory successfully: {}.",
            RUN_TRANSIENT_PATH
        );
    }

    Ok(())
}

fn create_transient_file_body(name: &str) -> Result<()> {
    let path = get_transient_file_path(name);
    fs::write(path.clone(), "# This is a transient unit file, created programmatically via the sysmaster API. Do not edit.\n")?;
    log::info!("create transient file successfully: {:?}.", path);
    Ok(())
}

fn get_transient_file_path(name: &str) -> PathBuf {
    Path::new(RUN_TRANSIENT_PATH).join(name)
}
