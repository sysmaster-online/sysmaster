use crate::null_str;

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum UnitType {
    UnitService = 0,
    UnitTarget,
    UnitTypeMax,
    UnitTypeInvalid,
    UnitTypeErrnoMax,
}

#[derive(Hash, PartialEq, Eq, Debug, Copy, Clone)]
pub enum UnitRelations {
    UnitRequires,
    UnitWants,
    UnitBindsTo,

    UnitRequiresBy,
    UnitWantsBy,
    UnitBoundBy,

    UnitBefore,
    UnitAfter,

    UnitTriggers,
    UnitTriggeredBy,
}

pub enum UnitConfigItem {
    UcItemName(String),
    UcItemDesc(String),
    UcItemDoc(String),
}

#[derive(Debug)]
pub struct UnitConfig {
    pub name: String,
    pub deps: Vec<(UnitRelations, String)>,
    pub desc: String,
    pub documnetation: String,
}

impl UnitConfig {
    pub fn new() -> UnitConfig {
        UnitConfig {
            name: String::from(""),
            deps: Vec::new(),
            desc: String::from(""),
            documnetation: null_str!(""),
        }
    }
}
