//! mount子类的核心逻辑
//!
use std::{cell::RefCell, rc::Rc};

use crate::mount_comm::MountComm;
use process1::manager::{UnitActiveState, UnitNotifyFlags};

// Mount包含两个状态，未挂载Dead，已挂载Mounted。对应到unit状态为inactive和active
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum MountState {
    Dead,
    Mounted,
}

impl MountState {
    fn to_unit_state(&self) -> UnitActiveState {
        match *self {
            MountState::Dead => UnitActiveState::UnitInActive,
            MountState::Mounted => UnitActiveState::UnitActive,
        }
    }
}

pub(super) struct MountMng {
    comm: Rc<MountComm>,
    state: RefCell<MountState>,
}

impl MountMng {
    pub(super) fn new(_comm: &Rc<MountComm>) -> Self {
        MountMng {
            comm: Rc::clone(_comm),
            state: RefCell::new(MountState::Dead),
        }
    }

    // process doesn't support manually mount/umount like systemd.
    // We only monitor the state of mountpoint.

    pub(super) fn enter_dead(&self) {
        self.set_state(MountState::Dead);
    }

    pub(super) fn enter_mounted(&self) {
        self.set_state(MountState::Mounted);
    }

    fn set_state(&self, new_state: MountState) {
        let old_state = self.state();
        self.state.replace(new_state);
        if new_state != old_state {
            log::debug!(
                "{} original state[{:?}] -> new state[{:?}]",
                self.comm
                    .unit()
                    .map_or_else(|| "0".to_string(), |u| u.get_id().to_string()),
                old_state,
                new_state,
            );
        }

        let old_unit_state = old_state.to_unit_state();
        let new_unit_state = new_state.to_unit_state();
        if let Some(_u) = self.comm.unit() {
            _u.notify(
                old_unit_state,
                new_unit_state,
                UnitNotifyFlags::UNIT_NOTIFY_RELOAD_FAILURE,
            );
        }
    }
    fn state(&self) -> MountState {
        *self.state.borrow()
    }

    pub fn to_unit_state(&self) -> UnitActiveState {
        self.state().to_unit_state()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::MountComm;
    use super::MountMng;
    use super::MountState;
    #[test]
    fn test_mount_set_state() {
        let _comm = Rc::new(MountComm::new());
        let tm = MountMng::new(&_comm);
        tm.set_state(MountState::Mounted);
        assert_eq!(tm.state(), MountState::Mounted)
    }

    #[test]
    fn test_mount_enter_dead() {
        let _comm = Rc::new(MountComm::new());
        let tm = MountMng::new(&_comm);
        tm.enter_dead();
        assert_eq!(tm.state(), MountState::Dead)
    }

    #[test]
    fn test_mount_enter_mounted() {
        let _comm = Rc::new(MountComm::new());
        let tm = MountMng::new(&_comm);
        tm.enter_mounted();
        assert_eq!(tm.state(), MountState::Mounted)
    }

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
