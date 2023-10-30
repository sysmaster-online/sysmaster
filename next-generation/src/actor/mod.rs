use tokio::sync::mpsc::{channel, Sender};

use crate::actor::{
    dep::DepStore, guard::GuardStore, mount_monitor::MountMonitorStore, state::StateStore,
    unit::UnitStore,
};

pub(crate) mod dep;
pub(crate) mod guard;
pub(crate) mod state;
pub(crate) mod unit;

mod mount_monitor;
mod test;

pub(crate) struct Actors {
    pub(crate) store: Sender<unit::Message>,
    pub(crate) state: Sender<state::Message>,
    pub(crate) guard: Sender<guard::Message>,
    pub(crate) dep: Sender<dep::Message>,
    pub(crate) mount_monitor: Sender<mount_monitor::Message>,
}

impl Actors {
    pub(crate) fn new() -> Self {
        // 1024 should be big enough for normal use
        const CHANNEL_LEN: usize = 1024;

        let (unit, unit_rx) = channel(CHANNEL_LEN);
        let (state, state_rx) = channel(CHANNEL_LEN);
        let (guard, guard_rx) = channel(CHANNEL_LEN);
        let (dep, dep_rx) = channel(CHANNEL_LEN);
        let (mount_monitor, mount_monitor_rx) = channel(CHANNEL_LEN);

        UnitStore::new(dep.clone()).run(unit_rx);
        StateStore::new(dep.clone()).run(state_rx);
        GuardStore::new(
            dep.clone(),
            state.clone(),
            unit.clone(),
            mount_monitor.clone(),
        )
        .run(guard_rx);
        DepStore::new(dep.clone(), state.clone(), guard.clone()).run(dep_rx);
        MountMonitorStore::new(guard.clone()).run(mount_monitor_rx);

        Self {
            store: unit,
            state,
            guard,
            dep,
            mount_monitor,
        }
    }
}
