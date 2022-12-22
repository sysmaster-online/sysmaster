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
