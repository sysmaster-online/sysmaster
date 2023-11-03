use crate::{
    unit::UnitCommon,
    util::loader::{empty_str, str_to_unitids},
    Rc,
};

use super::{
    super::{UnitDeps, UnitImpl},
    Impl, Kind,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Service {
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
    pub(crate) kind: Kind,
    pub(crate) start: String,
    #[serde(default)]
    pub(crate) stop: String,
    #[serde(default)]
    pub(crate) restart: String,
}

impl From<Service> for UnitImpl<Impl> {
    fn from(value: Service) -> Self {
        let Service {
            name,
            requires,
            wants,
            before,
            after,
            conflicts,
            kind,
            start,
            stop,
            restart,
        } = value;

        Self {
            common: UnitCommon {
                name: name.into(),
                description: empty_str(),
                documentation: empty_str(),
                deps: Rc::new(UnitDeps::from_strs(
                    &requires, &wants, &before, &after, &conflicts,
                )),
            },
            sub: Impl {
                kind,
                exec_start: start.into(),
                exec_stop: stop.into(),
                exec_restart: restart.into(),
            },
        }
    }
}

impl UnitDeps {
    pub(crate) fn from_strs(
        requires: &str,
        wants: &str,
        before: &str,
        after: &str,
        conflicts: &str,
    ) -> Self {
        let requires = str_to_unitids(requires);
        let wants = str_to_unitids(wants);
        let before = str_to_unitids(before);
        let after = str_to_unitids(after);
        let conflicts = str_to_unitids(conflicts);
        Self {
            requires,
            wants,
            after,
            before,
            conflicts,
        }
    }
}

pub(crate) fn load_service(s: &str) -> UnitImpl<Impl> {
    toml::from_str::<Service>(s).unwrap().into()
}
