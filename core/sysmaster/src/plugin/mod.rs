// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! Plugin provides a plug-in management mechanism, completes the dynamic loading of unit subclasses,
//!  and loads the so files in the specified directory. The priority of the specified directory is as follows:
//! a. First find the dynamic library under the /usr/lib/sysmaster/plugin/ path
//! b. Find the output directory of the rust cargo build such as target/debug/ or target/release
//! c. The path specified by the PROCESS_LIB_LOAD_PATH environment variable.
//! In the development stage of using cargo, the two methods b and c actually point to the /target/debug or release directory
//!  under the checkout directory of the sysmaster project, for example
//! sysmaster is cloned into the /home/test directory, the output directory is /home/test/target/debug or release directory
//! 2. The subclass type and the corresponding so mapping relationship configuration file, the default search path is the same as
//!  the search path of the subclass dynamic library. The path of the file under the source tree is sysmaster/conf/plugin.conf
//! In the development stage, it will be released to the /target/debug or release directory by default through the build script.
//!  This stage does not need to be concerned. If you need to run sysmaster separately,
//! The configuration file needs to be copied from the build release directory (target/debug/conf) to the
//!  /usr/lib/sysmaster/plugin directory, otherwise sysmaster cannot load the corresponding so file.
//! Change the configuration format of the file to unitType:soname, such as:
//! ````text
//! Service:libservice
//! Target:libtarget
//! Socket: libsocket
//! ````
//! 3. The implementation of the subclass imports the following macro definitions
//! ```macro_rules
//! const LOG_LEVEL: u32 = 4;
//! const PLUGIN_NAME: &str = "TargetUnit";
//! use core::declare_unitobj_plugin;
//! declare_unitobj_plugin!(Target, Target::default, PLUGIN_NAME, LOG_LEVEL);
//! ````
//! plugin or find the corresponding so according to the name of the corresponding unit configuration file, and load it dynamically, such as XXX.service to find libservice.so, XXX.socket to find libsocket.so
//!
use core::error::*;
use core::unit::UmIf;
use core::unit::{SubUnit, UnitManagerObj, UnitType};
use dy_re::Lib;
use dy_re::Symbol;
use dynamic_reload as dy_re;
use log::*;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use walkdir::{DirEntry, WalkDir};

const LIB_PLUGIN_PATH: &str = "/usr/lib/sysmaster/plugin/";

static INSTANCE: Lazy<Arc<Plugin>> = Lazy::new(|| {
    let plugin = Plugin::new();
    let default_lib_path = Plugin::get_default_libpath();
    let unit_type_lib_map = Plugin::get_unit_type_lib_map();
    plugin.init_unit_type_lib_map(&unit_type_lib_map);
    plugin.update_library_dir(&default_lib_path);
    Arc::new(plugin)
});

/// Plugin provides a plug-in management mechanism, completes the dynamic loading of unit subclasses,
/// and loads the so files in the specified directory. The priority of the specified directory is as follows:
///a. First find the dynamic library under the /usr/lib/sysmaster/plugin/ path
///b. Find the output directory of the rust cargo build such as target/debug/ or target/release
///c. The path specified by the PROCESS_LIB_LOAD_PATH environment variable.
pub struct Plugin {
    /*unitobj_lists: RefCell<Vec<Arc<Box<dyn UnitSubClass>>>>,//hide unit obj mut use refcell*/
    lib_type: RwLock<Vec<(String, String)>>,
    library_dir: RwLock<Vec<String>>,
    load_libs: RwLock<HashMap<UnitType, Arc<dy_re::Lib>>>,
    loaded: RwLock<bool>,
}

impl Plugin {
    fn new() -> Self {
        Self {
            //unitobj_lists: RefCell::new(Vec::new()),
            lib_type: RwLock::new(Vec::new()),
            library_dir: RwLock::new(Vec::new()),
            load_libs: RwLock::new(HashMap::new()),
            loaded: RwLock::new(false),
        }
    }

