use bitflags::bitflags;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum UnitActiveState {
    UnitActive,
    UnitReloading,
    UnitInActive,
    UnitFailed,
    UnitActivating,
    UnitDeActivating,
    UnitMaintenance,
}

bitflags! {
    pub struct UnitNotifyFlags: u8 {
        const UNIT_NOTIFY_RELOAD_FAILURE = 1 << 0;
        const UNIT_NOTIFY_WILL_AUTO_RESTART = 1 << 1;
    }
}

#[derive(Debug)]
pub struct UnitState {
    pub os: UnitActiveState,
    pub ns: UnitActiveState,
    pub flags: UnitNotifyFlags,
}

impl UnitState {
    pub fn new(os: UnitActiveState, ns: UnitActiveState, flags: UnitNotifyFlags) -> UnitState {
        UnitState { os, ns, flags }
    }
}
