pub mod bus;
pub mod config;

use self::config::Config;
use crate::{jobs::Jobs, units::Units};
use once_cell::sync::Lazy;
use std::{future::pending, io::Error, sync::Arc};
use tokio::sync::Mutex;

pub struct Manager {
    config: Config,
    units: Arc<Mutex<Units>>,
    jobs: Arc<Mutex<Jobs>>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            config: Config::new(),
            units: Arc::new(Mutex::new(Vec::new())),
            jobs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn units(&self) -> Arc<Mutex<Units>> {
        self.units.clone()
    }

    pub async fn jobs(&self) -> Arc<Mutex<Jobs>> {
        self.jobs.clone()
    }
    pub async fn load(&mut self) -> Result<(), Error> {
        self.config.load().await?;
        println!("load config ok");
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
