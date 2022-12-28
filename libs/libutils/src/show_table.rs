//! Struct used when printing some formatted output
use tabled::Tabled;

#[derive(Tabled)]
/// key-value pair used by sctl status
pub struct StatusItem {
    /// * key: keyword
    key: String,
    /// * value: current state
    value: String,
}

impl StatusItem {
    /// Create a new StatusItem
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }
}

#[derive(Tabled)]
/// item used by sctl list-units
pub struct ListUnitsItem {
    /// * unit name
    #[tabled(rename = "UNIT")]
    name: String,
    /// * the load state of one unit, i.e. loaded
    #[tabled(rename = "LOAD")]
    load_state: String,
    /// * the current running state of one unit, i.e. active, activating...
    #[tabled(rename = "ACTIVE")]
    active_state: String,
    /// * the sub state of one unit, i.e. waiting, plugged...
    #[tabled(rename = "SUB")]
    sub_state: String,
    /// * the description of one unit
    #[tabled(rename = "DESCRIPTION")]
    description: String,
}

impl ListUnitsItem {
    /// Create a new ListUnitsItem
    pub fn new(
        name: String,
        load_state: String,
        active_state: String,
        sub_state: String,
        description: String,
    ) -> Self {
        Self {
            name,
            load_state,
            active_state,
            sub_state,
            description,
        }
    }
}
