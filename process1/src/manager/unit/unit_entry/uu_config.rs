use crate::manager::data::UnitConfigItem;
use crate::null_str;

#[derive(Debug)]
pub(super) struct UeConfig {
    name: String,
    desc: String,
    documnetation: String,
}

impl UeConfig {
    pub(super) fn new() -> UeConfig {
        UeConfig {
            name: String::from(""),
            desc: String::from(""),
            documnetation: null_str!(""),
        }
    }

    pub(super) fn set(&mut self, item: UnitConfigItem) {
        match item {
            UnitConfigItem::UcItemName(name) => self.name = name,
            UnitConfigItem::UcItemDesc(desc) => self.desc = desc,
            UnitConfigItem::UcItemDoc(doc) => self.documnetation = doc,
        }
    }

    pub(super) fn get(&self, item: UnitConfigItem) -> UnitConfigItem {
        match item {
            UnitConfigItem::UcItemName(_) => UnitConfigItem::UcItemName(self.name.clone()),
            UnitConfigItem::UcItemDesc(_) => UnitConfigItem::UcItemDesc(self.desc.clone()),
            UnitConfigItem::UcItemDoc(_) => UnitConfigItem::UcItemDoc(self.documnetation.clone()),
        }
    }
}
