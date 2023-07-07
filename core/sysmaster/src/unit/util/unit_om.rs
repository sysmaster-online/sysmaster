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
    use mount::{self};
    use service::{self};
    use socket::{self};
    use std::rc::Rc;
    use target::{self};

    pub(super) fn create_um_obj(
        unit_type: UnitType,
        target: &str,
        file_size: u32,
        file_number: u32,
    ) -> Result<Box<dyn UnitManagerObj>> {
        let fun = match unit_type {
            UnitType::UnitMount => mount::__um_obj_create,
            UnitType::UnitService => service::__um_obj_create,
            UnitType::UnitSocket => socket::__um_obj_create,
            UnitType::UnitTarget => target::__um_obj_create,
            _ => {
                return Err(Error::Other {
                    msg: "Component doesn't exist".to_string(),
                })
            }
        };
        let boxed_raw = fun(log::max_level(), target, file_size, file_number);
        Ok(unsafe { Box::from_raw(boxed_raw) })
    }

    pub(super) fn create_subunit_with_um(
        unit_type: UnitType,
        um: Rc<dyn UmIf>,
    ) -> Result<Box<dyn SubUnit>> {
        let fun = match unit_type {
            UnitType::UnitMount => mount::__subunit_create_with_params,
            UnitType::UnitService => service::__subunit_create_with_params,
            UnitType::UnitSocket => socket::__subunit_create_with_params,
            UnitType::UnitTarget => target::__subunit_create_with_params,
            _ => {
                return Err(Error::Other {
                    msg: "Component doesn't exist".to_string(),
                })
            }
        };
        let boxed_raw = fun(um.clone());
        Ok(unsafe { Box::from_raw(boxed_raw) })
    }
}
