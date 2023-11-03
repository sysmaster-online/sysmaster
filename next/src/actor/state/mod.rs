use std::collections::HashMap;

use tokio::{
    sync::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};

use super::dep;
use crate::unit::{State, UnitId};

type MonitorRet = oneshot::Sender<Result<State, State>>;

pub(crate) enum Message {
    /// for debug
    DbgPrint,
    /// get state of the unit
    Get(UnitId, oneshot::Sender<State>),
    /// set state of the unit
    Set(UnitId, State),
    /// set state of the unit due to current state
    /// CAS
    SetWithCondition {
        id: UnitId,
        new_state: State,
        condition: Box<dyn FnOnce(State) -> bool + Send + 'static>,
    },
}

#[derive(Debug)]
pub(crate) struct StateStore {
    state: HashMap<UnitId, State>,
    dep: Sender<dep::Message>,
}

impl StateStore {
    pub(crate) fn new(dep: Sender<dep::Message>) -> Self {
        Self {
            state: Default::default(),
            dep,
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::DbgPrint => println!("{:#?}", self.state),
                    Message::Get(id, s) => {
                        if let Some(&state) = self.state.get(&id) {
                            s.send(state).ok();
                        } else {
                            s.send(State::Uninit).ok();
                        }
                    }
                    Message::Set(id, new_state) => self.set(id, new_state).await,
                    Message::SetWithCondition {
                        id,
                        new_state,
                        condition,
                    } => {
                        let old_state = self.state.get(&id).unwrap_or(&State::Uninit);
                        if condition(*old_state) {
                            self.set(id, new_state).await;
                        }
                    }
                }
            }
        })
    }

    /// use this to set state
    /// in order to send notifications
    async fn set(&mut self, id: UnitId, state: State) {
        println!("setting state: `{}` to `{}`", id, state);
        self.state.insert(id.clone(), state);
        self.dep
            .send(dep::Message::StateChange(id, state))
            .await
            .unwrap()
    }
}

pub(crate) async fn get_state(state_manager: &Sender<Message>, id: UnitId) -> State {
    let (s, r) = oneshot::channel();
    state_manager.send(Message::Get(id, s)).await.unwrap();
    r.await.unwrap()
}

pub(crate) async fn set_state(state_manager: &Sender<Message>, id: UnitId, state: State) {
    state_manager.send(Message::Set(id, state)).await.unwrap();
}

/// check the current state. if fit the condition, set the state to target.
/// return the previous state.
pub(crate) async fn set_state_with_condition(
    state_manager: &Sender<Message>,
    id: UnitId,
    new_state: State,
    condition: impl FnOnce(State) -> bool + Send + 'static,
) -> Result<State, State> {
    // hook: add oneshot in condition closure
    // and get the previous state
    let (s, r) = oneshot::channel();
    state_manager
        .send(Message::SetWithCondition {
            id,
            new_state,
            condition: Box::new(|state| {
                let ret = condition(state);
                let state = if ret { Ok(state) } else { Err(state) };
                s.send(state).unwrap();
                ret
            }),
        })
        .await
        .unwrap();
    r.await.unwrap()
}

pub(crate) async fn print_state(state_manager: &Sender<Message>) {
    state_manager.send(Message::DbgPrint).await.unwrap();
}
