use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    fstab::MountInfo,
    unit::{UnitCommon, UnitImpl},
    util::loader::{empty_dep, empty_str},
    Rc,
};

use super::Impl;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Mount {
    #[serde(default)]
    pub(crate) requires: String,
    #[serde(default)]
    pub(crate) wants: String,
    #[serde(default)]
    pub(crate) before: String,
    #[serde(default)]
    pub(crate) after: String,
    #[serde(default)]
    pub(crate) conflicts: String,

    source: String,
    mount_point: PathBuf,
    fs_type: String,
    #[serde(default)]
    mount_options: String,
}

impl From<Mount> for UnitImpl<Impl> {
    fn from(value: Mount) -> Self {
        Self {
            common: UnitCommon {
                name: value.source.into(),
                description: empty_str(),
                documentation: empty_str(),
                deps: empty_dep(),
            },
            sub: Rc::new(MountInfo {
                fs_spec: value.source.into(),
                mount_point: value.mount_point.into(),
                vfs_type: value.fs_type.into(),
                mount_options: value.mount_options.into(),
            }),
        }
    }
}

pub(crate) fn load_socket(s: &str) -> UnitImpl<Impl> {
    toml::from_str::<Mount>(s).unwrap().into()
}