    fn get_unit_type_lib_map() -> String {
        let mut buf = String::with_capacity(256);

        let check_other_path = || {
            #[cfg(test)]
            let out_dir = env::var("OUT_DIR").unwrap_or_else(|_x| {
                let _tmp_str: Option<&'static str> = option_env!("OUT_DIR");
                _tmp_str.unwrap_or("").to_string()
            });

            #[cfg(not(test))]
            let out_dir = String::new();

            if out_dir.is_empty() {
                env::var("PROCESS_LIB_LOAD_PATH").map_or("".to_string(), |_v| _v)
            } else {
                out_dir
            }
        };

        let mut conf_file = format!("{}plugin.conf", LIB_PLUGIN_PATH);
        let mut path = Path::new(&conf_file);
        if !path.exists() {
            let lib_path_str = check_other_path();
            log::debug!(
                "plugin conf file not found in:[{}],try find in devel path:[{}]",
                conf_file,
                lib_path_str,
            );

            if lib_path_str.contains("build") {
                let _tmp: Vec<_> = lib_path_str.split("build").collect();
                conf_file = format!("{}/conf/plugin.conf", _tmp[0]);
            } else {
                conf_file = format!("{}/conf/plugin.conf", lib_path_str);
            }
            path = Path::new(&conf_file);
        }

        let display = path.display();
        match File::open(path) {
            Ok(mut _f) => {
                if let Ok(_s) = _f.read_to_string(&mut buf) {
                    log::debug!("plugin support library is {}", buf);
                } else {
                    log::error!("library type is not config");
                }
            }
            Err(e) => {
                log::error!(
                    "library type config file is not found,err msg {}:{:?}",
                    display,
                    e
                );
            }
        }
        buf
    }

    fn get_default_libpath() -> String {
        let mut ret: String = String::with_capacity(256);
        #[cfg(test)]
        let lib_path_devel = {
            let devel_path = |out_dir: Option<&str>| {
                if let Some(outdir) = out_dir {
                    if outdir.contains("build") {
                        let _tmp: Vec<_> = outdir.split("build").collect();
                        String::from(_tmp[0])
                    } else {
                        outdir.to_string()
                    }
                } else {
                    String::new()
                }
            };
            devel_path(option_env!("OUT_DIR"))
        };

        #[cfg(not(test))]
        let lib_path_devel = String::new();

        let lib_path_env = env::var("PROCESS_LIB_LOAD_PATH").map_or("".to_string(), |_v| _v);
        let _lib_path = [
            LIB_PLUGIN_PATH,
            lib_path_devel.as_str(),
            lib_path_env.as_str(),
        ];
        for _tmp_str in _lib_path {
            if _tmp_str.is_empty() {
                continue;
            }
            let path = Path::new(_tmp_str);
            if !path.exists() || !path.is_dir() {
                continue;
            } else {
                ret.push_str(_tmp_str);
                break;
            }
        }
        ret
    }

