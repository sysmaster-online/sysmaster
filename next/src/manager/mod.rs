pub mod bus;
pub mod config;

use self::config::ManagerConfig;
use once_cell::sync::Lazy;
use std::{future::pending, io::Error};
use tokio::sync::Mutex;
use unit_parser::prelude::UnitConfig;

pub struct Manager {
    config: ManagerConfig,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            config: ManagerConfig::default(),
        }
    }

    pub const SYSTEM_CONFIG_FILE: &str = "/etc/sysmaster/system.conf";
    pub async fn load(&mut self) -> Result<(), Error> {
        let paths = vec![Manager::SYSTEM_CONFIG_FILE];
        if let Ok(cfg) = ManagerConfig::load_config(paths, "system.conf") {
            println!("load config ok {:?}", cfg);

            self.config = cfg;
        };
        println!("load config ok {:?}", self.config);
        Ok(())
    }

    pub async fn start_loop(&self) {
        pending::<()>().await;
    }
}

unsafe impl Sync for Manager {}
unsafe impl Send for Manager {}

pub static MANAGER: Lazy<Mutex<Manager>> = Lazy::new(manager_init);

fn manager_init() -> Mutex<Manager> {
    Mutex::new(Manager::new())
}
