use async_trait::async_trait;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::hash::Hash;
use std::path::Path;
use std::str::FromStr;
use tokio::sync::Mutex;

mod service;
mod socket;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitType {
    Service,
    Socket,
}

impl FromStr for UnitType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "service" => Ok(UnitType::Service),
            "ssocket" => Ok(UnitType::Socket),
            _ => Err(()),
        }
    }
}

pub enum _State {
    Started,
    Stopped,
    Loaded,
}

pub type Units = Vec<Unit>;
pub static UNITS: Lazy<Mutex<HashMap<&str, &mut Unit>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub struct Unit {
    id: String,
    inner: Box<dyn UnitTrait + Send + Sync>,
}

impl PartialEq for Unit {
    fn eq(&self, other: &Unit) -> bool {
        let se: u64 = unsafe { std::mem::transmute(self) };
        let ot: u64 = unsafe { std::mem::transmute(other) };
        se == ot
    }
}

impl Eq for Unit {}

impl Unit {
    pub fn new(name: &str) -> Self {
        if let Some(ext) = Path::new(name).extension() {
            let ext = ext.to_str().unwrap();

            Self {
                id: name.to_string(),
                inner: match ext {
                    "socket" => Box::new(socket::Socket::new()),
                    _ => Box::new(service::Service::new()),
                },
            }
        } else {
            log::warn!("No extension found, use default service");
            Self {
                id: name.to_string(),
                inner: Box::new(service::Service::new()),
            }
        }
    }
}

impl Unit {
    pub fn start(&self) -> bool {
        self.inner.start();
        println!("started {}", self.id);
        true
    }

    fn _stop(&self) -> bool {
        self.inner.stop();
        true
    }

    fn _load(&self) -> bool {
        true
    }
}

#[async_trait]
pub trait UnitTrait {
    fn start(&self) -> bool;
    fn stop(&self) -> bool;
    fn load(&self) -> bool;
    fn kind(&self) -> UnitType;
}

impl PartialEq for dyn UnitTrait {
    fn eq(&self, other: &dyn UnitTrait) -> bool {
        self.kind() == other.kind()
    }
}

impl Eq for dyn UnitTrait {}