    /// Get a instance of plugin
    /// Plugin is a singleton instance
    ///
    /// # Examples
    ///
    /// ```
    /// use core::plugin::Plugin;
    ///
    /// Plugin::get_instance();
    /// ```
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
                log::error!(
                    "load_lib library path [{:?}] is not a dir or not exist",
                    libdir_path
                );
                return false;
            }
            true
        };

        let is_dynamic_lib = |entry: &DirEntry| {
            let file_type = entry.file_type();
            let file_name = entry.file_name();
            file_type.is_file()
                && file_name
                    .to_str()
                    .map(|s| s.ends_with(".so"))
                    .unwrap_or(false)
        };

        if *(self.loaded.read().unwrap()) {
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
            Duration::from_secs(2),
        );

        for file_item in lib_path.iter() {
            log::debug!(
                "begin loading plugin library in library path [{:?}]",
                file_item
            );
            for entry in WalkDir::new(file_item)
                .min_depth(1)
                .follow_links(true)
                .into_iter()
                .filter_entry(|e| is_dynamic_lib(e))
            {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    continue;
                } else if let Some(file_name) = path.file_name() {
                    match Self::load_plugin(self, file_name, &mut reload_handler) {
                        Ok(_) => {
                            log::info!("Plugin load unit plugin[{:?}] successful", file_name);
                        }
                        Err(e) => {
                            log::error!(
                                "Plugin load unit plugin[{:?}] failed, detail is {}",
                                file_name,
                                e.to_string()
                            );
                        }
                    }
                }
            }
        }

        *self.loaded.write().unwrap() = true;
    }

    fn load_plugin(
        &self,
        filename: &OsStr,
        reload_handler: &mut dynamic_reload::DynamicReload,
    ) -> Result<()> {
        let v = match filename.to_str() {
            None => return Ok(()),
            Some(v) => v,
        };

        let unit_type = self.get_unit_type(v);
        if unit_type == UnitType::UnitTypeInvalid {
            log::error!("lib name {} is invalid, skipping", v);
            return Ok(());
        }

        let lib = match unsafe { reload_handler.add_library(v, dynamic_reload::PlatformName::No) } {
            Err(e) => {
                error!("Unable to loading dynamic lib, err {:?}", e);
                return Ok(());
            }
            Ok(v) => v,
        };

        /* We need the lib*.so pass SubUnit and UnitManagerObj to sysmaster-core, and we check if
         * subunit_create_with_params and um_obj_create exist in lib*.so, because these two functions
         * will be used to generate SubUnit and UnitManagerObj. */

        type FnTypeSubUnit = fn(um: Rc<dyn UmIf>) -> *mut dyn SubUnit;
        type FnTypeUnitManagerObj = fn(
            um: Rc<dyn UmIf>,
            level: Level,
            target: &str,
            file_size: u32,
            file_number: u32,
        ) -> *mut dyn UnitManagerObj;

        /* 1. check if the lib contains __subunit_create_with_params */
        #[allow(clippy::type_complexity)]
        let symunit: Result<Symbol<FnTypeSubUnit>> = unsafe {
            lib.lib
                .get(b"__subunit_create_with_params")
                .map_err(|e| Error::PluginLoad { msg: e.to_string() })
        };

        if symunit.is_err() {
            log::error!(
                "Lib {} doesn't contain __subunit_create_with_params, skipping",
                v
            );
            return Ok(());
        }

        /* 2. check if the lib contains __um_obj_create */
        #[allow(clippy::type_complexity)]
        let symum: Result<Symbol<FnTypeUnitManagerObj>> = unsafe {
            lib.lib
                .get(b"__um_obj_create")
                .map_err(|e| Error::PluginLoad { msg: e.to_string() })
        };
        if symum.is_err() {
            log::error!("Lib {} doesn't contain __um_obj_create, skipping", v);
            return Ok(());
        }

        /* 3. Insert */
        log::debug!("Inserting {:?} dynamic lib", unit_type);
        let mut wloadlibs = self.load_libs.write().unwrap();
        (*wloadlibs).insert(unit_type, lib.clone());
        log::info!("Successfully loaded dynamic lib");

        Ok(())
    }

    fn get_unit_type(&self, name: &str) -> UnitType {
        let read_s = self.lib_type.read().unwrap();
        for line in read_s.iter() {
            if name.contains(&line.1) {
                return UnitType::from_str(&line.0).unwrap();
            }
        }
        UnitType::UnitTypeInvalid
    }

    fn init_unit_type_lib_map(&self, unit_type_lib_map: &str) {
        for line in unit_type_lib_map.lines() {
            let _v_s: Vec<_> = line.split(':').collect();
            let mut _lib_w = self.lib_type.write().unwrap();
            (*_lib_w).push((_v_s[0].to_string(), _v_s[1].to_string()));
        }
    }
    ///
    /// default plugin library path is /usr/lib/sysmaster/plugin/
    /// if you want respecfic yourself path invoke this interface
    /// if the path is not different than last one the path will update
    /// add lib will reload
    fn update_library_dir(&self, library_dir: &str) {
        let update_dir = || {
            let _tmp_str: Vec<_> = library_dir.split(';').collect();
            let mut _new_dir: Vec<PathBuf> = Vec::new();
            let mut set_flag = false;

            for new_item in _tmp_str {
                if new_item.is_empty() {
                    continue;
                }
                let new_libdir = PathBuf::from(new_item);
                if !new_libdir.exists() || !new_libdir.is_dir() {
                    log::error!(" the path [{}] is not a dir/not exist", new_item);
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
                        log::debug!("update_library_path set [{}] into library load path variable successful", dir_str);
                        set_flag = true;
                    }
                }
            }
            if set_flag {
                let mut _load = self.loaded.write().unwrap();
                (*_load) = false;
            }
            set_flag
        };
        log::debug!("begain update library load path [{}]", library_dir);
        if update_dir() {
            self.load_lib();
        }
    }

    /// Create the subunit trait of unit
    /// each sub unit need reference of declare_unitobj_plugin_with_param
    ///
    pub fn create_subunit_with_um(
        &self,
        unit_type: UnitType,
        um: Rc<dyn UmIf>,
    ) -> Result<Box<dyn SubUnit>> {
        type FnType = fn(um: Rc<dyn UmIf>) -> *mut dyn SubUnit;

        let dy_lib = match self.get_lib(unit_type) {
            Err(_) => {
                return Err(Error::PluginLoad {
                    msg: format!("create unit, the {:?} plugin is not exist", unit_type),
                })
            }
            Ok(v) => v,
        };

        let sym: Result<Symbol<FnType>> = unsafe {
            dy_lib
                .lib
                .get(b"__subunit_create_with_params")
                .map_err(|e| Error::PluginLoad { msg: e.to_string() })
        };

        let fun = match sym {
            Err(_) => {
                return Err(Error::PluginLoad {
                    msg: format!("The library of {:?} is {:?}", unit_type, sym.err()),
                })
            }
            Ok(v) => v,
        };

        log::debug!("create unit obj with params");
        let boxed_raw = fun(um.clone());
        Ok(unsafe { Box::from_raw(boxed_raw) })
    }
    /// Create a  obj for subclasses of unit manager
    /// each sub unit manager need reference of declare_umobj_plugin
    pub fn create_um_obj(
        &self,
        unit_type: UnitType,
        target: &str,
        file_size: u32,
        file_number: u32,
    ) -> Result<Box<dyn UnitManagerObj>> {
        let dy_lib = match self.get_lib(unit_type) {
            Err(_) => {
                return Err(Error::PluginLoad {
                    msg: format!("create um, the {:?} plugin is not exist", unit_type),
                });
            }
            Ok(v) => v,
        };

        type FnType = fn(
            level: Level,
            target: &str,
            file_size: u32,
            file_number: u32,
        ) -> *mut dyn UnitManagerObj;

        #[allow(clippy::type_complexity)]
        let sym: Result<Symbol<FnType>> = unsafe {
            dy_lib
                .lib
                .get(b"__um_obj_create")
                .map_err(|e| Error::PluginLoad { msg: e.to_string() })
        };
        let fun = match sym {
            Err(_) => {
                return Err(Error::PluginLoad {
                    msg: format!("The library of {:?} is {:?}", unit_type, sym.err()),
                })
            }
            Ok(v) => v,
        };

        let boxed_raw = fun(log::Level::max(), target, file_size, file_number);
        Ok(unsafe { Box::from_raw(boxed_raw) })
    }

    fn get_lib(&self, unit_type: UnitType) -> Result<Arc<Lib>> {
        if !(*(self.loaded.read().unwrap())) {
            log::info!("plugin is not loaded");
            return Err(Error::PluginLoad {
                msg: "plugin is not loaded".to_string(),
            });
        }

        let mut retry_count = 0;
        let dy_lib = loop {
            let dy_lib: Result<Arc<Lib>> = match (*self.load_libs.read().unwrap()).get(&unit_type) {
                Some(lib) => Ok(lib.clone()),
                None => Err(Error::PluginLoad {
                    msg: format!("the {:?} plugin is not exist", unit_type),
                }),
            };
            if dy_lib.is_err() {
                if retry_count < 2 {
                    retry_count += 1;
                    self.load_lib();
                    continue;
                } else {
                    return Err(Error::PluginLoad {
                        msg: format!("the {:?} plugin is not exist", unit_type),
                    });
                }
            }
            break dy_lib;
        };

        dy_lib
    }
}

