
use dynamic_reload;
use std::sync::Arc;
use std::time::Duration;
use walkdir::{DirEntry,WalkDir};
use std::path::Path;

struct LibLoaders{
    libs: Vec<Arc<dynamic_reload::Lib>>,
}

impl LibLoaders {
    fn add_lib(&mut self, lib: &Arc<dynamic_reload::Lib>) {
        self.libs.push(lib.clone());
    }

    fn unload_lib(&mut self, lib: &Arc<dynamic_reload::Lib>) {
        for i in (0..self.libs.len()).rev() {
            if &self.libs[i] == lib {
                self.libs.swap_remove(i);
            }
        }
    }
    fn reload_lib(&mut self,lib:&Arc<dynamic_reload::Lib>){
        Self::add_lib(self,lib);
    }

    fn reload_callback(&mut self, state:dynamic_reload::UpdateState, lib: Option<&Arc<dynamic_reload::Lib>>){
        match state {
            dynamic_reload::UpdateState::Before => Self::unload_lib(self,lib.unwrap()),
            dynamic_reload::UpdateState::After => Self::reload_lib(self,lib.unwrap()),
            dynamic_reload::UpdateState::ReloadFailed(_) =>println!("Reload plugin failed"),
        }
    }
}


pub struct Plugin<T> {
    plugin_lists: Vec<Arc<T>>,
    library_dir: String,
    lib_loader: LibLoaders    
}

#[allow(dead_code)]
impl <T>Plugin<T> {
    fn new() -> Self {
        Self{
            plugin_lists: Vec::new(),
            library_dir: String::new(),
            lib_loader:  LibLoaders{libs:Vec::new()},

        }
    }

    pub fn set_library_dir(&mut self,library_dir:&str){
        self.library_dir.push_str(library_dir);
    }

    pub fn is_dynamic_lib(entry: &DirEntry) -> bool{
        let file_type = entry.file_type();
        let file_name = entry.file_name();
        if file_type.is_file() && file_name.to_str().map(|s| s.ends_with(".so.*")).unwrap_or(false) {
            true
        } else {
            false
        }
    }

    pub fn load_lib(&mut self){
        let file_exist = || {
            if self.library_dir.is_empty() {
                return false
            }
            if !Path::new(&self.library_dir).is_dir(){
                return false
            }
            true
        };
        if !file_exist() {
            return
        }else{
            let mut reload_handler = dynamic_reload::DynamicReload::new(Some(vec![self.library_dir.as_str()]), Some(self.library_dir.as_str()), dynamic_reload::Search::Default);
            for entry in WalkDir::new(&self.library_dir)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| Self::is_dynamic_lib(e)){
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir(){
                    continue;
                }else{
                    let file_name = path.file_name();
                    match file_name {
                        Some(v) => {
                            let str_name = v.to_str().unwrap();
                            match reload_handler.add_library(&str_name, dynamic_reload::PlatformName::Yes){
                                Ok(lib) =>{
                                    println!("loader dynamic lib in to lib_loader");
                                    self.lib_loader.add_lib(&lib);
                                }
                                Err(e) =>{
                                    println!("Unable to load dynamic lib, err {:?}", e);
                                }
                            }
                        }
                        None =>{
                            println!("file name is None");
                        }
                    }
                }
                
            }               
        }
    }
}
