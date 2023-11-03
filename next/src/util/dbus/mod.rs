use std::time::Duration;

use tokio::{sync::mpsc::Sender, time::sleep};
use zbus::{dbus_interface, Connection, ConnectionBuilder};

use crate::{
    actor::{
        state::{self, print_state},
        unit::{
            self,
            utils::{print_store, start_unit, stop_unit},
        },
    },
    unit::{State, UnitId},
};

#[derive(Debug)]
pub(crate) struct DbusServer {
    store: Sender<unit::Message>,
    state: Sender<state::Message>,
}

impl DbusServer {
    pub(crate) fn new(store: Sender<unit::Message>, state: Sender<state::Message>) -> Self {
        Self { store, state }
    }
}
#[dbus_interface(name = "org.sysrs.sysrs1")]
impl DbusServer {
    fn echo(&self, msg: &str) -> String {
        println!("dbus: called echo with `{msg}`");
        msg.to_owned()
    }
    async fn start_unit(&self, unit: &str) {
        let id = UnitId::from(unit);
        start_unit(&self.store, id.clone()).await;
        // todo: really wait unit change to starting and then get the result
        sleep(Duration::from_millis(10)).await;
    }

    async fn stop_unit(&self, unit: &str) {
        let id = UnitId::from(unit);
        stop_unit(&self.store, id.clone()).await;
        // todo: really wait unit change to stop and then get the result
        sleep(Duration::from_millis(10)).await;
    }

    async fn print_store(&self) {
        print_store(&self.store).await
    }
    async fn print_state(&self) {
        print_state(&self.state).await
    }

    fn get_unit(&self, unit: &str) {
        todo!()
    }
}

pub(crate) async fn connect_dbus(server: DbusServer) -> zbus::Result<Connection> {
    ConnectionBuilder::system()
        .unwrap()
        .name("org.sysrs.sysrs1")?
        .serve_at("/org/sysrs/sysrs1", server)?
        .build()
        .await
}
