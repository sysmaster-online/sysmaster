use super::u_error::UnitActionError;
use super::u_kill::{KillContext, KillOperation};
use super::u_state::{UnitActiveState, UnitNotifyFlags};
use super::um_interface::UnitMngUtil;
use super::{super::reliability::ReStation, UnitRelations};
use libutils::error::Error as ServiceError;
use libutils::Result;
use nix::{
    sys::{signal::Signal, socket::UnixCredentials},
    unistd::Pid,
};
use std::{collections::HashMap, error::Error, path::PathBuf, rc::Rc};

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
    fn kill_context(
        &self,
        k_context: Rc<KillContext>,
        m_pid: Option<Pid>,
        c_pid: Option<Pid>,
        ko: KillOperation,
    ) -> Result<(), Box<dyn Error>>;

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
    fn insert_two_deps(
        &self,
        ra: UnitRelations,
        rb: UnitRelations,
        u_name: String,
    ) -> Result<(), Box<dyn Error>>;
    ///
    fn insert_dep(&self, ra: UnitRelations, u_name: String);

    ///
    fn cg_path(&self) -> PathBuf;

    ///
    fn ignore_on_isolate(&self) -> bool;

    ///
    fn set_ignore_on_isolate(&self, ignore_on_isolate: bool);

    /// guess main pid from the cgroup path
    fn guess_main_pid(&self) -> Result<Pid, Box<dyn Error>>;
}

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
        pub fn __unit_obj_create_default() -> *mut dyn $crate::unit::SubUnit {
            logger::init_log_with_default($name, $level);
            let construcotr: fn() -> $unit_type = $constructor;

            let obj = construcotr();
            let boxed: Box<dyn $crate::unit::SubUnit> = Box::new(obj);
            Box::into_raw(boxed)
        }
    };
}

/// #[macro_use]
/// the macro for create a sub unit instance with dyn ref of UmIf,
/// which sub unit wants invoke um interface, about UmIf see doc of UmIf
#[macro_export]
macro_rules! declure_unitobj_plugin_with_param {
    ($unit_type:ty, $constructor:path, $name:expr) => {
        use log::LevelFilter;
        /// method for create the unit instance
        #[no_mangle]
        pub fn __unit_obj_create_with_params(
            um: Rc<dyn $crate::unit::UmIf>,
            level: LevelFilter,
        ) -> *mut dyn $crate::unit::SubUnit {
            logger::init_log_with_default($name, level);
            let construcotr: fn(um: Rc<dyn $crate::unit::UmIf>) -> $unit_type = $constructor;

            let obj = construcotr(um);
            let boxed: Box<dyn $crate::unit::SubUnit> = Box::new(obj);
            Box::into_raw(boxed)
        }
    };
}
