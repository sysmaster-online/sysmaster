use std::collections::{hash_map::Entry, HashMap};

use tokio::{
    select,
    sync::{
        mpsc::{self, Receiver, Sender},
        oneshot,
    },
    task::{yield_now, JoinHandle},
};

use super::{
    dep, mount_monitor,
    state::{self, set_state},
    unit::{self, utils::get_unit},
};
use crate::{
    actor::state::set_state_with_condition,
    unit::{RtMsg, State, UnitId, UnitKind, UnitObj},
};

struct Extra {}

pub(crate) enum GuardMessage {
    DepsReady,
    DepsFailed,
    Stop,
    NotifyDead,
}

/// the guard during the lifetime of the unit
struct Guard {
    unit: UnitObj,
    extra: Option<Extra>,
    state: Sender<state::Message>,
}

impl Guard {
    fn new(unit: UnitObj, extra: Option<Extra>, state: Sender<state::Message>) -> Self {
        Self { unit, extra, state }
    }

    /// state:
    /// 1. wait deps(afters) to start(be active)
    ///     - afters: active
    ///     - requires: Starting?
    /// 2. unit start:
    ///      1. set state to starting
    ///      2. run `unit.start` (todo: prestart -> start -> post start)
    ///      3. match unit.start {
    ///             Success => set state to `Active`
    ///             Failed => set state to `Failed` and exit
    ///         }
    /// 3. wait & monitor the unit to exit \
    /// or wait stop sig and kill the unit by run `unit.stop`
    fn run(self, mut rx: Receiver<GuardMessage>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let id = UnitId::from(self.unit.as_ref());
            // wait deps
            if let Some(msg) = rx.recv().await {
                match msg {
                    GuardMessage::DepsReady => (),
                    GuardMessage::DepsFailed => {
                        set_state(&self.state, id.clone(), State::Failed).await;
                        return;
                    }
                    GuardMessage::Stop => {
                        set_state(&self.state, id.clone(), State::Stopped).await; // maybe unnecessary since the unit is not active here?
                        return;
                    }
                    GuardMessage::NotifyDead => todo!(),
                }
            }

            // run start
            while let Err(old_state) =
                set_state_with_condition(&self.state, id.clone(), State::Starting, |s| s.is_dead())
                    .await
            {
                match old_state {
                    // wait the stopping instance
                    State::Stopping => yield_now().await,
                    _ => todo!(),
                }
            }

            let mut handle = match self.unit.start().await {
                Ok(handle) => handle,
                Err(()) => {
                    println!("unit start failed!");
                    set_state(&self.state, id.clone(), State::Failed).await;
                    return;
                }
            };
            set_state(&self.state, id.clone(), State::Active).await;

            // started, wait stop_sig / quit
            let state = loop {
                select! {
                    msg = rx.recv() => match msg.unwrap() {
                        GuardMessage::DepsReady | GuardMessage::DepsFailed => todo!("unreachable: log error for guard {}", id),
                        GuardMessage::Stop => {
                            set_state(&self.state, id.clone(), State::Stopping).await;
                            match self.unit.stop(handle).await {
                                Ok(()) => break State::Stopped,
                                Err(()) => todo!(),
                            }
                        },
                        GuardMessage::NotifyDead => {
                            break State::Stopped
                        }
                    },
                    rt_msg = handle.wait() => match rt_msg {
                        RtMsg::Yield => (),
                        RtMsg::Exit(state) => break state,
                        RtMsg::TriggerStart(id, extra) => {
                            // todo: start the unit with extra rt info
                        }
                    },
                }
            };
            set_state(&self.state, id.clone(), state).await;
        })
    }
}

#[derive(Debug)]
pub(crate) enum Message {
    /// Query if guard of the specific unit exists
    Contains(UnitId, oneshot::Sender<bool>),
    /// Insert a guard.
    Insert(UnitId),
    /// remove a guard \
    /// usually called by self when a gurad quits
    Remove(UnitId),
    /// notice all deps are ready for a specific unit \
    /// called by `Dep`
    DepsReady(UnitId),
    /// notice there's at least one required dep of the specific unit failed
    DepsFailed(UnitId),
    /// Send a Stop message to the specific unit guard
    Stop(UnitId),
    /// Notify a unit that it already dead
    NotifyDead(UnitId),
}

