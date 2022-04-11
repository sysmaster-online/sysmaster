use crate::manager::data::{DataManager, UnitActiveState, UnitState};
use std::rc::Rc;

pub(super) struct UeState {
    // associated objects
    dm: Rc<DataManager>,
}

impl UeState {
    pub(super) fn new(dm: Rc<DataManager>) -> UeState {
        UeState { dm }
    }

    pub(super) fn update(
        &self,
        name: &str,
        os: UnitActiveState,
        ns: UnitActiveState,
        flags: isize,
    ) {
        let u_state = UnitState::new(os, ns, flags);
        self.dm.insert_unit_state(name.to_string(), u_state);
    }
}
