use super::uu_base::UeBase;
use crate::manager::unit::data::{DataManager, UnitDepConf};
use crate::manager::unit::uload_util::UnitFile;
use crate::manager::unit::unit_rentry::{UnitLoadState, UnitRePps, UnitRelations};
use crate::reliability::ReStation;
use std::cell::RefCell;
use std::error::Error as stdError;
use std::path::PathBuf;
use std::rc::Rc;

use super::uu_config::UeConfig;
//#[derive(Debug)]
pub(super) struct UeLoad {
    // associated objects
    dm: Rc<DataManager>,
    file: Rc<UnitFile>,
    base: Rc<UeBase>,
    config: Rc<UeConfig>,

    // owned objects
    load_state: RefCell<UnitLoadState>,
    in_load_queue: RefCell<bool>,
    in_target_dep_queue: RefCell<bool>,
}

impl ReStation for UeLoad {
    // no input, no compensate

    // data
    fn db_map(&self) {
        if let Some(load_state) = self.base.rentry_load_get() {
            *self.load_state.borrow_mut() = load_state;
        }
    }

    fn db_insert(&self) {
        self.base.rentry_load_insert(*self.load_state.borrow());
    }

    // reload: no external connections, no entry
}

impl UeLoad {
    pub(super) fn new(
        dmr: &Rc<DataManager>,
        filer: &Rc<UnitFile>,
        baser: &Rc<UeBase>,
        config: &Rc<UeConfig>,
    ) -> UeLoad {
        let load = UeLoad {
            dm: Rc::clone(dmr),
            file: Rc::clone(filer),
            base: Rc::clone(baser),
            config: Rc::clone(config),
            load_state: RefCell::new(UnitLoadState::UnitStub),
            in_load_queue: RefCell::new(false),
            in_target_dep_queue: RefCell::new(false),
        };
        load.db_insert();
        let flags = UnitRePps::QUEUE_LOAD | UnitRePps::QUEUE_TARGET_DEPS;
        load.base.rentry_pps_clear(flags);
        load
    }

    pub(super) fn get_unit_id_fragment_pathbuf(&self) -> Vec<PathBuf> {
        self.file.get_unit_id_fragment_pathbuf(self.base.id())
    }

    pub(super) fn set_load_state(&self, load_state: UnitLoadState) {
        *self.load_state.borrow_mut() = load_state;
        self.db_update();
    }

    pub(super) fn load_state(&self) -> UnitLoadState {
        let state = self.load_state.clone();
        state.into_inner()
    }

    pub(super) fn set_in_load_queue(&self, t: bool) {
        *self.in_load_queue.borrow_mut() = t;
        if t {
            self.base.rentry_pps_set(UnitRePps::QUEUE_LOAD);
        } else {
            self.base.rentry_pps_clear(UnitRePps::QUEUE_LOAD);
        }
    }

    pub(super) fn in_load_queue(&self) -> bool {
        *self.in_load_queue.borrow()
    }

    pub(super) fn load_unit_confs(&self) -> Result<(), Box<dyn stdError>> {
        self.file.build_name_map(
            self.base.id().clone(),
            self.load_state() == UnitLoadState::UnitLoaded,
        );
        self.config
            .load_fragment_and_dropin(self.file.as_ref(), self.base.id())?;
        self.parse();
        Ok(())
    }

    pub(super) fn set_in_target_dep_queue(&self, t: bool) {
        self.in_target_dep_queue.replace(t);
        if t {
            self.base.rentry_pps_set(UnitRePps::QUEUE_TARGET_DEPS);
        } else {
            self.base.rentry_pps_clear(UnitRePps::QUEUE_TARGET_DEPS);
        }
    }

    pub(super) fn in_target_dep_queue(&self) -> bool {
        *self.in_target_dep_queue.borrow()
    }

    fn parse(&self) {
        let mut ud_conf = UnitDepConf::new(); // need get config from config database,and update depends hereW

        ud_conf.deps.insert(
            UnitRelations::UnitWants,
            self.config.config_data().borrow().Unit.Wants.clone(),
        );
        ud_conf.deps.insert(
            UnitRelations::UnitAfter,
            self.config.config_data().borrow().Unit.After.clone(),
        );
        ud_conf.deps.insert(
            UnitRelations::UnitBefore,
            self.config.config_data().borrow().Unit.Before.clone(),
        );
        ud_conf.deps.insert(
            UnitRelations::UnitRequires,
            self.config.config_data().borrow().Unit.Requires.clone(),
        );

        ud_conf.deps.insert(
            UnitRelations::UnitWantsBy,
            self.config.config_data().borrow().Install.WantedBy.clone(),
        );
        ud_conf.deps.insert(
            UnitRelations::UnitRequiresBy,
            self.config
                .config_data()
                .borrow()
                .Install
                .RequiredBy
                .clone(),
        );

        ud_conf.deps.insert(
            UnitRelations::UnitConflicts,
            self.config.config_data().borrow().Unit.Conflicts.clone(),
        );

        self.dm.insert_ud_config(self.base.id().clone(), ud_conf);
    }
}