#[derive(Debug, Clone)]
pub(crate) struct GuardStore {
    map: HashMap<UnitId, Sender<GuardMessage>>,
    dep: Sender<dep::Message>,
    state: Sender<state::Message>,
    unit: Sender<unit::Message>,
    mount_monitor: Sender<mount_monitor::Message>,
}

impl GuardStore {
    pub(crate) fn new(
        dep: Sender<dep::Message>,
        state: Sender<state::Message>,
        unit: Sender<unit::Message>,
        mount_monitor: Sender<mount_monitor::Message>,
    ) -> Self {
        Self {
            map: Default::default(),
            dep,
            state,
            unit,
            mount_monitor,
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::Contains(id, sender) => {
                        let ret = match self.map.entry(id) {
                            Entry::Occupied(o) if o.get().is_closed() => {
                                o.remove();
                                false
                            }
                            Entry::Occupied(_) => true,
                            Entry::Vacant(_) => false,
                        };
                        sender.send(ret).unwrap();
                    }
                    Message::Insert(id) => {
                        println!("guard: inserting {}", id);
                        let unitobj = get_unit(&self.unit, id.clone()).await.unwrap();
                        // hack for mountpoint monitor
                        if unitobj.kind() == UnitKind::Mount {
                            self.mount_monitor
                                .send(mount_monitor::Message::Register(id.clone()))
                                .await
                                .unwrap();
                        }
                        match self.map.entry(id.clone()) {
                            Entry::Occupied(mut o) if o.get().is_closed() => {
                                let (sender, receiver) = mpsc::channel(4); // todo: remove magic number
                                Guard::new(unitobj, None, self.state.clone()).run(receiver);
                                o.insert(sender);
                            }
                            Entry::Occupied(_) => {
                                println!("insert {} when guard already exists!", id)
                            }
                            Entry::Vacant(v) => {
                                // unit not running, create the guard to start the unit
                                let (sender, receiver) = mpsc::channel(4); // todo: remove magic number
                                Guard::new(unitobj, None, self.state.clone()).run(receiver);
                                v.insert(sender);
                            }
                        }
                    }
                    Message::Remove(id) => {
                        // leak if guard running?
                        self.map.remove(&id);
                    }
                    Message::DepsReady(id) => {
                        self.map
                            .get(&id)
                            .unwrap()
                            .send(GuardMessage::DepsReady)
                            .await
                            .ok(); // ignore error here since guard already dropped, this is useless to send
                    }
                    Message::DepsFailed(id) => {
                        self.map
                            .get(&id)
                            .unwrap()
                            .send(GuardMessage::DepsFailed)
                            .await
                            .ok();
                    }
                    Message::Stop(id) => {
                        if let Some(guard) = self.map.get(&id) {
                            guard.send(GuardMessage::Stop).await.ok(); // ignore error here since guard already dropped, this is useless to send
                        }
                        if id.kind() == UnitKind::Mount {
                            self.mount_monitor
                                .send(mount_monitor::Message::Remove(id))
                                .await
                                .unwrap();
                        }
                    }
                    Message::NotifyDead(id) => {
                        self.map
                            .get(&id)
                            .unwrap()
                            .send(GuardMessage::NotifyDead)
                            .await
                            .ok();
                    }
                }
            }
        })
    }
}

// pub(crate) async fn create_guard(guard_manager: &Sender<Message>, u: UnitId) {
//     guard_manager.send(Message::Insert(u)).await.unwrap();
// }

// pub(crate) async fn guard_stop(guard_manager: &Sender<Message>, u: UnitId) {
//     guard_manager.send(Message::Stop(u)).await.unwrap()
// }

pub(crate) async fn is_guard_exists(guard_manager: &Sender<Message>, u: UnitId) -> bool {
    let (s, r) = oneshot::channel();
    guard_manager.send(Message::Contains(u, s)).await.unwrap();
    r.await.unwrap()
}
