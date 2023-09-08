// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! is the core of the target unit
//!
use super::comm::TargetUnitComm;
use super::rentry::TargetState;
use basic::IN_SET;
use core::error::*;
use core::rel::ReStation;
use core::unit::{UnitActiveState, UnitNotifyFlags};
use std::{cell::RefCell, rc::Rc};

impl TargetState {
    fn to_unit_state(self) -> UnitActiveState {
        match self {
            TargetState::Dead | TargetState::StateMax => UnitActiveState::InActive,
            TargetState::Active => UnitActiveState::Active,
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
    fn db_map(&self, _reload: bool) {
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

    pub(super) fn start_check(&self) -> Result<()> {
        if IN_SET!(self.state(), TargetState::Active) {
            return Err(Error::UnitActionEAgain);
        }
        Ok(())
    }

    pub(super) fn start_action(&self, notify: bool) {
        //todo notify dbus is not implemented
        self.set_state(TargetState::Active, notify);
    }

    pub(super) fn stop_check(&self) -> Result<()> {
        if IN_SET!(self.state(), TargetState::Dead) {
            return Err(Error::UnitActionEAgain);
        }
        Ok(())
    }

    pub(super) fn stop_action(&self, notify: bool) {
        self.set_state(TargetState::Dead, notify);
    }

    pub fn get_state(&self) -> String {
        let state = *self.state.borrow();
        state.to_string()
    }

    fn set_state(&self, new_state: TargetState, notify: bool) {
        let old_state = self.state();
        self.state.replace(new_state);

        if notify {
            self.state_notify(new_state, old_state);
        }
    }

    fn state_notify(&self, new_state: TargetState, old_state: TargetState) {
        if let Some(unit) = self.comm.owner() {
            if new_state != old_state {
                log::debug!(
                    "{} original state[{:?}] ->new state[{:?}]",
                    unit.id(),
                    old_state,
                    new_state,
                );
            }
            let old_unit_state = old_state.to_unit_state();
            let new_unit_state = new_state.to_unit_state();
            unit.notify(
                old_unit_state,
                new_unit_state,
                UnitNotifyFlags::RELOAD_FAILURE,
            );

            self.db_update();
        }
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
        let comm = Rc::new(TargetUnitComm::new());
        let tm = TargetMng::new(&comm);
        tm.stop_action(false);
        assert_eq!(tm.state(), TargetState::Dead)
    }

    #[test]
    fn test_target_start_action() {
        let comm = Rc::new(TargetUnitComm::new());
        let tm = TargetMng::new(&comm);
        tm.start_action(false);
        assert_eq!(tm.state(), TargetState::Active)
    }
}
