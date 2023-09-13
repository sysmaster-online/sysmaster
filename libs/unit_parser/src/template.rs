//! Functions and interfaces for systemd unit template related stuff.
use crate::{config::Result, error::Error};

/// Describes the type of a unit file.
pub(crate) enum UnitType<'a> {
    Template(&'a str),         // template name
    Instance(&'a str, String), // instance name, template file name
    Regular(&'a str),          // unit name
}

/// Determines the type of a unit based on its filename.
pub(crate) fn unit_type<'a>(filename: &'a str) -> Result<UnitType<'a>> {
    let split: Vec<&str> = filename.split("@").collect();
    match split.len() {
        1 => Ok(UnitType::Regular(filename)),
        2 => {
            if split.get(1).unwrap().starts_with('.') {
                Ok(UnitType::Template(split.get(0).unwrap()))
            } else {
                let mut sub_split = split.get(1).unwrap().split('.');
                let template_name = sub_split.nth(0).unwrap();
                let suffix = sub_split.last().unwrap();
                Ok(UnitType::Instance(
                    split.get(0).unwrap(),
                    format!("{}@.{}", template_name, suffix),
                ))
            }
        }
        _ => Err(Error::InvalidFilenameError {
            filename: filename.to_string(),
        }),
    }
}
