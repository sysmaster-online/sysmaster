use super::unit_sets::UnitSets;
use crate::manager::signals::ProcessExit;
use crate::manager::unit::unit_entry::UnitX;
use nix::sys::signal::Signal;
use nix::sys::wait::{WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;

pub(super) struct UnitChild {
    data: RefCell<UnitChildData>,
}

impl UnitChild {
    pub(super) fn new(units: Rc<UnitSets>) -> UnitChild {
        UnitChild {
            data: RefCell::new(UnitChildData::new(units)),
        }
    }

    pub(super) fn dispatch_sigchld(&self) -> Result<(), Box<dyn Error>> {
        self.data.borrow_mut().dispatch_sigchld()
    }

    pub(super) fn add_watch_pid(&self, pid: Pid, id: &str) {
        self.data.borrow_mut().add_watch_pid(pid, id)
    }

    pub(super) fn unwatch_pid(&self, pid: Pid) {
        self.data.borrow_mut().unwatch_pid(pid)
    }
}

struct UnitChildData {
    units: Rc<UnitSets>,
    watch_pids: HashMap<Pid, Rc<UnitX>>, // key: pid, value: unit
}

// the declaration "pub(self)" is for identification only.
impl UnitChildData {
    pub(self) fn new(units: Rc<UnitSets>) -> UnitChildData {
        UnitChildData {
            units,
            watch_pids: HashMap::new(),
        }
    }

    pub(self) fn dispatch_sigchld(&mut self) -> Result<(), Box<dyn Error>> {
        log::debug!("Dispatching sighandler waiting for pid");
        let wait_pid = Pid::from_raw(-1);
        let flags = WaitPidFlag::WNOHANG;
        let process_exit = {
            match nix::sys::wait::waitpid(wait_pid, Some(flags)) {
                Ok(wait_status) => match wait_status {
                    WaitStatus::Exited(pid, code) => {
                        ProcessExit::Status(pid, code, Signal::SIGCHLD)
                    }
                    WaitStatus::Signaled(pid, signal, _dumped_core) => {
                        ProcessExit::Status(pid, -1, signal)
                    }
                    _ => {
                        log::debug!("Ignored child signal: {:?}", wait_status);
                        return Err(format!("Ignored child signal: {:?}", wait_status).into());
                    }
                },
                Err(e) => {
                    log::error!("Error while waiting pid: {}", e);
                    return Err(format!("Error while waiting pid: {}", e).into());
                }
            }
        };

        match process_exit {
            ProcessExit::Status(pid, code, signal) => {
                match self.watch_pids.get(&pid) {
                    Some(unit) => {
                        unit.sigchld_events(pid, code, signal);
                    }
                    None => {
                        log::debug!("not found unit obj of pid: {:?}", pid);
                        return Err(format!("not found unit obj of pid: {:?}", pid).into());
                    }
                }

                self.watch_pids.remove(&pid);
                Ok(())
            }
        }
    }

    pub(self) fn add_watch_pid(&mut self, pid: Pid, id: &str) {
        let unit = self.units.get(id).unwrap();
        self.watch_pids.insert(pid, unit);
    }

    pub(self) fn unwatch_pid(&mut self, pid: Pid) {
        self.watch_pids.remove(&pid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::data::{DataManager, UnitType};
    use crate::manager::unit::unit_file::UnitFile;
    use crate::manager::unit::unit_parser_mgr::UnitParserMgr;
    use crate::plugin::Plugin;
    use std::path::PathBuf;
    use utils::logger;

    #[test]
    #[should_panic]
    fn child_add_watch_pid_empty() {
        let sets = UnitSets::new();
        let name_test3 = String::from("test3.service");
        let child = UnitChild::new(Rc::new(sets));
        let pid = Pid::from_raw(1);

        child.add_watch_pid(pid, &name_test3);
    }

    #[test]
    fn child_add_watch_pid() {
        let sets = UnitSets::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&name_test2);
        sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        sets.insert(name_test2.clone(), Rc::clone(&unit_test2));
        let child = UnitChild::new(Rc::new(sets));
        let pid1 = Pid::from_raw(1);
        let pid2 = Pid::from_raw(2);

        assert_eq!(child.data.borrow().watch_pids.len(), 0);

        child.add_watch_pid(pid1, &name_test1);
        assert_eq!(child.data.borrow().watch_pids.len(), 1);

        child.add_watch_pid(pid2, &name_test2);
        assert_eq!(child.data.borrow().watch_pids.len(), 2);
    }

    #[test]
    fn child_unwatch_pid() {
        let sets = UnitSets::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&name_test2);
        sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        sets.insert(name_test2.clone(), Rc::clone(&unit_test2));
        let child = UnitChild::new(Rc::new(sets));
        let pid1 = Pid::from_raw(1);
        let pid2 = Pid::from_raw(2);

        assert_eq!(child.data.borrow().watch_pids.len(), 0);

        child.add_watch_pid(pid1, &name_test1);
        child.add_watch_pid(pid2, &name_test2);
        assert_eq!(child.data.borrow().watch_pids.len(), 2);

        child.unwatch_pid(pid1);
        assert_eq!(child.data.borrow().watch_pids.len(), 1);

        child.unwatch_pid(pid2);
        assert_eq!(child.data.borrow().watch_pids.len(), 0);
    }

    fn create_unit(name: &str) -> Rc<UnitX> {
        logger::init_log_with_console("test_unit_load", 4);
        log::info!("test");
        let dm = Rc::new(DataManager::new());
        let file = Rc::new(UnitFile::new());
        let unit_conf_parser_mgr = Rc::new(UnitParserMgr::default());
        let unit_type = UnitType::UnitService;
        let plugins = Rc::clone(&Plugin::get_instance());
        let mut config_path1 = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        config_path1.push("../target/debug");
        plugins
            .borrow_mut()
            .set_library_dir(&config_path1.to_str().unwrap());
        plugins.borrow_mut().load_lib();
        let mut config_path2 = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        config_path2.push("../target/release");
        plugins
            .borrow_mut()
            .set_library_dir(&config_path2.to_str().unwrap());
        plugins.borrow_mut().load_lib();
        let subclass = plugins.borrow().create_unit_obj(unit_type).unwrap();
        Rc::new(UnitX::new(
            dm,
            file,
            unit_conf_parser_mgr,
            unit_type,
            name,
            subclass.into_unitobj(),
        ))
    }
}
