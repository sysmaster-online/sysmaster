use bitflags::bitflags;

/**Unit stats：
 A[UnitActive]
 B[UnitReloading]
 C[UnitInActive]
 D[UnitFailed]
 E[UnitActivating]
 F[UnitDeActivating]
 G[UnitMaintenance]
 ```graph LR
C[UnitInActive] -> E[UnitActivating]
E->A[UnitActive]
B[UnitReloading] -> E
E->F[UnitDeActivating]
E->D[UnitFailed]
```
*/
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum UnitActiveState {
    /// unit is activated
    UnitActive,
    /// unit is in reloading
    UnitReloading,
    /// unit is not active
    UnitInActive,
    /// unit action is failed
    UnitFailed,
    /// unit is in starting
    UnitActivating,
    /// unit is in stopping
    UnitDeActivating,
    /// unit is in maintenance
    UnitMaintenance,
}

impl std::fmt::Display for UnitActiveState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnitActiveState::UnitActive => write!(f, "active"),
            UnitActiveState::UnitReloading => write!(f, "reloading"),
            UnitActiveState::UnitInActive => write!(f, "inactive"),
            UnitActiveState::UnitFailed => write!(f, "failed"),
            UnitActiveState::UnitActivating => write!(f, "activating"),
            UnitActiveState::UnitDeActivating => write!(f, "deactivating"),
            UnitActiveState::UnitMaintenance => write!(f, "maintenance"),
        }
    }
}

bitflags! {
    /// notify unit state to manager
    pub struct UnitNotifyFlags: u8 {
        /// notify reload failure to manager
        const UNIT_NOTIFY_RELOAD_FAILURE = 1 << 0;
        /// notify auto restart to manager
        const UNIT_NOTIFY_WILL_AUTO_RESTART = 1 << 1;
    }
}
