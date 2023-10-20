//! Crate for parsing and loading systemd-style units.
//! This crate provides a serde-like macro system for defining structs to represent
//! units and other systemd config files, as well as implementations of loading named units.

mod config;
pub mod error;
mod escape;
mod parser;
mod specifiers;
mod template;

// Implementations of time related parsing.
mod datetime;
mod duration;
// Work in progress.
// pub mod calender_events;

/// All public interfaces for normal usage.
/// Use `use unit_parser::prelude::*;` to include.
pub mod prelude;

/// Internal interfaces, should only be used in macro generated code.
#[doc(hidden)]
pub mod internal;
