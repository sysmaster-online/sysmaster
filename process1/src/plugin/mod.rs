
use dynamic_reload as dy_re;
use std::sync::Arc;
use walkdir::{DirEntry,WalkDir};
use std::path::Path;
use std::ffi::OsStr;
use log::*;
use std::io;
use super::manager::unit;

pub struct Plugin {
    unitobj_lists: Vec<Arc<Box<dyn unit::UnitObj>>>,
    library_dir: String,
    load_libs: Vec<Arc<dy_re::Lib>>,
}

#[allow(dead_code)]
impl Plugin  {
    fn new() -> Self {
        Self{
            unitobj_lists: Vec::new(),
            library_dir: String::new(),
            load_libs:  Vec::new(),
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
                    Self::load_plugin(self,file_name.unwrap(),&mut reload_handler);
                }
            }
        }
    }

    pub fn load_plugin(&mut self, filename: &OsStr, reload_handler: & mut dynamic_reload::DynamicReload) -> io::Result<()> {
        if let Some(v)  = filename.to_str(){    
            match reload_handler.add_library(v,dynamic_reload::PlatformName::Yes){
                Ok(lib) =>{
                    self.load_libs.push(lib.clone());
                    let dy_lib = self.load_libs.last().unwrap();
                    let fun: dynamic_reload::Symbol< fn()-> *mut dyn unit::UnitObj> = unsafe{dy_lib.lib.get(b"__unit_obj_create").unwrap()};
                    let boxed_raw = fun();
                    self.unitobj_lists.push(Arc::new(unsafe{Box::from_raw(boxed_raw)}));
                    debug!("loading dynamic lib sucessfully");
                }
                Err(e)  => error!("error loading Unable to load dynamic lib, err {:?}", e),
            }
        }
        Ok(())
    }

    pub fn set_library_dir(&mut self,library_dir: &str){
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

}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::service::ServiceUnit;

    #[test]
    fn test_plugin_load_library(){
        let mut plugins: Plugin = Plugin::new();
        plugins.set_library_dir("target/debug");
        plugins.load_lib();
        for uniobj in plugins.unitobj_lists {
            let u = Arc::clone(&uniobj);
            let u_box = unsafe{Arc::into_raw(u).as_ref().unwrap()};
            let service_unit = u_box.as_any().downcast_ref::<ServiceUnit>().unwrap();
            assert_eq!(service_unit.get_unit_name(),"");
        }
    }

}
