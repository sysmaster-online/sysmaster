use super::mount_base::{LOG_LEVEL, PLUGIN_NAME};
use super::mount_comm::MountUmComm;
use super::mount_rentry::{MountRe, MountReFrame};
use libevent::{EventState, EventType, Events, Source};
use libmount::mountinfo;
use libsysmaster::manager::{
    ReliLastFrame, UnitActiveState, UnitManager, UnitManagerObj, UnitMngUtil, UnitType,
};
use libsysmaster::{ReStation, Reliability};
use libutils::logger;
use libutils::{Error, Result};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::{AsRawFd, RawFd};
use std::rc::Rc;
use std::sync::Arc;

struct MountManager {
    // owned objects
    // control
    first_enum: RefCell<bool>,

    // data
    comm: Arc<MountUmComm>,
    monitor: MountMonitor,
}

impl ReStation for MountManager {
    // input
    fn input_rebuild(&self) {
        self.monitor.defer_enable(true).unwrap();
    }

    // compensate
    fn db_compensate_last(&self, _lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        assert!(lunit.is_some());

        let frame = self.comm.rentry().last_frame();
        if frame.is_none() {
            // debug
            return;
        }

        let unit_id = lunit.unwrap();
        match frame.unwrap() {
            MountReFrame::Monitor => self.comm.um().rentry_trigger_merge(unit_id, true), // merge to trigger
        }
    }

    fn do_compensate_last(&self, _lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        assert!(lunit.is_some());

        let frame = self.comm.rentry().last_frame();
        if frame.is_none() {
            // debug
            return;
        }

        let unit_id = lunit.unwrap();
        match frame.unwrap() {
            MountReFrame::Monitor => self.comm.um().trigger_unit(unit_id), // re-run
        }
    }

    // no data

    // reload
    fn register_ex(&self) {
        self.monitor.register_ex();
    }
}

impl UnitManagerObj for MountManager {
    fn enumerate(&self) {
        if !*self.first_enum.borrow() {
            self.register_ex();
            *self.first_enum.borrow_mut() = true;
        }
    }
}

// the declaration "pub(self)" is for identification only.
impl MountManager {
    pub(self) fn new() -> MountManager {
        let _comm = MountUmComm::get_instance();
        MountManager {
            first_enum: RefCell::new(false),
            comm: Arc::clone(&_comm),
            monitor: MountMonitor::new(&_comm),
        }
    }

    fn _parser(&self) -> Result<i32, Error> {
        let mut mount_data = String::new();
        File::open("/proc/self/mountinfo")
            .unwrap()
            .read_to_string(&mut mount_data)
            .unwrap();
        let parser = mountinfo::Parser::new(mount_data.as_bytes());
        for mount_result in parser {
            match mount_result {
                Ok(mount) => {
                    log::debug!(
                        "[{}] mounted on [{}]",
                        mount.mount_source.to_str().unwrap(),
                        mount.mount_point.to_str().unwrap()
                    );
                }
                Err(err) => {
                    println!("parse error: {}", err);
                }
            }
        }
        Ok(0)
    }

    fn register(&self) {
        self.monitor.register();
    }
}

struct MountMonitor {
    // owned objects
    data: Rc<MountMonitorData>,
    defer: Rc<MountMonitorDefer>,
    io: Rc<MountMonitorIo>,
}

// the declaration "pub(self)" is for identification only.
impl MountMonitor {
    pub(self) fn new(commr: &Arc<MountUmComm>) -> MountMonitor {
        let filename = "/proc/self/mountinfo".to_string();
        let _data = Rc::new(MountMonitorData::new(commr, filename));
        let _defer = Rc::new(MountMonitorDefer::new(&_data));
        MountMonitor {
            data: Rc::clone(&_data),
            defer: Rc::clone(&_defer),
            io: Rc::new(MountMonitorIo::new(&_data, &_defer)),
        }
    }

    pub(self) fn register_ex(&self) {
        let events = self.data.comm.um().events();

        // io
        let io = Rc::clone(&self.io);
        events.add_source(io).unwrap();
    }

    pub(self) fn defer_enable(&self, enable: bool) -> Result<i32> {
        self.io.defer_enable(enable)
    }

