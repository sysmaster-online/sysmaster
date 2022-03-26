use std::error::Error;

use crate::manager::data::*;
pub use unit_base::*;
pub use unit_entry::*;
pub (in crate::manager) use unit_manager::*;

use std::rc::Rc;
// use services::service::ServiceUnit;

mod unit_base;
mod unit_entry;
mod unit_sets;
mod unit_configs;
pub mod unit_manager;


//后续考虑和plugin结合
fn unit_new(dm:Rc<DataManager>, unit_type: UnitType, name: &str) -> Result<Rc<UnitX>, Box<dyn Error>> {
    UnitX::new(dm, unit_type, name)
}