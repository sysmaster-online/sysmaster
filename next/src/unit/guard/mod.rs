use std::collections::HashMap;

use futures::{future::BoxFuture, Future};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

use super::{
    state::{self, set_state, State},
    store, UnitEntry,
};

#[derive(Debug, Clone)]
pub(crate) struct GuardManager {
    map: HashMap<UnitEntry, Sender<GuardMessage>>,
    self_: Sender<Message>,
    store: Sender<store::Message>,
    state: Sender<state::Message>,
}

pub(crate) enum Message {
    Update(
        UnitEntry,
        Box<
            dyn FnOnce(
                    Sender<store::Message>,
                    Sender<state::Message>,
                    Receiver<GuardMessage>,
                ) -> BoxFuture<'static, State>
                + Send
                + 'static,
        >,
    ),
    Remove(UnitEntry),
    Stop(UnitEntry),
    Kill(UnitEntry),
}

impl GuardManager {
    pub(crate) fn new(
        self_: Sender<Message>,
        store: Sender<store::Message>,
        state: Sender<state::Message>,
    ) -> Self {
        Self {
            map: HashMap::new(),
            self_,
            store,
            state,
        }
    }

    pub(crate) fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    Message::Update(u, s) => {
                        let (sender, receiver) = mpsc::channel(4);
                        let f = s(self.store.clone(), self.state.clone(), receiver);
                        self.map.insert(u.clone(), sender);
                        let sender = self.self_.clone();
                        let state = self.state.clone();
                        tokio::spawn(async move {
                            let new_state = f.await;
                            // remove the entry after the guard end
                            set_state(&state, u.clone(), new_state).await;
                            sender.send(Message::Remove(u)).await.unwrap();
                        });
                    }
                    Message::Remove(u) => {
                        self.map.remove(&u);
                    }
                    Message::Stop(u) => {
                        self.map
                            .get(&u)
                            .unwrap()
                            .send(GuardMessage::Stop)
                            .await
                            .unwrap();
                    }
                    Message::Kill(u) => self
                        .map
                        .get(&u)
                        .unwrap()
                        .send(GuardMessage::Kill)
                        .await
                        .unwrap(),
                }
            }
        })
    }
}

pub(crate) enum GuardMessage {
    RequiresReady,
    AftersReady,
    Stop,
    Kill,
}

pub(crate) async fn create_guard<F, Fut>(guard_manager: &Sender<Message>, u: UnitEntry, f: F)
where
    F: FnOnce(Sender<store::Message>, Sender<state::Message>, Receiver<GuardMessage>) -> Fut
        + Send
        + 'static,
    Fut: Future<Output = State> + Send + 'static,
{
    guard_manager
        .send(Message::Update(
            u,
            Box::new(|store, state, rx| Box::pin(f(store, state, rx))),
        ))
        .await
        .unwrap();
}

pub(crate) async fn guard_stop(guard_manager: &Sender<Message>, u: UnitEntry) {
    guard_manager.send(Message::Stop(u)).await.unwrap()
}

pub(crate) async fn guard_kill(guard_manager: &Sender<Message>, u: UnitEntry) {
    guard_manager.send(Message::Stop(u)).await.unwrap()
}
