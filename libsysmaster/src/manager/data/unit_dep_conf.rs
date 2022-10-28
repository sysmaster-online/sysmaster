
use std::collections::HashMap;
use crate::manager::UnitRelations;


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
