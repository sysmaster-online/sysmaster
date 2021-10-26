
use std::sync::Arc;
pub trait  Mangerobj {
    fn init(&self){

    }

    fn load(&self);

    fn dispatch(&self) -> i32;

    fn reload(&self) -> Option<i32>;

    fn destroy(&self);
    
    // reserved for sd event
    fn event_dispatch(&self) -> Option<i32>;
}



pub struct MangerPlugins  {
    pub managers: Vec<Box <dyn Mangerobj>>,
}


impl  MangerPlugins{
    pub fn new() -> Self{
        MangerPlugins{
            managers: Vec::new()
        }
    }
    pub fn load_plugins(&mut self, d: Box<dyn Mangerobj>) {
            self.managers.push(d);
        }

        pub fn run(&mut self) -> i32{
            let mut ret:i32 = 0;
            for m in self.managers.iter(){
                m.init();
                m.load();
                ret =m.dispatch();
            }
            ret
        }

        pub fn destroy(&self) {
            for m in self.managers.iter(){
                m.destroy();
            }
        }

        pub fn reload(&self){
            for m in self.managers.iter(){
                m.reload();
            }
        }
}


