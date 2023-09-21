//! Definitions for all possible errors used in this crate.
use snafu::Snafu;
use std::io;

// TODO: change errors to `log::warn`s to prevent one bad file from stalling the entire loading process
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("{} is not a valid directory.", path))]
    InvalidDirectoryError { path: String },

    #[snafu(display("Failed to read directory {}: {}.", path, source))]
    ReadDirectoryError { source: io::Error, path: String },

    #[snafu(display("Cannot load a template unit: {}.", name))]
    LoadTemplateError { name: String },

    #[snafu(display("{} is not a file.", path))]
    NotAFileError { path: String },

    #[snafu(display("Failed to read file {}: {}.", path, source))]
    ReadFileError { source: io::Error, path: String },

    #[snafu(display("Failed to read directory entry: {}.", source))]
    ReadEntryError { source: io::Error },

    #[snafu(display("Unable to read filename for {}.", path))]
    FilenameUnreadable { path: String },

    #[snafu(display("Invalid filename {}.", filename))]
    InvalidFilenameError { filename: String },

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

    #[snafu(display("Failed to find unit {}.", name))]
    NoUnitFoundError { name: String },

    #[snafu(display("Invalid specifier: {}", specifier))]
    InvalidSpecifierError { specifier: char },
}
