//! Definitions for all possible errors used in this crate.
use snafu::Snafu;
use std::{io, path::PathBuf};

// TODO: change errors to `log::warn`s to prevent one bad file from stalling the entire loading process
/// Errors used in crate.
#[derive(Debug, Snafu)]
#[allow(missing_docs)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("{} is not a valid directory.", path.display()))]
    InvalidDirectoryError { path: PathBuf },

    #[snafu(display("Failed to read directory {}: {}.", path.display(), source))]
    ReadDirectoryError { source: io::Error, path: PathBuf },

    #[snafu(display("Cannot load a template unit: {}.", path.display()))]
    LoadTemplateError { path: PathBuf },

    #[snafu(display("{} is not a file.", path.display()))]
    NotAFileError { path: PathBuf },

    #[snafu(display("Failed to read file {}: {}.", path.display(), source))]
    ReadFileError { source: io::Error, path: PathBuf },

    #[snafu(display("Failed to read directory entry: {}.", source))]
    ReadEntryError { source: io::Error },

    #[snafu(display("Unable to read filename for {}.", path.display()))]
    FilenameUnreadable { path: PathBuf },

    #[snafu(display("Invalid filename {}.", path.display()))]
    InvalidFilenameError { path: PathBuf },

    #[snafu(display("Unit file should provide at least one section."))]
    NoSectionError,

    #[snafu(display("Failed to parse section {}.", key))]
    SectionParsingError { key: String },

    #[snafu(display("Missing entry with key {}, which is required.", key))]
    EntryMissingError { key: String },

    #[snafu(display("Missing section with key {}, which is required.", key))]
    SectionMissingError { key: String },

    #[snafu(display("Failed to parse {} as the value of entry with key {}.", value, key))]
    ValueParsingError { key: String, value: String },

    #[snafu(display("Failed to find unit {}.", path.display()))]
    NoUnitFoundError { path: PathBuf },

    #[snafu(display("Invalid specifier: {}", specifier))]
    InvalidSpecifierError { specifier: char },
}
