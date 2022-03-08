pub use unit_entry::*;
pub (in crate::manager) use unit_manager::*;
use crate::manager::service::ServiceUnit;

mod unit_entry;
mod unit_manager;


//后续考虑和plugin结合
fn unit_new(unit_type: UnitType, name: &str) -> Box<dyn UnitObj> {
    let unit = Unit::new(name);

    match unit_type {
        UnitType::UnitService => {
            return Box::new(ServiceUnit::new(unit))
        },
        UnitType::UnitTarget => {
            return Box::new(ServiceUnit::new(unit))
        },
        _ => {
            return Box::new(ServiceUnit::new(unit))
        },
    }
}