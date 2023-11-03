use std::collections::{
    hash_map::{Entry, OccupiedEntry},
    HashMap, HashSet,
};

use futures_util::{stream, StreamExt};
use tap::Pipe;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use crate::{
    actor::guard::is_guard_exists,
    unit::{State, UnitDeps, UnitId},
    Rc,
};

use super::{
    guard,
    state::{self, get_state},
};

/// runtime mutable dep info, used to wait deps
///
/// we only want to know what is blocking this unit to start/stop
/// so only `after` is useful when starting and `before` is useful when stopping
/// this contains afters when starting and before when stopping
#[derive(Debug)]
enum JobWaitInfo {
    Start { after: HashSet<UnitId> },
    Stop { before: HashSet<UnitId> },
}

impl JobWaitInfo {
    fn is_empty(&self) -> bool {
        match self {
            JobWaitInfo::Start { after } => after.is_empty(),
            JobWaitInfo::Stop { before } => before.is_empty(),
        }
    }
}

// after is useless in ReverseDepInfo since what we want is triggers/blocking_relations here,
// self will never block afters start
#[derive(Default)]
struct FullDepInfo {
    required_by: HashSet<UnitId>,
    requires: HashSet<UnitId>,
    wanted_by: HashSet<UnitId>,
    wants: HashSet<UnitId>,
    before: HashSet<UnitId>,
    after: HashSet<UnitId>,
    conflicts: HashSet<UnitId>,
}

pub(crate) enum Message {
    /// Load depinfo of the unit
    Load(UnitId, Rc<UnitDeps>),
    /// add a unit waiting to start
    AddToStart(UnitId),
    /// add a unit waiting to stop
    AddToStop(UnitId),
    /// receive notify: state of the unit has changed
    StateChange(UnitId, State),
}
pub(crate) struct DepStore {
    pending_jobs: HashMap<UnitId, JobWaitInfo>,
    dep_map: HashMap<UnitId, FullDepInfo>,
    dep: Sender<Message>,
    state: Sender<state::Message>,
    guard: Sender<guard::Message>,
}

impl DepStore {
    pub(crate) fn new(
        dep: Sender<Message>,
        state: Sender<state::Message>,
        guard: Sender<guard::Message>,
    ) -> Self {
        Self {
            pending_jobs: Default::default(),
            dep_map: Default::default(),
            dep,
            state,
            guard,
        }
    }
    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::Load(id, deps) => {
                        let dmap = &mut self.dep_map;
                        dep_map_insert(&id, &deps.requires, dmap, |rdep| &mut rdep.requires);
                        dep_map_insert(&id, &deps.wants, dmap, |rdep| &mut rdep.wants);
                        dep_map_insert(&id, &deps.after, dmap, |rdep| &mut rdep.after);
                        dep_map_insert(&id, &deps.before, dmap, |rdep| &mut rdep.before);
                        dep_map_insert(&id, &deps.conflicts, dmap, |rdep| &mut rdep.conflicts);

                        dep_map_rinsert(&id, &deps.requires, dmap, |rdep| &mut rdep.required_by);
                        dep_map_rinsert(&id, &deps.wants, dmap, |rdep| &mut rdep.wanted_by);
                        dep_map_rinsert(&id, &deps.after, dmap, |rdep| &mut rdep.before);
                        dep_map_rinsert(&id, &deps.before, dmap, |rdep| &mut rdep.after);
                        dep_map_rinsert(&id, &deps.conflicts, dmap, |rdep| &mut rdep.conflicts);
                    }
                    Message::AddToStart(id) => self.add_to_start(id).await,
                    Message::AddToStop(id) => self.add_to_stop(id).await,
                    Message::StateChange(state_change_id, new_state) => {
                        let Self {
                            pending_jobs,
                            dep_map,
                            dep,
                            state,
                            guard,
                        } = &mut self;
                        if let Entry::Occupied(full_dep) = dep_map.entry(state_change_id.clone()) {
                            let full_dep = full_dep.get();
                            match new_state {
                                State::Uninit => unreachable!(),
                                State::Stopped => {
                                    handle_stopped(full_dep, pending_jobs, &state_change_id, guard)
                                        .await;
                                }
                                State::Failed => {
                                    // rule 3: failed, requires should fail
                                    for waiting_id in full_dep.required_by.iter().cloned() {
                                        pending_jobs.remove(&waiting_id);
                                        guard
                                            .send(guard::Message::DepsFailed(waiting_id))
                                            .await
                                            .unwrap()
                                    }
                                }
                                State::Starting => {
                                    // starting: things required by me should start
                                }
                                State::Active => {
                                    handle_active(full_dep, pending_jobs, state_change_id, guard)
                                        .await;
                                }
                                State::Stopping => {
                                    // stopping: things require me should stop
                                    // for unit in reverse_dep.required_by.iter().cloned() {
                                    //     guard.send(guard::Message::Stop(unit)).await.unwrap()
                                    // }
                                }
                            }
                        }
                    }
                }
            }
        })
    }

    async fn add_to_start(&mut self, id: UnitId) {
        // simple job merge
        let deps = self.dep_map.get(&id).unwrap();
        if let Entry::Occupied(o) = self.pending_jobs.entry(id.clone()) {
            match o.get() {
                // since there's already waiting for its deps
                // dont need to insert another time
                JobWaitInfo::Start { .. } => return,
                // remove the previous stop job here
                JobWaitInfo::Stop { .. } => {
                    o.remove();
                }
            }
        } else if get_state(&self.state, id.clone()).await.is_active() {
            handle_active(deps, &mut self.pending_jobs, id, &self.guard).await;
            return;
        } else {
            self.guard
                .send(guard::Message::Insert(id.clone()))
                .await
                .unwrap();
        }
        // build wait list: find what is blocking the start operation
        let wait_list = JobWaitInfo::Start {
            after: stream::iter(deps.after.iter().cloned())
                .filter(|after| {
                    let after = after.clone();
                    async { !get_state(&self.state, after).await.is_active() }
                })
                .collect()
                .await,
        };

        // trigger deps
        println!("dep: adding {} to start list", id);
        // add requires && wants to start; add conflicts to stop
        println!("dep: requires and wants:");
        for unit_id in deps.requires.union(&deps.wants).cloned() {
            println!("dep:     {}", unit_id);
            self.dep.send(Message::AddToStart(unit_id)).await.unwrap();
        }
        for unit_id in deps.conflicts.iter().cloned() {
            self.dep.send(Message::AddToStop(unit_id)).await.unwrap();
        }

        // trigger action or add to wait list
        if wait_list.is_empty() {
            self.guard
                .send(guard::Message::DepsReady(id))
                .await
                .unwrap();
        } else {
            self.pending_jobs.insert(id, wait_list);
        }
    }

    async fn add_to_stop(&mut self, id: UnitId) {
        println!("adding {} to stop", id);
        let deps = self.dep_map.get(&id).unwrap();
        // simple job merge
        if let Entry::Occupied(o) = self.pending_jobs.entry(id.clone()) {
            match o.get() {
                // remove the previous start job here
                JobWaitInfo::Start { .. } => {
                    o.remove();
                }
                // since there's already waiting for its deps
                // dont need to insert another time
                JobWaitInfo::Stop { .. } => return,
            }
        } else if !is_guard_exists(&self.guard, id.clone()).await {
            handle_stopped(deps, &mut self.pending_jobs, &id, &self.guard).await;
            return;
        }

        // build wait_list: find what is blocking the stop operation
        let wait_list = JobWaitInfo::Stop {
            before: stream::iter(deps.before.iter().cloned())
                .filter(|before| {
                    let before = before.clone();
                    async { !get_state(&self.state, before).await.is_stopped() }
                })
                .collect()
                .await,
        };

        for unit_id in deps.required_by.iter().cloned() {
            self.dep.send(Message::AddToStop(unit_id)).await.unwrap();
        }

        // trigger or add to wait list
        if wait_list.is_empty() {
            self.guard.send(guard::Message::Stop(id)).await.unwrap();
        } else {
            // add id.required_by to stop
            self.pending_jobs.insert(id, wait_list);
        }
    }
}

