use nix::sys::signal::Signal;


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



/// kill operation send to process
#[allow(missing_docs)]
#[derive(PartialEq, Eq)]
pub enum KillOperation {
    KillTerminate,
    KillTerminateAndLog,
    KillRestart,
    KillKill,
    KillWatchdog,
    KillInvalid,
}

impl KillOperation {
    ///
    pub fn to_signal(&self) -> Signal {
        match *self {
            KillOperation::KillTerminate
            | KillOperation::KillTerminateAndLog
            | KillOperation::KillRestart => Signal::SIGTERM,
            KillOperation::KillKill => Signal::SIGKILL,
            KillOperation::KillWatchdog => Signal::SIGABRT,
            _ => Signal::SIGTERM,
        }
    }
}
