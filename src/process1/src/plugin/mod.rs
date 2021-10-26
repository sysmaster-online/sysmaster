
use dynamic_reload;
use std::sync::Arc;
use std::time::Duration;
use std::thread;

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
    loader_dir: Option<String>
}


impl <T>Plugin<T> {
    fn init() {

    }
}