async fn handle_active(
    full_dep: &FullDepInfo,
    pending_jobs: &mut HashMap<UnitId, JobWaitInfo>,
    state_change_id: UnitId,
    guard: &Sender<guard::Message>,
) {
    // rule 4: active, requires/wants after me can start
    for waiting_id in &full_dep.before & &(&full_dep.required_by | &full_dep.wanted_by) {
        if let Entry::Occupied(o) = pending_jobs.entry(waiting_id) {
            tick_start_waiting(o, &state_change_id, guard).await;
        }
    }
}

async fn handle_stopped(
    full_dep: &FullDepInfo,
    pending_jobs: &mut HashMap<UnitId, JobWaitInfo>,
    state_change_id: &UnitId,
    guard: &Sender<guard::Message>,
) {
    // rule 1: stopped, conflicts after me can start
    for waiting_id in &full_dep.before & &full_dep.conflicts {
        if let Entry::Occupied(o) = pending_jobs.entry(waiting_id) {
            tick_start_waiting(o, state_change_id, guard).await;
        }
    }
    // rule 2: stopped, requires before me can stop
    for waiting_id in &full_dep.after & &full_dep.requires {
        if let Entry::Occupied(o) = pending_jobs.entry(waiting_id) {
            tick_stop_waiting(o, state_change_id, guard).await;
        }
    }
}

async fn tick_stop_waiting(
    mut o: OccupiedEntry<'_, UnitId, JobWaitInfo>,
    state_change_id: &UnitId,
    guard: &Sender<guard::Message>,
) {
    if let JobWaitInfo::Stop { before } = o.get_mut() {
        before.remove(state_change_id);
    }
    if o.get().is_empty() {
        let (k, _) = o.remove_entry();
        guard.send(guard::Message::Stop(k)).await.unwrap()
    }
}

async fn tick_start_waiting(
    mut o: OccupiedEntry<'_, UnitId, JobWaitInfo>,
    state_change_id: &UnitId,
    guard: &Sender<guard::Message>,
) {
    if let JobWaitInfo::Start { after } = o.get_mut() {
        after.remove(state_change_id);
    }
    if o.get().is_empty() {
        let (k, _) = o.remove_entry();
        guard.send(guard::Message::DepsReady(k)).await.unwrap();
    }
}

fn dep_map_insert(
    unit0: &UnitId,
    src: &[UnitId],
    target: &mut HashMap<UnitId, FullDepInfo>,
    field: impl Fn(&mut FullDepInfo) -> &mut HashSet<UnitId>,
) {
    target
        .entry(unit0.clone())
        .or_default()
        .pipe(&field)
        .extend(src.iter().cloned());
}
fn dep_map_rinsert(
    unit0: &UnitId,
    src: &[UnitId],
    target: &mut HashMap<UnitId, FullDepInfo>,
    field: impl Fn(&mut FullDepInfo) -> &mut HashSet<UnitId>,
) {
    for unit in src.iter() {
        target
            .entry(unit.clone())
            .or_default()
            .pipe(&field)
            .insert(unit0.clone());
    }
}
