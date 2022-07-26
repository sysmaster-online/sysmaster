use confique::Error;

use crate::manager::data::{DataManager, UnitDepConf, UnitRelations};
use crate::manager::unit::uload_util::UnitFile;
use crate::manager::unit::unit_base::UnitLoadState;
use crate::null_str;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use super::uu_config::UeConfig;
//#[derive(Debug)]
pub(super) struct UeLoad {
    // associated objects
    dm: Rc<DataManager>,
    file: Rc<UnitFile>,
    config: Rc<UeConfig>,

    // owned objects
    /* key */
    id: String,
    /* data */
    load_state: RefCell<UnitLoadState>,
    config_file_path: RefCell<String>,
    config_file_mtime: RefCell<u128>,
    in_load_queue: RefCell<bool>,
    default_dependencies: bool,
}

impl UeLoad {
    pub(super) fn new(
        dmr: &Rc<DataManager>,
        filer: &Rc<UnitFile>,
        config: &Rc<UeConfig>,
        id: String,
    ) -> UeLoad {
        UeLoad {
            dm: Rc::clone(dmr),
            config: Rc::clone(config),
            file: Rc::clone(filer),
            id,
            load_state: RefCell::new(UnitLoadState::UnitStub),
            config_file_path: RefCell::new(null_str!("")),
            config_file_mtime: RefCell::new(0),
            in_load_queue: RefCell::new(false),
            default_dependencies: true,
        }
    }

    pub(super) fn get_unit_id_fragment_pathbuf(&self) -> Vec<PathBuf> {
        self.file.get_unit_id_fragment_pathbuf(&self.id)
    }

    pub(super) fn set_load_state(&self, load_state: UnitLoadState) {
        *self.load_state.borrow_mut() = load_state;
    }

    pub(super) fn load_state(&self) -> UnitLoadState {
        let state = self.load_state.clone();
        state.into_inner()
    }

    pub(super) fn set_in_load_queue(&self, t: bool) {
        *self.in_load_queue.borrow_mut() = t;
    }

    pub(super) fn in_load_queue(&self) -> bool {
        *self.in_load_queue.borrow() == true
    }

    pub(super) fn load_unit_confs(&self) -> Result<(), Error> {
        self.file.build_name_map(self.id.clone());
        self.config
            .load_fragment_and_dropin(self.file.as_ref(), &self.id)?;
        Ok(())
    }

    fn parse(&self) {
        let mut ud_conf = UnitDepConf::new(); // need get config from config database,and update depends hereW

        ud_conf
            .deps
            .insert(UnitRelations::UnitWants, self.config.Unit.wants.clone());
        ud_conf
            .deps
            .insert(UnitRelations::UnitAfter, self.config.Unit.after.clone());
        ud_conf
            .deps
            .insert(UnitRelations::UnitBefore, self.config.Unit.before.clone());
        ud_conf.deps.insert(
            UnitRelations::UnitRequires,
            self.config.Unit.requires.clone(),
        );

        ud_conf.deps.insert(
            UnitRelations::UnitWantsBy,
            self.config.Install.wanted_by.clone(),
        );
        ud_conf.deps.insert(
            UnitRelations::UnitRequiresBy,
            self.config.Install.required_by.clone(),
        );
        self.dm.insert_ud_config(self.id.clone(), ud_conf);
    }
}
