use super::job::JobManager;
use super::unit_datastore::UnitDb;
use crate::manager::rentry::ReliLastFrame;
use crate::reliability::{ReStation, Reliability};
use libevent::{EventState, EventType, Events, Source};
use libutils::Result;
use nix::errno::Errno;
use nix::sys::signal::Signal;
use nix::sys::wait::{self, Id, WaitPidFlag, WaitStatus};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub(super) struct Sigchld {
    // associated objects
    jm: Rc<JobManager>,

    // owned objects
    sub: Rc<SigchldSub>,
}

impl ReStation for Sigchld {
    // input
    fn input_rebuild(&self) {
        self.enable(true).unwrap();
    }

    // compensate
    fn db_compensate_last(&self, _lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        if let Some(unit_id) = lunit {
            // merge to trigger
            self.jm.rentry_trigger_merge(unit_id, true);
        }
    }

    fn do_compensate_last(&self, _lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        if let Some(unit_id) = lunit {
            // re-run
            self.jm.trigger_unit(unit_id);
        }
    }

    // no data

    // reload: no external connections, no entry
}

impl Sigchld {
    pub(super) fn new(
        eventr: &Rc<Events>,
        relir: &Rc<Reliability>,
        dbr: &Rc<UnitDb>,
        jmr: &Rc<JobManager>,
    ) -> Sigchld {
        Sigchld {
            jm: Rc::clone(jmr),
            sub: SigchldSub::new(eventr, relir, dbr),
        }
    }

    pub(super) fn enable(&self, enable: bool) -> Result<i32> {
        self.sub.enable(enable)
    }
}

// make event happy
struct SigchldSub {
    // associated objects
    event: Rc<Events>,

    // owned objects
    data: Rc<SigchldData>,
}

// the declaration "pub(self)" is for identification only.
impl SigchldSub {
    pub(self) fn new(
        eventr: &Rc<Events>,
        relir: &Rc<Reliability>,
        dbr: &Rc<UnitDb>,
    ) -> Rc<SigchldSub> {
        let sub = Rc::new(SigchldSub {
            event: Rc::clone(eventr),
            data: Rc::new(SigchldData::new(relir, dbr)),
        });
        sub.data.set_sub(&sub);
        sub.register();
        sub
    }

    pub(self) fn enable(&self, enable: bool) -> Result<i32> {
        println!("sigchild enable.");
        let source = Rc::clone(&self.data);
        let state = match enable {
            true => EventState::On,
            false => EventState::Off,
        };
        self.event.set_enabled(source, state)?;
        Ok(0)
    }

    fn register(&self) {
        // event
        let source = Rc::clone(&self.data);
        self.event.add_source(source).unwrap();
    }
}

struct SigchldData {
    // associated objects
    reli: Rc<Reliability>,
    db: Rc<UnitDb>,
    sub: RefCell<Weak<SigchldSub>>,
}

impl Source for SigchldData {
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
        println!("sigchld dispatch");

        self.reli.set_last_frame1(ReliLastFrame::SigChld as u32);
        let enable = self.sigchld_dispatch();
        self.reli.clear_last_frame();

        if !enable {
            self.sub().enable(false)?;
        }

        Ok(0)
    }
}

// the declaration "pub(self)" is for identification only.
impl SigchldData {
    pub(self) fn new(relir: &Rc<Reliability>, dbr: &Rc<UnitDb>) -> SigchldData {
        SigchldData {
            reli: Rc::clone(relir),
            db: Rc::clone(dbr),
            sub: RefCell::new(Weak::new()),
        }
    }

    pub(self) fn sigchld_dispatch(&self) -> bool {
        log::debug!("Dispatching sighandler waiting for pid");

        // peek signal
        let flags = WaitPidFlag::WEXITED | WaitPidFlag::WNOHANG | WaitPidFlag::WNOWAIT;
        let ret = wait::waitid(Id::All, flags);
        if let Err(err) = ret {
            if err != Errno::ECHILD {
                log::error!("Error while waiting pid: {}", err);
            }
            return false; // turn_off
        }

        // get wait information
        let mut turn_on = true;
        let wait_status = ret.unwrap();
        let si = match wait_status {
            WaitStatus::Exited(pid, code) => Some((pid, code, Signal::SIGCHLD)),
            WaitStatus::Signaled(pid, signal, _dc) => Some((pid, -1, signal)),
            WaitStatus::StillAlive => {
                turn_on = false;
                None
            } // nothing to wait, turn_off
            _ => None, // ignore it, stay turn_on
        };
        if si.is_none() {
            log::debug!(
                "Ignored child signal: {:?}, turn_on: {}",
                wait_status,
                turn_on
            );
            return turn_on;
        }

        // check
        let (pid, code, signal) = si.unwrap();
        if pid.as_raw() <= 0 {
            log::debug!("invalid pid in signal: {:?}", pid);
            return false; // turn_off
        }

        // record + action
        if let Some(unit) = self.db.get_unit_by_pid(pid) {
            self.reli.set_last_unit(unit.id());
            unit.sigchld_events(pid, code, signal);
            self.db.child_unwatch_pid(unit.id(), pid);
            self.reli.clear_last_unit();
        } else {
            log::debug!("not found unit obj of pid: {:?}", pid);
        }

        // pop: reap the zombie
        if let Err(e) = wait::waitid(Id::Pid(pid), WaitPidFlag::WEXITED) {
            log::error!("Error when reap the zombie, ignoring: {}", e);
        }

        true // stay turn_on
    }

    pub(self) fn set_sub(&self, sub: &Rc<SigchldSub>) {
        self.sub.replace(Rc::downgrade(sub));
    }

    pub(self) fn sub(&self) -> Rc<SigchldSub> {
        self.sub.clone().into_inner().upgrade().unwrap()
    }
}
