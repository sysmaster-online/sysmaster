use std::collections::{HashMap, VecDeque};

use futures::{Stream, StreamExt};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use super::{
    guard,
    state::{self, get_state},
    Unit, UnitEntry,
};
use crate::Rc;

type UnitObj = Rc<dyn Unit + Send + Sync + 'static>;

#[derive(Debug)]
pub(crate) struct UnitStore {
    map: HashMap<UnitEntry, UnitObj>, // info in unit files
    state_manager: Sender<state::Message>,
    guard_manager: Sender<guard::Message>,
}

pub(crate) enum Message {
    DbgPrint,
    Update(UnitEntry, UnitObj),
    Remove(UnitEntry),
    Start(UnitEntry),
    Stop(UnitEntry),
    Restart(UnitEntry),
}

impl UnitStore {
    pub(crate) fn new(
        state_manager: Sender<state::Message>,
        guard_manager: Sender<guard::Message>,
    ) -> Self {
        Self {
            map: HashMap::new(),
            state_manager,
            guard_manager,
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::DbgPrint => println!("{:#?}", self.map),
                    Message::Update(entry, unit) => {
                        println!("updating unit: {:?}", &entry);
                        self.map.insert(entry, unit);
                    }
                    Message::Remove(entry) => {
                        self.map.remove(&entry);
                    }
                    Message::Start(entry) => {
                        println!("starting unit: {:?}", &entry);
                        if let Some(unit) = self.map.get(&entry) {
                            // find deps
                            let mut wants = self.find_wants(&unit).await;
                            while let Some(unit) = wants.pop() {
                                unit.start(self.state_manager.clone(), self.guard_manager.clone())
                                    .await;
                            }
                            let mut requires = self.find_requires(&unit).await;
                            while let Some(unit) = requires.pop() {
                                unit.start(self.state_manager.clone(), self.guard_manager.clone())
                                    .await;
                            }
                            unit.start(self.state_manager.clone(), self.guard_manager.clone())
                                .await;
                        }
                    }
                    Message::Stop(entry) => {
                        println!("stopping unit: {:?}", &entry);
                        if let Some(unit) = self.map.get(&entry) {
                            unit.stop(self.state_manager.clone(), self.guard_manager.clone())
                                .await;
                        }
                    }
                    Message::Restart(entry) => todo!(),
                }
            }
        })
    }

    async fn find_requires(&self, unit: &UnitObj) -> Vec<UnitObj> {
        let mut queue = VecDeque::new();
        queue.extend(unit.deps().requires.iter().cloned());
        let mut stack = Vec::new();
        while let Some(e) = queue.pop_front() {
            if get_state(&self.state_manager, e.clone())
                .await
                .is_inactive()
            {
                println!("finding requires...");
                if let Some(unit) = self.map.get(&e) {
                    let unit = unit.clone();
                    let deps = unit.deps();
                    for dep in deps.requires.iter().cloned() {
                        println!("pushing {:?} into queue", &dep);
                        queue.push_back(dep);
                    }
                    if stack
                        .iter()
                        .all(|u_in_stack| !Rc::ptr_eq(&unit, u_in_stack))
                    {
                        stack.push(unit);
                    }
                } else {
                    todo!("handle missing unit dep")
                }
            }
        }
        stack
    }

    async fn find_wants(&self, unit: &UnitObj) -> Vec<UnitObj> {
        let mut queue = VecDeque::new();
        queue.extend(unit.deps().wants.iter().cloned());
        let mut stack = Vec::new();
        while let Some(e) = queue.pop_front() {
            if get_state(&self.state_manager, e.clone())
                .await
                .is_inactive()
            {
                println!("finding wants...");
                if let Some(unit) = self.map.get(&e) {
                    let unit = unit.clone();
                    let deps = unit.deps();
                    for dep in deps.wants.iter().cloned() {
                        println!("pushing {:?} into queue", &dep);
                        queue.push_back(dep);
                    }
                    if stack
                        .iter()
                        .all(|u_in_stack| !Rc::ptr_eq(&unit, u_in_stack))
                    {
                        stack.push(unit);
                    }
                } else {
                    todo!("handle missing unit dep, missing: {e}")
                }
            }
        }
        stack
    }
}

pub(crate) async fn update_unit(store: &Sender<Message>, unit: impl Unit + Send + Sync + 'static) {
    let entry = UnitEntry::from(&unit);
    store
        .send(Message::Update(entry, Rc::new(unit)))
        .await
        .unwrap();
}

pub(crate) async fn update_units(store: &Sender<Message>, units: impl Stream<Item = UnitObj>) {
    units
        .for_each_concurrent(None, |unit| async move {
            let entry = UnitEntry::from(unit.as_ref());
            store.send(Message::Update(entry, unit)).await.unwrap()
        })
        .await
}

pub(crate) async fn start_unit(store: &Sender<Message>, entry: UnitEntry) {
    store.send(Message::Start(entry)).await.unwrap();
}

pub(crate) async fn stop_unit(store: &Sender<Message>, entry: UnitEntry) {
    store.send(Message::Stop(entry)).await.unwrap();
}

pub(crate) async fn print_store(store: &Sender<Message>) {
    store.send(Message::DbgPrint).await.unwrap()
}
