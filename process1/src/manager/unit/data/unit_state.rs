use bitflags::bitflags;

/**Unit包含6种状态，Unit状态的转换关系如下：
 A[UnitActive]:已启动;
 B[UnitReloading]:重新加载;
 C[UnitInActive]:未启动;
 D[UnitFailed]:启动失败;
 E[UnitActivating]:正在激活中，属于瞬时状态;
 F[UnitDeActivating]:关闭过程中;
 G[UnitMaintenance]:维护清理过程中;
 ```graph LR
C[UnitInActive] -> E[UnitActivating]
E->A[UnitActive]
B[UnitReloading] -> E
E->F[UnitDeActivating]
E->D[UnitFailed]
```
*/

///
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum UnitActiveState {
    ///
    UnitActive,
    ///
    UnitReloading,
    ///
    UnitInActive,
    ///
    UnitFailed,
    ///
    UnitActivating,
    ///
    UnitDeActivating,
    ///
    UnitMaintenance,
}

bitflags! {
    ///
    pub struct UnitNotifyFlags: u8 {
        ///
        const UNIT_NOTIFY_RELOAD_FAILURE = 1 << 0;
        ///
        const UNIT_NOTIFY_WILL_AUTO_RESTART = 1 << 1;
    }
}

#[derive(Debug)]
pub(in crate::manager) struct UnitState {
    pub(in crate::manager) os: UnitActiveState,
    pub(in crate::manager) ns: UnitActiveState,
    pub(in crate::manager) flags: UnitNotifyFlags,
}

impl UnitState {
    pub(in crate::manager) fn new(
        os: UnitActiveState,
        ns: UnitActiveState,
        flags: UnitNotifyFlags,
    ) -> UnitState {
        UnitState { os, ns, flags }
    }
}
