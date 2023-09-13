#![allow(non_snake_case, dead_code)]

use chrono::Duration;
use unit_parser::prelude::*;

#[derive(UnitConfig, Debug)]
#[unit(suffix = "unit")]
pub struct Unit {
    // sections can be attributed with default to fallback to default when not present
    // trait `Default` is required to be implemented
    // sections can also have alternative key name
    #[section(default, key = "AlternativeKey")]
    pub Section1: SimpleSection,

    #[section(must)]
    pub Section2: AdvancedSection,

    pub Section3: Option<OptionalSection>,
}

#[derive(UnitSection, Debug)]
pub struct SimpleSection {
    #[entry(must)]
    pub Field: String,
}

impl Default for SimpleSection {
    fn default() -> Self {
        Self {
            Field: "value".to_string(),
        }
    }
}

#[derive(UnitSection, Debug)]
pub struct AdvancedSection {
    /// a regular public config field
    #[entry(must)]
    pub Regular: String,

    /// a private config field
    #[entry(must)]
    Private: String,

    /// a config field with values within an enum
    #[entry(must)]
    Enum: MyEnum,

    /// a config field with custom key name
    #[entry(key = "AlternativeKey", must)]
    CustomNamed: String,

    /// a config field with default value
    #[entry(default = "default-value")]
    DefaultValued: String,

    /// a duration config field
    #[entry(must)]
    Duration: Duration,

    /// a field that can appear multiple times
    #[entry(multiple)]
    Multiple: Vec<i64>,

    /// an optional field
    Optional: Option<u64>,
}

#[derive(UnitSection, Debug)]
pub struct OptionalSection {}

#[derive(UnitEntry, Debug)]
enum MyEnum {
    Val1,
    Val2,
}

fn main() {
    let result = Unit::load_named(vec!["./libs/unit_parser/examples"], "example", false).unwrap();
    println!("{:#?}", result);
}
