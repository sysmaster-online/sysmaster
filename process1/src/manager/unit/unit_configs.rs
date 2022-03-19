#![warn(unused_imports)]
use std::rc::Rc;
use crate::manager::table::{TableSubscribe, TableOp};
use crate::manager::data::{UnitConfig, DataManager};
use super::unit_sets::{UnitSets};

pub(super) struct UnitConfigs {
    name:String, // key for table-subscriber
    data:Rc<UnitConfigsSub>, // data for table-subscriber
}

impl UnitConfigs {
    pub(super) fn new(units:Rc<UnitSets>) -> UnitConfigs {
        let ud = UnitConfigs {
            name:String::from("UnitConfigs"),
            data:Rc::new(UnitConfigsSub::new(units)),
        };
        // ud.register(dm); // managers-manager
        ud
    }

    fn register(&self, dm:&mut DataManager) {
        let subscriber = Rc::clone(&self.data);
        dm.register_unit_config(self.name.clone(), subscriber).expect("job dependency has been registered.");
    }

}

#[derive(Debug)]
struct UnitConfigsSub {
    units:Rc<UnitSets>,
}

impl TableSubscribe<String, UnitConfig> for UnitConfigsSub {
    fn filter(&self, _op:&TableOp<String, UnitConfig>) -> bool {
        // everything is allowed
        true
    }

    fn notify(&self, op:&TableOp<String, UnitConfig>) {
        match op {
            TableOp::TableInsert(name, config) => self.insert_config(name, config),
            TableOp::TableRemove(_, _) => {}, // self.remove_config(name)
        }
    }
}

// the declaration "pub(self)" is for identification only.
impl UnitConfigsSub {
    pub(self) fn new(units:Rc<UnitSets>) -> UnitConfigsSub {
        UnitConfigsSub {
            units,
        }    
    }

    pub(self) fn insert_config(&self, source:&str, config:&UnitConfig) {
        let unitx = match self.units.get_unit_on_name(source) {
            Some(u) => u,
            None => todo!(), // load
        };

        // config
        unitx.as_ref().borrow().set_config(config);

        // dependency
        for (relation, name) in config.deps.iter() {
            unitx.as_ref().borrow().add_dependencies(*relation, name);
        }
    }

    pub(self) fn remove_config(&self, _source:&str) {
        todo!();
    }
}
