


#[allow(missing_docs)]
#[macro_export]
macro_rules! null_str {
    ($name:expr) => {
        String::from($name)
    };
}

pub use unit::execute;
pub use unit::UmIf;
pub mod unit;

pub mod reliability;