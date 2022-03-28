use crate::manager::data::*;
use crate::null_str;

#[derive(Debug)]
pub struct UeConfig {
    name: String,
    desc: String,
    documnetation: String,
}

impl UeConfig {
    pub fn new() -> UeConfig {
        UeConfig {
            name: String::from(""),
            desc: String::from(""),
            documnetation: null_str!(""),
        }
    }

    pub fn set(&mut self, item: UnitConfigItem) {
        match item {
            UnitConfigItem::UcItemName(name) => self.name = name,
            UnitConfigItem::UcItemDesc(desc) => self.desc = desc,
            UnitConfigItem::UcItemDoc(doc) => self.documnetation = doc,
        }
    }

    pub fn get(&self, item: UnitConfigItem) -> UnitConfigItem {
        match item {
            UnitConfigItem::UcItemName(_) => UnitConfigItem::UcItemName(self.name.clone()),
            UnitConfigItem::UcItemDesc(_) => UnitConfigItem::UcItemDesc(self.desc.clone()),
            UnitConfigItem::UcItemDoc(_) => UnitConfigItem::UcItemDoc(self.documnetation.clone()),
        }
    }
}
