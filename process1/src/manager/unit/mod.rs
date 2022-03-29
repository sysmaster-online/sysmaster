pub use unit_base::{KillOperation, UnitActiveState};
pub use unit_entry::{Unit, UnitDb, UnitObj, UnitX};
pub use unit_manager::UnitManager;

mod unit_base;
mod unit_configs;
mod unit_dep;
mod unit_entry;
mod unit_manager;
mod unit_sets;

//后续考虑和plugin结合
use crate::manager::data::{DataManager, UnitType};
use std::error::Error;
use std::rc::Rc;

fn unit_new(
    dm: Rc<DataManager>,
    unit_db: Rc<UnitDb>,
    unit_type: UnitType,
    name: &str,
) -> Result<Rc<UnitX>, Box<dyn Error>> {
    UnitX::new(dm, unit_db, unit_type, name)
}
