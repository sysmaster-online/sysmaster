use std::collections::HashMap;

use tokio::{
    sync::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};

use super::dep;
use crate::unit::{UnitId, UnitObj};

pub(crate) mod utils;

pub(crate) enum Message {
    /// for debug
    DbgPrint,
    /// update/insert static info of the unit
    Update(UnitId, UnitObj),
    /// remove the unit from store
    Remove(UnitId),
    /// get the static info of the unit
    Get(UnitId, oneshot::Sender<UnitObj>),
    /// start the unit
    Start(UnitId),
    /// stop the unit
    Stop(UnitId),
    /// restart the unit
    Restart(UnitId),
}

#[derive(Debug)]
pub(crate) struct UnitStore {
    map: HashMap<UnitId, UnitObj>, // info in unit files
    dep: Sender<dep::Message>,
}

impl UnitStore {
    pub(crate) fn new(dep: Sender<dep::Message>) -> Self {
        Self {
            map: HashMap::new(),
            dep,
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::DbgPrint => println!("{:#?}", self.map),
                    Message::Update(id, unit) => {
                        println!("updating unit: {:?}", &id);
                        if let Some(old) = self.map.insert(id.clone(), unit.clone()) {
                            todo!("feature: update dep_info");
                        } else {
                            self.dep
                                .send(dep::Message::Load(id, unit.deps()))
                                .await
                                .unwrap();
                        }
                    }
                    Message::Remove(id) => {
                        self.map.remove(&id);
                    }
                    Message::Get(id, sender) => {
                        if let Some(unitobj) = self.map.get(&id).cloned() {
                            sender.send(unitobj).ok();
                        }
                    }
                    // start the unit and its deps
                    Message::Start(id) => {
                        println!("starting unit: {:?}", &id);
                        self.dep.send(dep::Message::AddToStart(id)).await.unwrap()
                    }
                    Message::Stop(id) => {
                        println!("stopping unit: {:?}", &id);
                        self.dep.send(dep::Message::AddToStop(id)).await.unwrap()
                    }
                    Message::Restart(id) => todo!(),
                }
            }
        })
    }
}
