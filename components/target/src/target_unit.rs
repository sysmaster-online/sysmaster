// target unit是 target类型的入口，需要实现
//UnitObj,UnitMngUtil,以及UnitSubClass三个trait
// UnitObj是Unit的抽象，定义Unit的具体行为
// UnitMngUtil是为了关联subUnit和Manger，由于rust不支持继承和多态，因此需要采用这种方式来间接支持
// UnitSubClass为了实现SubUnit到UnitObj的转换，简介达成多态的目的

use std::rc::Rc;

use super::target_mng::TargetMng;
use super::target_comm::TargetComm;
use process1::manager::{UnitObj,UnitSubClass,UnitMngUtil,UnitActiveState};
use utils::logger;

struct Target{
    comm: Rc<TargetComm>,
    mng: Rc<TargetMng>,
}

impl Target{
    fn new()-> Target{
        let _comm =  Rc::new(TargetComm::new());
        Target{
            comm: Rc::clone(&_comm),
            mng: Rc::new(TargetMng::new(&_comm))
        }
    }
}

impl UnitObj for Target{
    fn load(&self, _conf_str: &str) -> utils::Result<(), Box<dyn std::error::Error>> {
        //todo add default dependency funnction neeed add
        Ok(())
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.mng.to_unit_state()
    }

    fn attach_unit(&self, unit: Rc<process1::manager::Unit>) {
        self.comm.attach_unit(unit);
    }

    fn init(&self) {}

    fn done(&self) {}

    fn coldplug(&self) {}

    fn dump(&self) {}

    fn start(&self) -> utils::Result<(), process1::manager::UnitActionError> {
        //如果状态不对，直接返回失败
        self.mng.start_check()?;

        self.mng.start_action();
        Ok(())
    }

    fn stop(&self) -> utils::Result<(), process1::manager::UnitActionError> {
        self.mng.stop_check()?;

        self.mng.stop_action();
        Ok(())
    }

    fn reload(&self) {}

    fn kill(&self) {}

    fn release_resources(&self) {}

    fn sigchld_events(&self, _pid: nix::unistd::Pid, _code: i32, _status: nix::sys::signal::Signal) {}

    fn reset_failed(&self) {}
}

impl UnitSubClass for Target{
    fn into_unitobj(self: Box<Self>) -> Box<dyn UnitObj> {
       Box::new(*self)
    }
}


impl UnitMngUtil for Target{
    fn attach(&self, _um: Rc<process1::manager::UnitManager>) {
        return
    }
}

impl Default for Target{
    fn default() -> Self{
        Target::new()
    }
}


const LOG_LEVEL: u32 = 4;
const PLUGIN_NAME: &str = "TargetUnit";
use process1::declure_unitobj_plugin;
declure_unitobj_plugin!(Target, Target::default, PLUGIN_NAME, LOG_LEVEL);

#[cfg(test)]
mod tests{
    
}