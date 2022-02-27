use std::error::Error;

pub use unit_entry::*;
pub (in crate::manager) use unit_manager::*;

use crate::plugin::Plugin;
// use services::service::ServiceUnit;

pub mod unit_entry;
pub mod unit_manager;


//后续考虑和plugin结合
fn unit_new(unit_type: UnitType, name: &str) -> Result<Box<dyn UnitObj>, Box<dyn Error>> {
    let unit = Unit::new(name);

    let plugins = Plugin::get_instance();

    plugins.clone().create_unit_obj(unit_type, unit)
}