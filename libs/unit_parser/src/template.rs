//! Functions and interfaces for systemd unit template related stuff.
use std::path::Path;

use crate::{config::Result, error::Error};

/// Describes the type of a unit file.
pub(crate) enum UnitType<'a> {
    Template(&'a str),         // template name
    Instance(&'a str, String), // instance name, template file name
    Regular(&'a str),          // unit name
}

/// Determines the type of a unit based on its filename.
pub(crate) fn unit_type(filename: &str) -> Result<UnitType> {
    /* Take foo@123.service for example, "foo@123" is its first_name; "foo" is prefix or
     * template; "service" is its last_name, suffix, or type; "123" is instance. */
    let (template, instance_suffix) = match filename.split_once('@') {
        None => return Ok(UnitType::Regular(filename)),
        Some(v) => (v.0, v.1),
    };
    if instance_suffix.starts_with('.') {
        return Ok(UnitType::Template(filename));
    }
    let (instance, suffix) = match instance_suffix.split_once('.') {
        None => {
            return Err(Error::InvalidFilenameError {
                path: Path::new(filename).to_path_buf(),
            })
        }
        Some(v) => (v.0, v.1),
    };
    Ok(UnitType::Instance(
        instance,
        format!("{}@.{}", template, suffix),
    ))
}
