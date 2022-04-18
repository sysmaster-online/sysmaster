use super::manager::{UnitSubClass, UnitType};
use dynamic_reload as dy_re;
use log::*;
use std::cell::RefCell;
use std::ffi::OsStr;
use std::io;
use std::sync::RwLock;
use std::{collections::HashMap, error::Error, path::PathBuf, sync::Arc};
use walkdir::{DirEntry, WalkDir};

pub struct Plugin {
    /*unitobj_lists: RefCell<Vec<Arc<Box<dyn UnitSubClass>>>>,//hide unit obj mut use refcell*/
    library_dir: RwLock<String>,
    load_libs: RwLock<HashMap<UnitType, Arc<dy_re::Lib>>>,
    _loaded: RefCell<bool>,
}

#[allow(dead_code)]
impl Plugin {
    fn new() -> Self {
        Self {
            //unitobj_lists: RefCell::new(Vec::new()),
            library_dir: RwLock::new(String::new()),
            load_libs: RwLock::new(HashMap::new()),
            _loaded: RefCell::new(false),
        }
    }

    pub fn get_instance() -> Arc<Plugin> {
        log::info!("get_instance");
        static mut PLUGIN: Option<Arc<Plugin>> = None;
        unsafe {
            PLUGIN
                .get_or_insert_with(|| {
                    let plugin = Plugin::new();
                    plugin.set_library_dir("target/debug");
                    plugin.load_lib();
                    Arc::new(plugin)
                })
                .clone()
        }
    }

    pub fn load_lib(&self) {
        let lib_path = self.library_dir.read().unwrap();
        let file_exist = || {
            if *(self._loaded.borrow()) {
                log::info!("plugin is already loaded");
                return false;
            }
            if (*lib_path).is_empty() {
                return false;
            }
            let libdir_path = PathBuf::from((*lib_path).as_str());
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
        log::debug!("begin loading library in library dir {:?}", lib_path);
        let mut reload_handler = dynamic_reload::DynamicReload::new(
            Some(vec![(*lib_path).as_str()]),
            Some((*lib_path).as_str()),
            dynamic_reload::Search::Default,
        );
        for entry in WalkDir::new((*lib_path).as_str())
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
        self._loaded.replace(true);
    }

    pub fn load_plugin(
        &self,
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
                    {
                        let mut wloadlibs = self.load_libs.write().unwrap();
                        (*wloadlibs).insert(unit_type, lib.clone());
                    }
                    /*
                    let dy_lib = (*self.load_libs.read().unwrap()).get(&unit_type).unwrap();
                    let fun: dynamic_reload::Symbol<fn() -> *mut dyn UnitSubClass> =
                        unsafe { dy_lib.lib.get(b"__unit_obj_create").unwrap() };
                    let boxed_raw = fun();
                    self.unitobj_lists.borrow_mut()
                        .push(Arc::new(unsafe { Box::from_raw(boxed_raw) }));
                    */
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

    pub fn set_library_dir(&self, library_dir: &str) {
        let update_dir = || {
            let new_libdir = PathBuf::from(library_dir);
            if !new_libdir.is_dir() || !new_libdir.is_dir() {
                log::error!("library_dir {:?} is not a dir or not exist", library_dir);
                return;
            }
            match self.library_dir.try_read() {
                Ok(pathdir) => {
                    let old_libdir = PathBuf::from((*pathdir).as_str());
                    if old_libdir == new_libdir {
                        log::info!("library dir is already set {}", library_dir);
                        return;
                    }
                }
                Err(e) => {
                    log::error!("set library dir failed {}", e.to_string());
                    return;
                }
            }
            let mut w = self.library_dir.write().unwrap();
            (*w).clear();
            (*w).push_str(library_dir);
            log::debug!("set libray dir {}", library_dir);
            self._loaded.replace(false);
        };
        update_dir();
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

    pub fn create_unit_obj(
        &self,
        unit_type: UnitType,
    ) -> Result<Box<dyn UnitSubClass>, Box<dyn Error>> {
        let dy_lib = match (*self.load_libs.read().unwrap()).get(&unit_type) {
            Some(lib) => lib.clone(),
            None => {
                return Err(format!("the {:?} plugin is not exist", unit_type.to_string()).into())
            }
        };

        let fun: dynamic_reload::Symbol<fn() -> *mut dyn UnitSubClass> =
            unsafe { dy_lib.lib.get(b"__unit_obj_create").unwrap() };
        let boxed_raw = fun();

        Ok(unsafe { Box::from_raw(boxed_raw) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use services::service::ServiceUnit;

    #[test]
    fn test_plugin_load_library() {
        let plugins = Arc::clone(&Plugin::get_instance());
        let t_p = plugins;
        for key in (*t_p.load_libs.read().unwrap()).keys() {
            assert_eq!(key.to_string(), UnitType::UnitService.to_string());
            // let service_unit = u_box.as_any().downcast_ref::<ServiceUnit>().unwrap();
            // assert_eq!(service_unit.get_unit_name(),"");
        }
    }
}
