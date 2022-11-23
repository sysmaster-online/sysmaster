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

bitflags! {
    /// notify unit state to manager
    pub struct UnitNotifyFlags: u8 {
        /// notify reload failure to manager
        const UNIT_NOTIFY_RELOAD_FAILURE = 1 << 0;
        /// notify auto restart to manager
        const UNIT_NOTIFY_WILL_AUTO_RESTART = 1 << 1;
    }
}
