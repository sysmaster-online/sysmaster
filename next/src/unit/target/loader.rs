use serde::{Deserialize, Serialize};

use crate::{
    unit::{UnitCommon, UnitDeps, UnitImpl},
    util::loader::empty_str,
};

use super::Impl;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Target {
    pub(crate) name: String,
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
}

impl From<Target> for UnitImpl<Impl> {
    fn from(value: Target) -> Self {
        Self {
            common: UnitCommon {
                name: value.name.into(),
                description: empty_str(),
                documentation: empty_str(),
                deps: UnitDeps::from_strs(
                    &value.requires,
                    &value.wants,
                    &value.before,
                    &value.after,
                    &value.conflicts,
                )
                .into(),
            },
            sub: Impl {},
        }
    }
}

pub(crate) fn load_target(s: &str) -> UnitImpl<Impl> {
    // dbg!(&t);
    toml::from_str::<Target>(s).unwrap().into()
    // dbg!(t)
}
