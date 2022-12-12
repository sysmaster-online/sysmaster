use sysmaster::unit::{UnitActiveState, UnitNotifyFlags};

#[derive(Debug)]
pub(in crate::core) struct UnitState {
    pub(in crate::core) os: UnitActiveState,
    pub(in crate::core) ns: UnitActiveState,
    pub(in crate::core) flags: UnitNotifyFlags,
}

impl UnitState {
    pub(in crate::core) fn new(
        os: UnitActiveState,
        ns: UnitActiveState,
        flags: UnitNotifyFlags,
    ) -> UnitState {
        UnitState { os, ns, flags }
    }
}