#[cfg(test)]
mod tests {

    use core::unit::UmIf;

    use super::*;
    // use services::service::ServiceUnit;

    struct UmIfD;
    impl UmIf for UmIfD {}

    fn init_test() -> Arc<Plugin> {
        log::init_log(
            "test_plugin",
            log::Level::Trace,
            vec!["console", "syslog"],
            "",
            0,
            0,
            false,
        );
        Arc::clone(&Plugin::get_instance())
    }

    #[test]
    fn test_plugin_load_library() {
        let t_p = init_test();
        let mf = env!("CARGO_MANIFEST_DIR");
        let out_dir = option_env!("OUT_DIR");
        log::info!("{:?},{}", out_dir, mf);
        for key in (*t_p.load_libs.read().unwrap()).keys() {
            // let service_unit = u_box.as_any().downcast_ref::<ServiceUnit>().unwrap();
            // assert_eq!(service_unit.get_unit_name(),"");
            assert_ne!(*key, UnitType::UnitTypeInvalid);
        }
    }

    #[test]
    fn test_plugin_create_unit() {
        let plugin = init_test();
        let umifd = Rc::new(UmIfD);
        let unitobj = plugin.create_subunit_with_um(UnitType::UnitService, umifd);
        assert!(
            unitobj.is_ok(),
            "create unit [{:?}] failed",
            UnitType::UnitService
        );
    }
}
