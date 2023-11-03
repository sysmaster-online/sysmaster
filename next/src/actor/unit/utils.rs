use futures::{Stream, StreamExt};
use tokio::sync::{mpsc::Sender, oneshot};

use super::{Message, UnitObj};
use crate::{
    unit::{Unit, UnitId},
    Rc,
};

pub(crate) async fn update_unit(store: &Sender<Message>, unit: impl Unit + Send + Sync + 'static) {
    let id = UnitId::from(&unit);
    store
        .send(Message::Update(id, Rc::new(unit)))
        .await
        .unwrap();
}

pub(crate) async fn update_units(store: &Sender<Message>, units: impl Stream<Item = UnitObj>) {
    units
        .for_each_concurrent(None, |unit| async move {
            let id = UnitId::from(unit.as_ref());
            store.send(Message::Update(id, unit)).await.unwrap()
        })
        .await
}

pub(crate) async fn get_unit(store: &Sender<Message>, unit_id: UnitId) -> Option<UnitObj> {
    let (s, r) = oneshot::channel();
    store.send(Message::Get(unit_id, s)).await.unwrap();
    r.await.ok()
}

pub(crate) async fn start_unit(store: &Sender<Message>, id: UnitId) {
    store.send(Message::Start(id)).await.unwrap();
}

pub(crate) async fn stop_unit(store: &Sender<Message>, id: UnitId) {
    store.send(Message::Stop(id)).await.unwrap();
}

pub(crate) async fn print_store(store: &Sender<Message>) {
    store.send(Message::DbgPrint).await.unwrap()
}
