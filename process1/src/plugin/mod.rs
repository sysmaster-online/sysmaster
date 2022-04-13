use super::manager::{UnitObj, UnitType};
use dynamic_reload as dy_re;
use log::*;
use std::ffi::OsStr;
use std::io;
use std::{collections::HashMap, error::Error, path::PathBuf, sync::Arc};
use walkdir::{DirEntry, WalkDir};

use std::cell::RefCell;
use std::rc::Rc;

pub struct Plugin {
    unitobj_lists: Vec<Arc<Box<dyn UnitObj>>>,
    library_dir: String,
    load_libs: HashMap<UnitType, Arc<dy_re::Lib>>,
    is_loaded: bool,
}

#[allow(dead_code)]
impl Plugin {
    fn new() -> Self {
        Self {
            unitobj_lists: Vec::new(),
            library_dir: String::new(),
            load_libs: HashMap::new(),
            is_loaded: false,
        }
    }

    pub fn get_instance() -> Rc<RefCell<Plugin>> {
        log::info!("get_instance");
        static mut PLUGIN: Option<Rc<RefCell<Plugin>>> = None;
        unsafe {
            PLUGIN
                .get_or_insert_with(|| {
                    let mut plugin = Plugin::new();
                    plugin.set_library_dir("target/debug");
                    plugin.load_lib();
                    Rc::new(RefCell::new(plugin))
                })
                .clone()
        }
    }

    pub fn load_lib(&mut self) {
        let file_exist = || {
            if self.is_loaded {
                log::info!("plugin is already loaded");
                return false;
            }
            if self.library_dir.is_empty() {
                return false;
            }
            let libdir_path = PathBuf::from(self.library_dir.as_str());
            if !libdir_path.exists() || !libdir_path.is_dir() {
                log::error!("library_dir {:?} is not a dir or not exist", libdir_path);
                return false;
            }
            true
        };
        let a = file_exist();
        if !a {
            return;
        }
        log::debug!(
            "begin loading library in library dir {:?}",
            self.library_dir
        );
        let mut reload_handler = dynamic_reload::DynamicReload::new(
            Some(vec![self.library_dir.as_str()]),
            Some(self.library_dir.as_str()),
            dynamic_reload::Search::Default,
        );
        for entry in WalkDir::new(&self.library_dir)
            .min_depth(1)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| Self::is_dynamic_lib(e))
        {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                continue;
            } else {
                let file_name = path.file_name();
                let result = Self::load_plugin(self, file_name.unwrap(), &mut reload_handler);
                if let Ok(_r) = result {
                    log::info!("Plugin load unit plugin[{:?}] sucessfull", file_name);
                } else if let Err(_e) = result {
                    log::error!(
                        "Plugin load unit plugin[{:?}] failed,deatil is {}",
                        file_name,
                        _e.to_string()
                    );
                }
            }
        }
        self.is_loaded = true;
    }

    pub fn load_plugin(
        &mut self,
        filename: &OsStr,
        reload_handler: &mut dynamic_reload::DynamicReload,
    ) -> io::Result<()> {
        if let Some(v) = filename.to_str() {
            match reload_handler.add_library(v, dynamic_reload::PlatformName::No) {
                Ok(lib) => {
                    let unit_type = self.get_unit_type(v);
                    if unit_type == UnitType::UnitTypeInvalid {
                        log::error!("invalid service type os lib {}", v);
                        return Ok(());
                    }
                    log::debug!(
                        "insert unit {} dynamic lib into libs",
                        unit_type.to_string()
                    );
                    self.load_libs.insert(unit_type, lib.clone());
                    let dy_lib = self.load_libs.get(&unit_type).unwrap();
                    let fun: dynamic_reload::Symbol<fn() -> *mut dyn UnitObj> =
                        unsafe { dy_lib.lib.get(b"__unit_obj_create").unwrap() };
                    let boxed_raw = fun();
                    self.unitobj_lists
                        .push(Arc::new(unsafe { Box::from_raw(boxed_raw) }));
                    log::info!("loading dynamic lib sucessfully");
                }
                Err(e) => error!("error loading Unable to load dynamic lib, err {:?}", e),
            }
        }
        Ok(())
    }

    fn get_unit_type(&self, name: &str) -> UnitType {
        if name.contains("libservice") {
            return UnitType::UnitService;
        }

        UnitType::UnitTypeInvalid
    }

    pub fn set_library_dir(&mut self, library_dir: &str) {
        self.library_dir.clear();
        self.library_dir.push_str(library_dir);
        log::debug!("set libray dir {}", library_dir);
    }

    pub fn is_dynamic_lib(entry: &DirEntry) -> bool {
        let file_type = entry.file_type();
        let file_name = entry.file_name();
        if file_type.is_file()
            && file_name
                .to_str()
                .map(|s| s.ends_with(".so"))
                .unwrap_or(false)
        {
            true
        } else {
            false
        }
    }

    pub fn create_unit_obj(&self, unit_type: UnitType) -> Result<Box<dyn UnitObj>, Box<dyn Error>> {
        let dy_lib = match self.load_libs.get(&unit_type) {
            Some(lib) => lib.clone(),
            None => {
                return Err(format!("the {:?} plugin is not exist", unit_type.to_string()).into())
            }
        };

        let fun: dynamic_reload::Symbol<fn() -> *mut dyn UnitObj> =
            unsafe { dy_lib.lib.get(b"__unit_obj_create").unwrap() };
        let boxed_raw = fun();

        return Ok(unsafe { Box::from_raw(boxed_raw) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use services::service::ServiceUnit;

    #[test]
    #[ignore]
    fn test_plugin_load_library() {
        let plugins = Rc::clone(&Plugin::get_instance());
        let t_p = plugins.borrow();
        for uniobj in &t_p.unitobj_lists {
            let u = Arc::clone(&uniobj);
            let _u_box = unsafe { Arc::into_raw(u).as_ref().unwrap() };
            // let service_unit = u_box.as_any().downcast_ref::<ServiceUnit>().unwrap();
            // assert_eq!(service_unit.get_unit_name(),"");
        }
    }
}
