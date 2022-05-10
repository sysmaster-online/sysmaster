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

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum UnitNotifyFlags {
    UnitNotifyReloadFailure = 1 << 0,
    UnitNotifyWillAutoRestart = 1 << 1,
}

#[derive(Debug)]
pub struct UnitState {
    pub os: UnitActiveState,
    pub ns: UnitActiveState,
    pub flags: isize,
}

impl UnitState {
    pub fn new(os: UnitActiveState, ns: UnitActiveState, flags: isize) -> UnitState {
        UnitState { os, ns, flags }
    }
}
