//! innner lib of sysmaster
//! libsysmaster
/// null_str macro
#[macro_export]
macro_rules! null_str {
    ($name:expr) => {
        String::from($name)
    };
}

pub mod exec;
pub use unit::UmIf;
pub mod error;
pub mod rel;
pub mod serialize;
pub mod unit;
pub mod utils;
