use super::manager::{UnitSubClass, UnitType};
use dynamic_reload as dy_re;
use log::*;
use once_cell::sync::Lazy;
use std::ffi::OsStr;
use std::sync::RwLock;
use std::{collections::HashMap, error::Error, path::PathBuf, sync::Arc};
use std::{env, io};
use walkdir::{DirEntry, WalkDir};

const LIB_PLUGIN_PATH: &str = "/usr/lib/process1/plugin/";

static INSTANCE: Lazy<Arc<Plugin>> = Lazy::new(||{
    let plugin = Plugin::new();
    let default_lib_path = Plugin::get_default_libpath();
    plugin.update_library_dir(&default_lib_path);
    Arc::new(plugin)
});

pub struct Plugin {
    /*unitobj_lists: RefCell<Vec<Arc<Box<dyn UnitSubClass>>>>,//hide unit obj mut use refcell*/
    library_dir: RwLock<Vec<String>>,
    load_libs: RwLock<HashMap<UnitType, Arc<dy_re::Lib>>>,
    _loaded: RwLock<bool>,
}

#[allow(dead_code)]
impl Plugin {
    fn new() -> Self {
        Self {
            //unitobj_lists: RefCell::new(Vec::new()),
            library_dir: RwLock::new(Vec::new()),
            load_libs: RwLock::new(HashMap::new()),
            _loaded: RwLock::new(false),
        }
    }

    fn get_default_libpath() -> String {
        let mut ret: String = String::with_capacity(256);
        let devel_path = || {
            let out_dir = env::var("OUT_DIR");
            out_dir
        };
        let lib_path = env::var("PROCESS_LIB_LOAD_PATH");
        match lib_path {
            Ok(lib_path_str) => {
                let _tmp: Vec<_> = lib_path_str.split("target").collect();
                ret.push_str(format!("{}/target/debug;", _tmp[0]).as_str());
                ret.push_str(format!("{}/target/release;", _tmp[0]).as_str());
            }
            Err(_) => {
                let _tmp_lib_path = devel_path();
                let lib_path_str = _tmp_lib_path.unwrap_or(LIB_PLUGIN_PATH.to_string());
                let _tmp: Vec<_> = lib_path_str.split("target").collect();
                if _tmp.is_empty() {
                    ret.push_str(lib_path_str.as_str());
                } else {
                    ret.push_str(format!("{}target/debug;", _tmp[0]).as_str());
                    ret.push_str(format!("{}target/release;", _tmp[0]).as_str());
                }
            }
        }
        ret
    }


    pub fn get_instance() -> Arc<Plugin> {
        INSTANCE.clone()
    }

    fn load_lib(&self) {
        let file_exist = |file_name: &str| {
            if file_name.is_empty() {
                return false;
            }
            let libdir_path = PathBuf::from(file_name);
            if !libdir_path.exists() || !libdir_path.is_dir() {
                log::error!("load_lib library path [{:?}] is not a dir or not exist", libdir_path);
                return false;
            }
            true
        };

        if *(self._loaded.read().unwrap()) {
            log::info!("load_lib plugin is already loaded");
            return;
        }

        let lib_path = self.library_dir.read().unwrap();
        let search_path: Vec<&str> = (*lib_path)
            .iter()
            .map(|x| {
                let a = file_exist(x);
                if a {
                    x
                } else {
                    ""
                }
            })
            .collect();

        let shadow_dir = search_path[0];

        let mut reload_handler = dynamic_reload::DynamicReload::new(
            Some(search_path),
            Some(shadow_dir),
            dynamic_reload::Search::Default,
        );
        for file_item in lib_path.iter() {
            log::debug!("begin loading  plugin library in libraray path [{:?}]", file_item);
            for entry in WalkDir::new(file_item)
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
        }

        let mut _load = self._loaded.write().unwrap();
        (*_load) = true;
    }

    fn load_plugin(
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
    ///
    /// default plugin library path is /usr/lib/process1/plugin/
    /// if you want respecfic yourself path invoke this interface
    /// if the path is not different than last one the path will update
    /// add lib will reload
    pub fn update_library_dir(&self, library_dir: &str) {
        let update_dir = || {
            let _tmp_str: Vec<_> = library_dir.split(";").collect();
            let mut _new_dir: Vec<PathBuf> = Vec::new();
            let mut set_flag = false;

            for new_item in _tmp_str {
                if new_item.is_empty() {
                    continue;
                }
                let new_libdir = PathBuf::from(new_item);
                if !new_libdir.is_dir() || !new_libdir.is_dir() {
                    log::error!( " the path [{}] is not a dir/not exist", new_item);
                    continue;
                } else {
                    let mut _tmp_flag = false;
                    match self.library_dir.try_read() {
                        Ok(pathdir) => {
                            for item in (*pathdir).iter() {
                                let old_libdir = PathBuf::from(item);
                                if old_libdir == new_libdir {
                                    log::info!("update_library_path [{}] is already  in  the variable of libaray load path ", item);
                                    _tmp_flag = true;
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("update_library_path set [{}] into load path variable  failed,reason: {}", new_item, e.to_string());
                            return false;
                        }
                    }
                    if !_tmp_flag {
                        let dir_str = new_libdir.to_str().unwrap();
                        let mut w = self.library_dir.write().unwrap();
                        (*w).push(dir_str.to_string());
                        log::debug!("update_library_path set [{}] into library load path variable sucessful", dir_str);
                        set_flag = true;
                    }
                }
            }
            if set_flag {
                let mut _load = self._loaded.write().unwrap();
                (*_load) = false;
            }
            return set_flag;
        };
        log::debug!("begine update library load path [{}]", library_dir);
        if update_dir() {
            Self::load_lib(self);
        }
    }

    fn is_dynamic_lib(entry: &DirEntry) -> bool {
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
        if !(*(self._loaded.read().unwrap())) {
            log::info!("plugin is not loaded");
            return Err(format!("plugin is not loaded").into());
        }
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
    use utils::logger;

    use super::*;
    // use services::service::ServiceUnit;

    #[test]
    fn test_plugin_load_library() {
        logger::init_log_with_console("test_unit_load", 4);
        let plugins = Arc::clone(&Plugin::get_instance());
        let t_p = plugins;
        let mf = env!("CARGO_MANIFEST_DIR");
        let out_dir = env!("OUT_DIR");
        println!("{},{}", out_dir, mf);
        for key in (*t_p.load_libs.read().unwrap()).keys() {
            assert_eq!(key.to_string(), UnitType::UnitService.to_string());
            // let service_unit = u_box.as_any().downcast_ref::<ServiceUnit>().unwrap();
            // assert_eq!(service_unit.get_unit_name(),"");
        }
    }
}
