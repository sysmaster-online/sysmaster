use std::error::Error;
use sysmaster_ng::manager::{bus::Bus, MANAGER};
use zbus::ConnectionBuilder;

// Although we use `tokio` here, you can use any async runtime of choice.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // struct and bus initializes.
    let bus: Bus = Bus { count: 0 };
    let _connection = ConnectionBuilder::session()?
        .name("org.openEuler.sysMaster")?
        .serve_at("/org/openEuler/sysMaster", bus)?
        .build()
        .await?;

    let mut manager = MANAGER.lock().await;
    manager.load().await?;
    manager.start_loop().await;
    // let manager = MANAGER.lock().await;
    // manager.start_loop().await;
    Ok(())
}
