#![allow(non_snake_case, dead_code)]

use unit_parser::prelude::*;

#[derive(UnitConfig, Debug)]
#[unit(suffix = "service")]
struct Unit {
    #[section(must)]
    Section: Section,
}

#[derive(UnitSection, Debug)]
struct Section {
    #[entry(must)]
    Field1: u32,

    #[entry(must)]
    Field2: u32,

    #[entry(must)]
    Field3: u32,

    #[entry(must)]
    Field4: u32,
}

fn main() {
    let result = Unit::load_named(
        vec!["libs/unit_parser/examples/dropins"],
        "foo-bar-baz",
        false,
    )
    .unwrap();

    println!("result: {:#?}", result);
}
