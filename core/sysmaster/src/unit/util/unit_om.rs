use core::error::*;
use core::unit::UmIf;
use core::unit::{SubUnit, UnitManagerObj, UnitType};
use std::rc::Rc;

/// Create a  obj for subclasses of unit manager
pub(crate) fn create_um_obj(
    unit_type: UnitType,
    target: &str,
    file_size: u32,
    file_number: u32,
) -> Result<Box<dyn UnitManagerObj>> {
    #[cfg(feature = "noplugin")]
    return noplugin::create_um_obj(unit_type, target, file_size, file_number);
    #[cfg(feature = "plugin")]
    return plugin::create_um_obj(unit_type, target, file_size, file_number);
}

/// Create the subunit trait of unit
pub(crate) fn create_subunit_with_um(
    unit_type: UnitType,
    um: Rc<dyn UmIf>,
) -> Result<Box<dyn SubUnit>> {
    #[cfg(feature = "noplugin")]
    return noplugin::create_subunit_with_um(unit_type, um);
    #[cfg(feature = "plugin")]
    return plugin::create_subunit_with_um(unit_type, um);
}

#[cfg(feature = "plugin")]
mod plugin {
    use crate::plugin::Plugin;
    use core::error::*;
    use core::unit::UmIf;
    use core::unit::{SubUnit, UnitManagerObj, UnitType};
    use std::rc::Rc;

    pub(super) fn create_um_obj(
        unit_type: UnitType,
        target: &str,
        file_size: u32,
        file_number: u32,
    ) -> Result<Box<dyn UnitManagerObj>> {
        Plugin::get_instance().create_um_obj(unit_type, target, file_size, file_number)
    }

    pub(super) fn create_subunit_with_um(
        unit_type: UnitType,
        um: Rc<dyn UmIf>,
    ) -> Result<Box<dyn SubUnit>> {
        Plugin::get_instance().create_subunit_with_um(unit_type, um)
    }
}

#[cfg(feature = "noplugin")]
mod noplugin {
    use core::error::*;
    use core::unit::UmIf;
    use core::unit::{SubUnit, UnitManagerObj, UnitType};
    #[cfg(feature = "mount")]
    use mount::{self};
    #[cfg(feature = "path")]
    use path::{self};
    #[cfg(feature = "service")]
    use service::{self};
    #[cfg(feature = "socket")]
    use socket::{self};
    use std::rc::Rc;
    #[cfg(feature = "target")]
    use target::{self};
    #[cfg(feature = "timer")]
    use timer::{self};

    pub(super) fn create_um_obj(
        unit_type: UnitType,
        target: &str,
        file_size: u32,
        file_number: u32,
    ) -> Result<Box<dyn UnitManagerObj>> {
        let fun = match unit_type {
            #[cfg(feature = "mount")]
            UnitType::UnitMount => mount::__um_obj_create,
            #[cfg(feature = "service")]
            UnitType::UnitService => service::__um_obj_create,
            #[cfg(feature = "socket")]
            UnitType::UnitSocket => socket::__um_obj_create,
            #[cfg(feature = "target")]
            UnitType::UnitTarget => target::__um_obj_create,
            #[cfg(feature = "timer")]
            UnitType::UnitTimer => timer::__um_obj_create,
            #[cfg(feature = "path")]
            UnitType::UnitPath => path::__um_obj_create,
            _ => {
                return Err(Error::Other {
                    msg: "Component unsupported!".to_string(),
                });
            }
        };

        /* The parameter for log level is only valid in plugin mode. Thus we could set arbitrary level here. */
        let boxed_raw = fun(log::Level::max(), target, file_size, file_number);

        Ok(unsafe { Box::from_raw(boxed_raw) })
    }

    pub(super) fn create_subunit_with_um(
        unit_type: UnitType,
        um: Rc<dyn UmIf>,
    ) -> Result<Box<dyn SubUnit>> {
        let fun = match unit_type {
            #[cfg(feature = "mount")]
            UnitType::UnitMount => mount::__subunit_create_with_params,
            #[cfg(feature = "service")]
            UnitType::UnitService => service::__subunit_create_with_params,
            #[cfg(feature = "socket")]
            UnitType::UnitSocket => socket::__subunit_create_with_params,
            #[cfg(feature = "target")]
            UnitType::UnitTarget => target::__subunit_create_with_params,
            #[cfg(feature = "timer")]
            UnitType::UnitTimer => timer::__subunit_create_with_params,
            #[cfg(feature = "path")]
            UnitType::UnitPath => path::__subunit_create_with_params,
            _ => {
                return Err(Error::Other {
                    msg: "Component unsupported!".to_string(),
                })
            }
        };
        let boxed_raw = fun(um.clone());
        Ok(unsafe { Box::from_raw(boxed_raw) })
    }
}
