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
    os: UnitActiveState,
    ns: UnitActiveState,
    flags: isize,
}

impl UnitState {
    pub fn new(os: UnitActiveState, ns: UnitActiveState, flags: isize) -> UnitState {
        UnitState { os, ns, flags }
    }

    pub fn get_os(&self) -> UnitActiveState {
        self.os
    }

    pub fn get_ns(&self) -> UnitActiveState {
        self.ns
    }

    pub fn get_flags(&self) -> isize {
        self.flags
    }
}
