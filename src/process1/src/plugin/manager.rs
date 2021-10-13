struct Manager <T>{
    id:String,
    name:String,
    desc:String,
    configdir:String,
    m_obj_list:Vec <T>,  //manage obj list
}

trait  MangerMethod <T>{
    fn init(self);

    fn load(&mut self);

    fn dispatch(&mut self) -> Option<i32>;

    fn relaod(&mut self) -> Option<i32>;

    fn destroy(&mut self);
    
    // reserved for sd event
    fn event_dispatch(&mut self) -> Option<i32>;
}

struct unit{

}
struct UnitManager {
    unitManager: Manager<unit>
}

impl MangerMethod <UnitManager> for UnitManager {
    fn init(self){
        let id = self.unitManager.id;
        self.unitManager.configdir;
    }
    
    fn load(&mut self){
        self.unitManager.m_obj_list.push(unit{});
    }

    fn dispatch(&mut self) -> Option<i32> {
        None
    }

    fn relaod(&mut self) -> Option<i32>{
        None
    }

    fn destroy(&mut self){

    }
    
    // reserved for sd event
    fn event_dispatch(&mut self) -> Option<i32>{
        None
    }


}

pub fn load_Manager(){
    //**此处应该支持扩展多种Manager */
    let manager:Manager<unit> = Manager{
        id: todo!(),
        name: todo!(),
        desc: todo!(),
        configdir: todo!(),
        m_obj_list: todo!(),
    };
    let um = UnitManager{unitManager:manager};
    um.init();
    um.load();
    um.dispatch();   
}