    fn register(&self) {
        let events = self.data.comm.um().events();

        // defer
        let defer = Rc::clone(&self.defer);
        events.add_source(defer).unwrap();
    }
}

struct MountMonitorIo {
    // associated objects
    data: Rc<MountMonitorData>,
    defer: Rc<MountMonitorDefer>,
}

// the declaration "pub(self)" is for identification only.
impl MountMonitorIo {
    pub(self) fn new(
        datar: &Rc<MountMonitorData>,
        deferr: &Rc<MountMonitorDefer>,
    ) -> MountMonitorIo {
        MountMonitorIo {
            data: Rc::clone(datar),
            defer: Rc::clone(deferr),
        }
    }

    pub(self) fn defer_enable(&self, enable: bool) -> Result<i32> {
        let source = Rc::clone(&self.defer);
        let state = match enable {
            true => EventState::OneShot,
            false => EventState::Off,
        };
        let events = self.data.comm.um().events();
        events.set_enabled(source, state)?;
        Ok(0)
    }

    fn reli(&self) -> Rc<Reliability> {
        self.data.comm.reli()
    }

    fn rentry(&self) -> Rc<MountRe> {
        self.data.comm.rentry()
    }
}

impl Source for MountMonitorIo {
    fn event_type(&self) -> EventType {
        EventType::Io
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn dispatch(&self, _e: &Events) -> Result<i32> {
        drain_out(self.data.epfd);

        self.reli()
            .set_last_frame2(ReliLastFrame::SubManager as u32, UnitType::UnitMount as u32);
        self.rentry().set_last_frame(MountReFrame::Monitor);
        let ret = self.data.dispatch_mountinfo();
        self.rentry().clear_last_frame();
        self.reli().clear_last_frame();

        if let Err(e) = ret {
            log::error!("Failed to dispatch mountinfo, ignoring: {:?}", e);
        }

        self.defer_enable(false)?;
        Ok(0)
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn fd(&self) -> RawFd {
        self.data.epfd
    }
}

struct MountMonitorDefer {
    // associated objects
    data: Rc<MountMonitorData>,
}

// the declaration "pub(self)" is for identification only.
impl MountMonitorDefer {
    pub(self) fn new(datar: &Rc<MountMonitorData>) -> MountMonitorDefer {
        MountMonitorDefer {
            data: Rc::clone(datar),
        }
    }

    fn reli(&self) -> Rc<Reliability> {
        self.data.comm.reli()
    }

    fn rentry(&self) -> Rc<MountRe> {
        self.data.comm.rentry()
    }
}

impl Source for MountMonitorDefer {
    fn event_type(&self) -> EventType {
        EventType::Defer
    }

    fn epoll_event(&self) -> u32 {
        0
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn dispatch(&self, _event: &Events) -> Result<i32> {
        println!("mount monitor dispatch");

        self.reli()
            .set_last_frame2(ReliLastFrame::SubManager as u32, UnitType::UnitMount as u32);
        self.rentry().set_last_frame(MountReFrame::Monitor);
        let ret = self.data.dispatch_mountinfo();
        self.rentry().clear_last_frame();
        self.reli().clear_last_frame();

        if let Err(_e) = ret {
            // debug
        }

        Ok(0)
    }
}

struct MountMonitorData {
    // associated objects
    comm: Arc<MountUmComm>,

    // owned objects
    _file: File,
    epfd: i32,
    _epevent: epoll::Event,
    _events: Vec<epoll::Event>,
}

impl MountMonitorData {
    pub(self) fn new(commr: &Arc<MountUmComm>, filename: String) -> Self {
        let me_file = match File::open(&filename) {
            Ok(me_file) => me_file,
            Err(why) => {
                panic!("Failed to open {}: {}", &filename, why);
            }
        };

        let me_fd = me_file.as_raw_fd();
        let me_epfd = epoll::create(false).unwrap();
        let event = epoll::Events::EPOLLET | epoll::Events::EPOLLIN;
        let me_epevent = epoll::Event::new(event, me_fd as u64);
        let op = epoll::ControlOptions::EPOLL_CTL_ADD;
        drain_out(me_epfd);
        epoll::ctl(me_epfd, op, me_fd, me_epevent).unwrap();

        log::debug!("Monitoring {} by epool fd {}", filename, me_epfd);
        MountMonitorData {
            comm: Arc::clone(commr),
            _file: me_file,
            epfd: me_epfd,
            _epevent: me_epevent,
            _events: Vec::new(),
        }
    }

    pub(self) fn dispatch_mountinfo(&self) -> Result<()> {
        // First mark all active mount point we have as dead.
        let mut dead_mount_set: HashSet<String> = HashSet::new();
        let unit_type = Some(UnitType::UnitMount);
        for unit in self.comm.um().units_get_all(unit_type).iter() {
            if unit.current_active_state() == UnitActiveState::UnitActive {
                dead_mount_set.insert(String::from(unit.id()));
            }
        }

        // Then start mount point we don't know.
        let mut mountinfo_content = String::new();
        File::open("/proc/self/mountinfo")
            .unwrap()
            .read_to_string(&mut mountinfo_content)
            .unwrap();
        let parser = mountinfo::Parser::new(mountinfo_content.as_bytes());
        for mount_result in parser {
            // pop
            match mount_result {
                Ok(mount) => {
                    // We don't process autofs for now, because it is not
                    // .mount but .automount in systemd.
                    if mount.fstype.to_str() == Some("autofs") {
                        continue;
                    }
                    let unit_name = mount_point_to_unit_name(mount.mount_point.to_str().unwrap());
                    if dead_mount_set.contains(unit_name.as_str()) {
                        dead_mount_set.remove(unit_name.as_str());
                    } else if let Some(unit) = self.comm.um().load_unit(unit_name.as_str()) {
                        // record + action
                        self.comm.reli().set_last_unit(unit.id());
                        let start_err = unit.start().is_err();
                        self.comm.reli().clear_last_unit();

                        if start_err {
                            log::debug!("{} change to mounted.", unit_name);
                        } else {
                            log::error!("Failed to start {}", unit_name);
                        }
                    }
                }
                Err(err) => {
                    log::error!("Failed to parse /proc/self/mountinfo: {}", err);
                }
            }
        }

        // Finally stop mount point in dead_mount_set.
        for unit_name in dead_mount_set.into_iter() {
            // pop
            if let Some(unit) = self.comm.um().units_get(unit_name.as_str()) {
                // record + action
                self.comm.reli().set_last_unit(unit.id());
                let ret = unit.stop(false);
                self.comm.reli().clear_last_unit();

                if ret.is_ok() {
                    log::debug!("{} change to dead.", unit_name);
                } else {
                    log::error!("Failed to stop {}.", unit_name);
                }
            }
        }
        Ok(())
    }
}

fn drain_out(epfd: i32) {
    // drain out all events.
    let mut me_events: Vec<epoll::Event> = vec![epoll::Event::new(epoll::Events::empty(), 0)];
    while epoll::wait(epfd, 0, &mut me_events).unwrap() > 0 {}
}

fn mount_point_to_unit_name(mount_point: &str) -> String {
    let mut res = String::from(mount_point).replace('/', "-") + ".mount";
    if res != "-.mount" {
        res = String::from(&res[1..])
    }
    res
}

impl UnitMngUtil for MountManager {
    fn attach_um(&self, um: Rc<UnitManager>) {
        self.comm.attach_um(um);
        self.register();
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

impl Default for MountManager {
    fn default() -> Self {
        MountManager::new()
    }
}

use libsysmaster::declure_umobj_plugin;
declure_umobj_plugin!(MountManager, MountManager::default, PLUGIN_NAME, LOG_LEVEL);

#[cfg(test)]
mod test {
    //use super::*;

    #[test]
    fn test_monitor() {
        //let event = Rc::new(Events::new().unwrap());
        //let reli = Rc::new(Reliability::new());
        //let comm = Rc::new(MountComm::new());
        //let mount_source = Rc::new(MountMonitor::new(&event, &reli, &comm));

        //event.add_source(mount_source.clone()).unwrap();
        //event.set_enabled(mount_source.clone(), EventState::On).unwrap();

        //event.run(10).unwrap();
    }
}
