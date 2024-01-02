// use std::error::Error;
use zbus::dbus_interface;

use crate::units::{self, UNITS};

pub struct Bus {
    pub count: u64,
}

#[dbus_interface(name = "org.openEuler.sysMaster1")]
impl Bus {
    async fn start(&self, name: &str) {
        //
        println!("start {:?}", name);
        let mut units = UNITS.lock().await;
        let n = Box::leak(Box::new(name.to_string()));
        let u = Box::leak(Box::new(units::Unit::new(name)));
        units.insert(n, u);
        println!("{:?}", units.contains_key(name));
        if let Some(unit) = units.get(name) {
            unit.start();
            units.remove(name);
            if units.is_empty() {
                println!("units is empty");
            }
        }

        // manager.units().await.lock().await.contains(name);
    }

    fn stop(&self) {
        println!("stop");
    }

    fn reload(&self) {
        println!("reload");
    }

    fn restart(&self) {
        println!("restart");
    }

    fn daemon_reload(&self) {
        println!("daemon_reload");
    }
}
