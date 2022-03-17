#![warn(unused_imports)]
use super::unit_dep::UnitDep;
use super::unit_sets::UnitSets;
use crate::manager::data::{DataManager, UnitConfig};
use crate::manager::table::{TableOp, TableSubscribe};
use std::rc::Rc;

#[derive(Debug)]
pub(super) struct UnitConfigs {
    name: String,             // key for table-subscriber
    data: Rc<UnitConfigsSub>, // data for table-subscriber
}

impl UnitConfigs {
    pub(super) fn new(dm: Rc<DataManager>, units: Rc<UnitSets>, dep: Rc<UnitDep>) -> UnitConfigs {
        let uc = UnitConfigs {
            name: String::from("UnitConfigs"),
            data: Rc::new(UnitConfigsSub::new(units, dep)),
        };
        uc.register(&dm);
        uc
    }

    fn register(&self, dm: &DataManager) {
        let subscriber = Rc::clone(&self.data);
        dm.register_unit_config(self.name.clone(), subscriber)
            .expect("unit dependency has been registered.");
    }
}

#[derive(Debug)]
struct UnitConfigsSub {
    units: Rc<UnitSets>,
    dep: Rc<UnitDep>,
}

impl TableSubscribe<String, UnitConfig> for UnitConfigsSub {
    fn filter(&self, _op: &TableOp<String, UnitConfig>) -> bool {
        // everything is allowed
        true
    }

    fn notify(&self, op: &TableOp<String, UnitConfig>) {
        match op {
            TableOp::TableInsert(name, config) => self.insert_config(name, config),
            TableOp::TableRemove(_, _) => {} // self.remove_config(name)
        }
    }
}

// the declaration "pub(self)" is for identification only.
impl UnitConfigsSub {
    pub(self) fn new(units: Rc<UnitSets>, dep: Rc<UnitDep>) -> UnitConfigsSub {
        UnitConfigsSub { units, dep }
    }

    pub(self) fn insert_config(&self, source: &str, config: &UnitConfig) {
        let unitx = match self.units.get(source) {
            Some(u) => u,
            None => todo!(), // load
        };

        // config
        unitx.set_config(config);

        // dependency
        for (relation, name) in config.deps.iter() {
            self.dep.insert(
                Rc::clone(&unitx),
                *relation,
                self.units.get(name).unwrap(),
                0,
            );
        }
    }

    pub(self) fn remove_config(&self, _source: &str) {
        todo!();
    }
}
