//! target 子类的核心逻辑
//!
use std::{cell::RefCell, rc::Rc};

use process1::manager::{UnitActionError, UnitActiveState, UnitNotifyFlags};
use utils::IN_SET;

use crate::target_comm::TargetComm;

// target 包含两个状态，Dead和Active，对应到unit状态为inactive和 active
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum TargetState {
    Dead,
    Active,
    StateMax,
}

impl TargetState {
    fn to_unit_state(&self) -> UnitActiveState {
        match *self {
            TargetState::Dead | TargetState::StateMax => UnitActiveState::UnitInActive,
            TargetState::Active => UnitActiveState::UnitActive,
        }
    }
}

pub(super) struct TargetMng {
    comm: Rc<TargetComm>,
    state: RefCell<TargetState>,
}

impl TargetMng {
    pub(super) fn new(_comm: &Rc<TargetComm>) -> Self {
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

    pub(super) fn start_action(&self) {
        //todo notify dbus功能暂时未实现
        self.set_state(TargetState::Active);
    }

    pub(super) fn stop_check(&self) -> Result<(), UnitActionError> {
        if IN_SET!(self.state(), TargetState::Dead) {
            return Err(UnitActionError::UnitActionEAgain);
        }
        Ok(())
    }

    pub(super) fn stop_action(&self) {
        self.set_state(TargetState::Dead);
    }

    fn set_state(&self, state: TargetState) {
        let o_state = self.state();
        self.state.replace(state);
        if state != o_state {
            log::debug!(
                "{} original state[{:?}] ->new state[{:?}]",
                self.comm.unit().map_or_else(||"0".to_string(),|u| u.get_id().to_string()),
                o_state,
                state,
            );
        }

        let old_unit_state = o_state.to_unit_state();
        let new_unit_state = state.to_unit_state();
        if let Some(_u) = self.comm.unit() {
            _u.notify(
                old_unit_state,
                new_unit_state,
                UnitNotifyFlags::UNIT_NOTIFY_RELOAD_FAILURE,
            );
        }
    }
    
    fn state(&self) -> TargetState {
        *self.state.borrow()
    }

    pub fn to_unit_state(&self) -> UnitActiveState{
        self.state().to_unit_state()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::TargetComm;
    use super::TargetMng;
    use super::TargetState;
    #[test]
    fn test_target_set_state() {
        let _comm = Rc::new(TargetComm::new());
        let tm = TargetMng::new(&_comm);
        tm.set_state(TargetState::Active);
        assert_eq!(tm.state(), TargetState::Active)
    }

    #[test]
    fn test_target_stop_action() {
        let _comm = Rc::new(TargetComm::new());
        let tm = TargetMng::new(&_comm);
        tm.stop_action();
        assert_eq!(tm.state(), TargetState::Dead)
    }

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
