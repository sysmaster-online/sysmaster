#[macro_use]
extern crate strum;

pub use crate::service::ServiceUnit;

mod exec_child;
pub mod service;
mod service_base;
mod service_parse;
mod service_start;

#[cfg(test)]
mod tests;
