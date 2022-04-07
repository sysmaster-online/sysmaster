#![allow(unused_macros)]
#[macro_export]
macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        let res = unsafe { libc::$fn($($arg, )*) };
        if res < 0 {
            utils::Result::Err(utils::Error::Syscall { syscall: stringify!($fn), errno: unsafe { *libc::__errno_location() }, ret: res })
        } else {
            utils::Result::Ok(res)
        }
    }};
}
