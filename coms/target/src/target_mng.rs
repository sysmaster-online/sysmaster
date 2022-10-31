//! is the core of the target unit
//!
use super::target_comm::TargetUnitComm;
use super::target_rentry::TargetState;
use libsysmaster::manager::{UnitActionError, UnitActiveState, UnitNotifyFlags};
use libsysmaster::ReStation;
use libutils::IN_SET;
use std::{cell::RefCell, rc::Rc};

impl TargetState {
    fn to_unit_state(self) -> UnitActiveState {
        match self {
            TargetState::Dead | TargetState::StateMax => UnitActiveState::UnitInActive,
            TargetState::Active => UnitActiveState::UnitActive,
        }
    }
}

pub(super) struct TargetMng {
    comm: Rc<TargetUnitComm>,
    state: RefCell<TargetState>,
}

impl ReStation for TargetMng {
    // no input, no compensate

    // data
    fn db_map(&self) {
        if let Some(state) = self.comm.rentry_mng_get() {
            *self.state.borrow_mut() = state;
        }
    }

    fn db_insert(&self) {
        self.comm.rentry_mng_insert(self.state());
    }

    // reload: no external connections, no entry
}

impl TargetMng {
    pub(super) fn new(_comm: &Rc<TargetUnitComm>) -> Self {
        TargetMng {
            comm: Rc::clone(_comm),
            state: RefCell::new(TargetState::StateMax),
        }
    }

    pub(super) fn start_check(&self) -> Result<(), UnitActionError> {
        if IN_SET!(self.state(), TargetState::Active) {
            return Err(UnitActionError::UnitActionEAgain);
        }
        Ok(())
    }

    pub(super) fn start_action(&self, notify: bool) {
        //todo notify dbus is not implemented
        self.set_state(TargetState::Active, notify);
    }

    pub(super) fn stop_check(&self) -> Result<(), UnitActionError> {
        if IN_SET!(self.state(), TargetState::Dead) {
            return Err(UnitActionError::UnitActionEAgain);
        }
        Ok(())
    }

    pub(super) fn stop_action(&self, notify: bool) {
        self.set_state(TargetState::Dead, notify);
    }

    fn set_state(&self, new_state: TargetState, notify: bool) {
        let old_state = self.state();
        self.state.replace(new_state);

        if notify {
            self.state_notify(new_state, old_state);
        }
    }

    fn state_notify(&self, new_state: TargetState, old_state: TargetState) {
        if new_state != old_state {
            log::debug!(
                "{} original state[{:?}] ->new state[{:?}]",
                self.comm.unit().id(),
                old_state,
                new_state,
            );
        }

        let old_unit_state = old_state.to_unit_state();
        let new_unit_state = new_state.to_unit_state();
        self.comm.unit().notify(
            old_unit_state,
            new_unit_state,
            UnitNotifyFlags::UNIT_NOTIFY_RELOAD_FAILURE,
        );

        self.db_update();
    }

    fn state(&self) -> TargetState {
        *self.state.borrow()
    }

    pub fn to_unit_state(&self) -> UnitActiveState {
        self.state().to_unit_state()
    }
}

#[cfg(test)]
mod tests {
    use super::TargetMng;
    use super::TargetState;
    use super::TargetUnitComm;
    use std::rc::Rc;

    #[test]
    fn test_target_set_state() {
        let _comm = Rc::new(TargetUnitComm::new());
        let tm = TargetMng::new(&_comm);
        tm.set_state(TargetState::Active, false);
        assert_eq!(tm.state(), TargetState::Active)
    }

    #[test]
    fn test_target_stop_action() {
        let _comm = Rc::new(TargetUnitComm::new());
        let tm = TargetMng::new(&_comm);
        tm.stop_action(false);
        assert_eq!(tm.state(), TargetState::Dead)
    }
}
