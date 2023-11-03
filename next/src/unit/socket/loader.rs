use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    unit::{UnitCommon, UnitImpl},
    util::loader::{empty_dep, empty_str},
};

use super::Impl;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Socket {
    name: String,
    path: PathBuf,
    service: String,
}

impl From<Socket> for UnitImpl<Impl> {
    fn from(value: Socket) -> Self {
        Self {
            common: UnitCommon {
                name: value.name.into(),
                description: empty_str(),
                documentation: empty_str(),
                deps: empty_dep(),
            },
            sub: Impl {
                path: value.path.into(),
                service: value.service.as_str().into(),
            },
        }
    }
}

pub(crate) fn load_socket(s: &str) -> UnitImpl<Impl> {
    toml::from_str::<Socket>(s).unwrap().into()
}
