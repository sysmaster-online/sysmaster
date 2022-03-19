pub use unit_base::{UnitActiveState, KillOperation};
pub use unit_entry::{Unit, UnitObj, UnitX};
pub use unit_manager::{UnitManager};

mod unit_base;
mod unit_entry;
mod unit_sets;
mod unit_dep;
mod unit_configs;
mod unit_manager;

//后续考虑和plugin结合
use std::error::Error;
use std::rc::Rc;
use crate::manager::data::{UnitType, DataManager};

fn unit_new(
    dm: Rc<DataManager>,
    unit_type: UnitType,
    name: &str,
) -> Result<Rc<UnitX>, Box<dyn Error>> {
    UnitX::new(dm, unit_type, name)
}
