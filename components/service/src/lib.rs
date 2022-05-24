#[macro_use]
extern crate strum;

pub use crate::service::ServiceUnit;

pub mod service;
mod service_base;
mod service_parse;

#[cfg(test)]
mod tests;
