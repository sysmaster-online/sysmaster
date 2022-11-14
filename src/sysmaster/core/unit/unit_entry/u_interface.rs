use std::{collections::HashMap, error::Error, path::PathBuf, rc::Rc};

use nix::{
    sys::{signal::Signal, socket::UnixCredentials},
    unistd::Pid,
};

use libutils::error::Error as ServiceError;

use crate::core::{
    unit::{UnitActionError, UnitActiveState, UnitMngUtil},
    ReStation,
};

use super::Unit;

///The trait Defining Shared Behavior of sub unit
///
/// difference sub unit ref by dynamic trait
///
pub trait SubUnit: ReStation + UnitMngUtil {
    ///
    fn init(&self) {}

    ///
    fn done(&self) {}

    ///
    fn load(&self, conf: Vec<PathBuf>) -> Result<(), Box<dyn Error>>;

    ///
    fn dump(&self) {}

    /// Start a Unit
    /// Each Sub Unit need to implement its own start function
    ///
    fn start(&self) -> Result<(), UnitActionError> {
        Ok(())
    }

    ///
    // process reentrant with force
    fn stop(&self, _force: bool) -> Result<(), UnitActionError> {
        Ok(())
    }

    ///
    fn reload(&self) {}

    ///
    fn kill(&self) {}

    ///
    fn release_resources(&self) {}

    ///
    fn sigchld_events(&self, _pid: Pid, _code: i32, _status: Signal) {}

    ///
    fn reset_failed(&self) {}

    ///
    fn collect_fds(&self) -> Vec<i32> {
        Vec::new()
    }

    ///Get the the unit state
    ///
    /// Every sub unit  can define self states and map to [`UnitActiveState`]
    ///
    fn current_active_state(&self) -> UnitActiveState;

    ///
    fn attach_unit(&self, unit: Rc<Unit>);

    ///
    fn notify_message(
        &self,
        _ucred: &UnixCredentials,
        _events: &HashMap<&str, &str>,
        _fds: Vec<i32>,
    ) -> Result<(), ServiceError> {
        Ok(())
    }
}

/// #[macro_use]
/// the macro for create a sub unit instance
#[macro_export]
macro_rules! declure_unitobj_plugin_default {
    ($unit_type:ty, $constructor:path, $name:expr, $level:expr) => {
        /// method for create the unit instance
        #[no_mangle]
        pub fn __unit_obj_create_default() -> *mut dyn $crate::manager::SubUnit {
            logger::init_log_with_default($name, $level);
            let construcotr: fn() -> $unit_type = $constructor;

            let obj = construcotr();
            let boxed: Box<dyn $crate::manager::SubUnit> = Box::new(obj);
            Box::into_raw(boxed)
        }
    };
}

/// #[macro_use]
/// the macro for create a sub unit instance with dyn ref of UmIf,
/// which sub unit want's invoke um interface, about UmIf see doc of UmIf
#[macro_export]
macro_rules! declure_unitobj_plugin_with_param {
    ($unit_type:ty, $constructor:path, $name:expr, $level:expr) => {
        /// method for create the unit instance
        #[no_mangle]
        pub fn __unit_obj_create_with_params(
            um: Rc<dyn $crate::manager::UmIf>,
        ) -> *mut dyn $crate::manager::SubUnit {
            logger::init_log_with_default($name, $level);
            let construcotr: fn(um: Rc<dyn $crate::manager::UmIf>) -> $unit_type = $constructor;

            let obj = construcotr(um);
            let boxed: Box<dyn $crate::manager::SubUnit> = Box::new(obj);
            Box::into_raw(boxed)
        }
    };
}
