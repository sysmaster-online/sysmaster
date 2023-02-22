use super::manager::UnitManager;
use crate::plugin::Plugin;
use crate::unit::data::{DataManager, UnitDepConf};
use crate::unit::datastore::UnitDb;
use crate::unit::entry::UnitX;
use crate::unit::rentry::{self, UnitRe};
use crate::unit::runtime::UnitRT;
use crate::unit::util::UnitFile;
use crate::utils::table::{TableOp, TableSubscribe};
use libutils::path_lookup::LookupPaths;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use sysmaster::unit::UnitType;

//#[derive(Debug)]
pub(super) struct UnitLoad {
    sub_name: String, // key for table-subscriber: UnitDepConf
    data: Rc<UnitLoadData>,
}

impl UnitLoad {
    pub(super) fn new(
        dmr: &Rc<DataManager>,
        rentryr: &Rc<UnitRe>,
        dbr: &Rc<UnitDb>,
        rtr: &Rc<UnitRT>,
        lookup_path: &Rc<LookupPaths>,
    ) -> UnitLoad {
        let load = UnitLoad {
            sub_name: String::from("UnitLoad"),
            data: Rc::new(UnitLoadData::new(dmr, rentryr, dbr, rtr, lookup_path)),
        };
        load.register(dmr);
        load
    }

    pub(super) fn load_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        self.data.load_unit(name)
    }

    pub(super) fn set_um(&self, um: &Rc<UnitManager>) {
        self.data.set_um(um);
    }

    pub(super) fn try_new_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        self.data.try_new_unit(name)
    }

    fn register(&self, dm: &DataManager) {
        let subscriber = Rc::clone(&self.data);
        let ret = dm.register_ud_config(&self.sub_name, subscriber);
        assert!(ret.is_none())
    }
}

//#[derive(Debug)]
struct UnitLoadData {
    // associated objects
    dm: Rc<DataManager>,
    rentry: Rc<UnitRe>,
    um: RefCell<Weak<UnitManager>>,
    db: Rc<UnitDb>,
    rt: Rc<UnitRT>,

    // owned objects
    file: Rc<UnitFile>,
}

// the declaration "pub(self)" is for identification only.
impl UnitLoadData {
    pub(self) fn new(
        dmr: &Rc<DataManager>,
        rentryr: &Rc<UnitRe>,
        dbr: &Rc<UnitDb>,
        rtr: &Rc<UnitRT>,
        lookup_path: &Rc<LookupPaths>,
    ) -> UnitLoadData {
        log::debug!("UnitLoadData db count is {}", Rc::strong_count(dbr));
        let file = Rc::new(UnitFile::new(lookup_path));
        UnitLoadData {
            dm: Rc::clone(dmr),
            rentry: Rc::clone(rentryr),
            um: RefCell::new(Weak::new()),
            db: Rc::clone(dbr),
            rt: Rc::clone(rtr),
            file: Rc::clone(&file),
        }
    }

    pub(self) fn prepare_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        match self.try_new_unit(name) {
            Some(unit) => {
                self.db.units_insert(name.to_string(), Rc::clone(&unit));
                self.rt.push_load_queue(Rc::clone(&unit));
                Some(Rc::clone(&unit))
            }
            None => {
                log::error!(
                    "create unit obj failed,name is {},{}",
                    name,
                    Rc::strong_count(&self.db)
                );
                None
            }
        }
    }

    pub(self) fn push_dep_unit_into_load_queue(&self, name: &str) -> Option<Rc<UnitX>> {
        if let Some(unit) = self.db.units_get(name) {
            return Some(Rc::clone(&unit));
        };

        self.prepare_unit(name)
    }

    pub(self) fn load_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        if let Some(unit) = self.db.units_get(name) {
            return Some(Rc::clone(&unit));
        };
        let unit = self.prepare_unit(name)?;
        log::info!("begin dispatch unit in  load queue");
        self.rt.dispatch_load_queue();
        Some(Rc::clone(&unit))
    }

    pub(self) fn set_um(&self, um: &Rc<UnitManager>) {
        self.um.replace(Rc::downgrade(um));
    }

    pub(self) fn try_new_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        let unit_type = rentry::unit_name_to_type(name);
        if unit_type == UnitType::UnitTypeInvalid {
            return None;
        }

        log::info!(
            "begin create obj for type {:?}, name {} by plugin",
            unit_type,
            name
        );
        let um = self.um();
        let um_rc = Rc::clone(&um);
        let subclass = match Plugin::get_instance().create_unit_obj_with_um(unit_type, um_rc) {
            Ok(sub) => sub,
            Err(_e) => {
                log::error!("Failed to create unit_obj!{}", _e);
                return None;
            }
        };

        let reli = um.reliability();
        subclass.attach_um(um);
        subclass.attach_reli(reli);

        Some(Rc::new(UnitX::new(
            &self.dm,
            &self.rentry,
            &self.file,
            unit_type,
            name,
            subclass,
        )))
    }

    fn um(&self) -> Rc<UnitManager> {
        self.um.clone().into_inner().upgrade().unwrap()
    }
}

impl TableSubscribe<String, UnitDepConf> for UnitLoadData {
    fn notify(&self, op: &TableOp<String, UnitDepConf>) {
        match op {
            TableOp::TableInsert(name, config) => self.insert_udconf(name, config),
            TableOp::TableRemove(_, _) => {} // self.remove_udconf(name)
        }
    }
}

impl UnitLoadData {
    fn insert_udconf(&self, name: &str, config: &UnitDepConf) {
        //hash map insert return is old value,need reconstruct
        let unit = match self.db.units_get(name) {
            Some(u) => u,
            None => {
                log::error!("create unit obj error in unit manager");
                return;
            } // load
        };

        // dependency
        for (relation, list) in config.deps.iter() {
            for o_name in list {
                let tmp_unit: Rc<UnitX>;
                if let Some(o_unit) = self.push_dep_unit_into_load_queue(o_name) {
                    //can not call unit_load directly, will be nested.
                    tmp_unit = Rc::clone(&o_unit);
                } else {
                    log::error!("create unit obj error in unit manager");
                    return;
                }

                if let Err(_e) = self
                    .db
                    .dep_insert(Rc::clone(&unit), *relation, tmp_unit, true, 0)
                //insert the dependency, but not judge loaded success, if loaded failed, whether record the dependency.
                {
                    log::debug!("add dependency relation failed for source unit is {},dependency unit is {}",unit.id(),o_name);
                    return;
                }
            }
        }
    }

    #[allow(dead_code)]
    fn remove_udconf(&self, _source: &str) {
        todo!();
    }
}
