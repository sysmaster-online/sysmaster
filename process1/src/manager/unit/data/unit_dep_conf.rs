use crate::manager::unit::unit_rentry::UnitRelations;
use std::collections::HashMap;

#[derive(Default)]
pub struct UnitDepConf {
    pub deps: HashMap<UnitRelations, Vec<String>>,
}

impl UnitDepConf {
    pub fn new() -> UnitDepConf {
        UnitDepConf {
            deps: HashMap::new(),
        }
    }
}
