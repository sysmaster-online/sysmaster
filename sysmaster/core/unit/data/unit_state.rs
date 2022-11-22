use bitflags::bitflags;


bitflags! {
    /// notify unit state to manager
    pub struct UnitNotifyFlags: u8 {
        /// notify reload failure to manager
        const UNIT_NOTIFY_RELOAD_FAILURE = 1 << 0;
        /// notify auto restart to manager
        const UNIT_NOTIFY_WILL_AUTO_RESTART = 1 << 1;
    }
}

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
