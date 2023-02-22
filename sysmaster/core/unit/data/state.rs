use sysmaster::unit::{UnitActiveState, UnitNotifyFlags};

#[derive(Debug)]
pub(crate) struct UnitState {
    pub(crate) os: UnitActiveState,
    pub(crate) ns: UnitActiveState,
    pub(crate) flags: UnitNotifyFlags,
}

impl UnitState {
    pub(crate) fn new(
        os: UnitActiveState,
        ns: UnitActiveState,
        flags: UnitNotifyFlags,
    ) -> UnitState {
        UnitState { os, ns, flags }
    }
}
