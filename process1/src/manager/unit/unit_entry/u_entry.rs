extern crate siphasher;
use super::uu_child::UeChild;
use super::uu_config::UeConfig;
use super::uu_load::UeLoad;
use super::uu_state::UeState;
use crate::manager::data::{DataManager, UnitActiveState, UnitType};
use crate::manager::unit::unit_base::{self, UnitLoadState};
use crate::manager::unit::unit_file::UnitFile;
use crate::manager::unit::unit_parser_mgr::{UnitConfigParser, UnitParserMgr};
use log;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use std::any::Any;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use utils::unit_conf::{Conf, Section};

pub struct Unit {
    // associated objects
    file: Rc<UnitFile>,
    unit_conf_mgr: Rc<UnitParserMgr<UnitConfigParser>>,
    unit_type: UnitType,
    id: String,
    config: UeConfig,
    state: UeState,
    load: UeLoad,
    child: UeChild,
    sub: Rc<RefCell<Box<dyn UnitObj>>>,
}

impl PartialEq for Unit {
    fn eq(&self, other: &Self) -> bool {
        self.unit_type == other.unit_type && self.id == other.id
    }
}

impl Eq for Unit {}

impl PartialOrd for Unit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Unit {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl Hash for Unit {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub trait UnitObj {
    //: std::fmt::Debug {
    fn init(&self) {}
    fn done(&self) {}
    fn load(&mut self, section: &Section<Conf>) -> Result<(), Box<dyn Error>>;
    fn coldplug(&self) {}
    fn dump(&self) {}
    fn start(&mut self) {}
    fn stop(&mut self) {}
    fn reload(&mut self) {}

    fn kill(&self) {}
    fn check_gc(&self) -> bool;
    fn release_resources(&self) {}
    fn check_snapshot(&self) {}
    fn sigchld_events(&mut self, _pid: Pid, _code: i32, _status: Signal) {}
    fn reset_failed(&self) {}
    fn trigger(&mut self, _other: Rc<RefCell<Box<dyn UnitObj>>>) {}
    fn in_load_queue(&self) -> bool;

    fn eq(&self, other: &dyn UnitObj) -> bool;
    fn hash(&self) -> u64;
    fn as_any(&self) -> &dyn Any;

    fn get_private_conf_section_name(&self) -> Option<&str>;
}

impl Unit {
    pub(super) fn new(
        dm: Rc<DataManager>,
        file: Rc<UnitFile>,
        unit_conf_mgr: Rc<UnitParserMgr<UnitConfigParser>>,
        unit_type: UnitType,
        name: &str,
        sub: Box<dyn UnitObj>,
    ) -> Self {
        Unit {
            unit_type,
            file,
            unit_conf_mgr,
            id: String::from(name),
            config: UeConfig::new(),
            state: UeState::new(Rc::clone(&dm)),
            load: UeLoad::new(Rc::clone(&dm), String::from(name)),
            child: UeChild::new(),
            sub: Rc::new(RefCell::new(sub)),
        }
    }

    pub fn in_load_queue(&self) -> bool {
        self.load.in_load_queue()
    }

    pub fn set_in_load_queue(&self, t: bool) {
        self.load.set_in_load_queue(t);
    }

    fn build_name_map(&self) {
        self.file.build_name_map();
    }

    fn get_unit_file_path(&self) -> Option<String> {
        match self.file.get_unit_file_path(&self.id) {
            Some(v) => return Some(v.to_string()),
            None => {
                log::error!("not find unit file {}", &self.id);
                None
            }
        }
    }

    pub fn load_unit(&self) -> Result<(), Box<dyn Error>> {
        self.set_in_load_queue(false);
        self.build_name_map();
        if let Some(p) = self.get_unit_file_path() {
            self.load.set_config_file_path(&p);
            let unit_type = unit_base::unit_name_to_type(&self.id); //best use of owner not name,need reconstruct
            let confs = self
                .unit_conf_mgr
                .unit_file_parser(&unit_type.to_string(), &p);
            match confs {
                Ok(confs) => {
                    let result = self.load.unit_load(&confs);
                    if let Err(s) = result {
                        self.load.set_load_state(UnitLoadState::UnitError);
                        return Err(s);
                    }
                    let _ret;
                    let private_section_name = self.get_private_conf_section_name();
                    if let Some(p_s_name) = private_section_name {
                        let _section = confs.get_section_by_name(&p_s_name);
                        let _result = _section.map(|s| self.sub.borrow_mut().load(s));
                        if let Some(r) = _result{
                            _ret = r.map(|_s|self.load.set_load_state(UnitLoadState::UnitLoaded));
                        }else{
                            _ret = Err(format!("load Unit {} failed",self.id).into());
                        }
                    }else{
                        _ret = Err(format!("Cann't found private section conf for {}",self.id).into());
                    }
                    self.load.set_load_state(UnitLoadState::UnitLoaded);
                    _ret
                }
                Err(e) => {
                    self.load.set_load_state(UnitLoadState::UnitNotFound);
                    return Err(format!("{}", e.to_string()).into());
                }
            }
        } else {
            self.load.set_load_state(UnitLoadState::UnitNotFound);
            return Err(format!("Unit[ {}] file Not found", self.id).into());
        }
    }

    pub fn notify(&self, original_state: UnitActiveState, new_state: UnitActiveState) {
        if original_state != new_state {
            log::debug!(
                "unit active state change from: {:?} to {:?}",
                original_state,
                new_state
            );
        }
        self.state.update(&self.id, original_state, new_state, 0);
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_unit_type(&self) -> UnitType {
        self.unit_type
    }

    pub fn get_private_conf_section_name(&self) -> Option<String> {
        let sub = Rc::clone(&self.sub);
        let str = sub
            .borrow()
            .get_private_conf_section_name()
            .map(|s| s.to_string());
        str
    }
}
