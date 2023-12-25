use zbus::{dbus_proxy, Connection, Result};

#[dbus_proxy(
    interface = "org.openEuler.sysMaster1",
    default_service = "org.openEuler.sysMaster",
    default_path = "/org/openEuler/sysMaster"
)]
trait sysMaster {
    async fn start(&self, name: &str) -> Result<()>;
    async fn stop(&self, name: &str) -> Result<()>;
}

// Although we use `async-std` here, you can use any async runtime of choice.
#[tokio::main]
async fn main() -> Result<()> {
    let connection = Connection::session().await?;

    let proxy = sysMasterProxy::new(&connection).await?;
    let reply = proxy.start("Maria").await?;
    println!("{:?}", reply);

    Ok(())
}
