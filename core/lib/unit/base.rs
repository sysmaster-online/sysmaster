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

use super::super::rel::ReStation;
use super::kill::{KillContext, KillOperation};
use super::state::{UnitActiveState, UnitNotifyFlags};
use super::umif::UnitMngUtil;
use super::UnitType;
use crate::error::*;
use bitflags::bitflags;
use nix::sys::wait::WaitStatus;
use nix::{sys::socket::UnixCredentials, unistd::Pid};
use std::any::Any;
use std::num::ParseIntError;
use std::str::FromStr;
use std::{collections::HashMap, path::PathBuf, rc::Rc};

///The trait Defining Shared Behavior from Base Unit  to SUB unit
///
/// only one impl,sub unit ref by impl UnitBase
///
pub trait UnitBase {
    ///
    fn id(&self) -> &String;
    ///
    fn test_start_limit(&self) -> bool;
    ///
    fn reset_start_limit(&self);
    ///
    fn kill_context(
        &self,
        k_context: Rc<KillContext>,
        m_pid: Option<Pid>,
        c_pid: Option<Pid>,
        ko: KillOperation,
        main_pid_alien: bool,
    ) -> Result<bool>;

    ///
    fn notify(
        &self,
        original_state: UnitActiveState,
        new_state: UnitActiveState,
        flags: UnitNotifyFlags,
    );

    ///
    fn prepare_exec(&self) -> Result<()>;

    ///
    fn default_dependencies(&self) -> bool;

    ///
    fn cg_path(&self) -> PathBuf;

    ///
    fn ignore_on_isolate(&self) -> bool;

    ///
    fn set_ignore_on_isolate(&self, ignore_on_isolate: bool);

    /// guess main pid from the cgroup path
    fn guess_main_pid(&self) -> Result<Pid>;
}

///The trait Defining Shared Behavior of sub unit
///
/// difference sub unit ref by dynamic trait
///
pub trait SubUnit: ReStation + UnitMngUtil {
    ///
    fn as_any(&self) -> &dyn Any;
    ///
    fn init(&self) {}

    ///
    fn done(&self) {}

    ///
    fn load(&self, conf: Vec<PathBuf>) -> Result<()>;

    ///
    fn dump(&self) {}

    /// Start a Unit
    /// Each Sub Unit need to implement its own start function
    ///
    fn start(&self) -> Result<()> {
        Ok(())
    }

    ///
    // process reentrant with force
    fn stop(&self, _force: bool) -> Result<()> {
        Ok(())
    }

    /// return UnitActionEOpNotSupp for default, if the sub unit not realizing the method
    fn reload(&self) -> Result<()> {
        Err(Error::UnitActionEOpNotSupp)
    }

    ///
    fn can_reload(&self) -> bool {
        false
    }

    ///
    fn kill(&self) {}

    ///
    fn release_resources(&self) {}

    ///
    fn sigchld_events(&self, _wait_status: WaitStatus) {}

    ///
    fn reset_failed(&self) {}

    ///
    fn collect_fds(&self) -> Vec<i32> {
        Vec::new()
    }

    ///
    fn set_socket_fd(&self, _fd: i32) {}

    ///
    fn release_socket_fd(&self, _fd: i32) {}

    ///
    fn trigger(&self, _other: &str) {}

    ///Get the the unit state
    ///
    /// Every sub unit  can define self states and map to [`UnitActiveState`]
    ///
    fn current_active_state(&self) -> UnitActiveState;

    /// Return the state of subunit, i.e. (mounted for .mount, running for .service, listening for .socket)
    fn get_subunit_state(&self) -> String;

    ///
    fn attach_unit(&self, unit: Rc<dyn UnitBase>);

    ///
    fn notify_message(
        &self,
        _ucred: &UnixCredentials,
        _events: &HashMap<&str, &str>,
        _fds: Vec<i32>,
    ) -> Result<()> {
        Ok(())
    }

    ///
    fn get_perpetual(&self) -> bool {
        false
    }
}

/// #[macro_use]
/// the macro for create a sub unit instance with dyn ref of UmIf,
/// which sub unit wants invoke um interface, about UmIf see doc of UmIf
#[macro_export]
macro_rules! declure_unitobj_plugin_with_param {
    ($unit_type:ty, $constructor:path) => {
        use log::LevelFilter;
        /// method for create the unit instance
        #[no_mangle]
        pub fn __subunit_create_with_params(
            um: Rc<dyn $crate::unit::UmIf>,
        ) -> *mut dyn $crate::unit::SubUnit {
            let construcotr: fn(um: Rc<dyn $crate::unit::UmIf>) -> $unit_type = $constructor;
            let obj = construcotr(um);
            let boxed: Box<dyn $crate::unit::SubUnit> = Box::new(obj);
            Box::into_raw(boxed)
        }
    };
}

bitflags! {
    /// used to when check the given unit name is valid
    pub struct UnitNameFlags: u8 {
        /// Allow foo.service
        const PLAIN = 1 << 0;
        /// Allow foo@.service
        const TEMPLATE = 1 << 1;
        /// Allow foo@123.service
        const INSTANCE = 1 << 2;
        /// Any of the above
        const ANY = Self::PLAIN.bits() | Self::TEMPLATE.bits() | Self::INSTANCE.bits();
    }
}

/// The maximum length of a valid unit name
const UNIT_NAME_MAX: usize = 255;

/// check if the given unit name is valid
pub fn unit_name_is_valid(name: &str, flag: UnitNameFlags) -> bool {
    if name.is_empty() {
        return false;
    }
    if name.len() > UNIT_NAME_MAX {
        return false;
    }
    /* Take foo@123.service for example, "foo@123" is its first_name,
     * "foo" is prefix, "service" is its last_name, suffix, or type. */
    let (first_name, last_name) = match name.split_once('.') {
        None => return false,
        Some(v) => (v.0, v.1),
    };

    let unit_type = match unit_type_from_string(last_name) {
        Err(_) => return false,
        Ok(v) => v,
    };

    if unit_type == UnitType::UnitTypeInvalid {
        return false;
    }

    match first_name.split_once('@') {
        None => flag.contains(UnitNameFlags::PLAIN),
        Some(v) => {
            /* "@" is the first character */
            if v.0.is_empty() {
                return false;
            }
            /* "@" is the last character */
            if v.1.is_empty() {
                return flag.contains(UnitNameFlags::TEMPLATE);
            }
            /* there is more than one "@" */
            if v.1.contains('@') {
                return false;
            }
            flag.contains(UnitNameFlags::INSTANCE)
        }
    }
}

/// convert the type string of one unit to UnitType
pub fn unit_type_from_string(type_string: &str) -> Result<UnitType, ParseIntError> {
    UnitType::from_str(type_string)
}
