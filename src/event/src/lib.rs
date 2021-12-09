//pub mod events;
pub mod poll;
//pub mod sources;

#[allow(unused_macros)]
#[macro_export]
macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        let res = unsafe { libc::$fn($($arg, )*) };
        if res == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
}

#[cfg(test)]
mod test {
    #[cfg(unix)]
    #[test]
    fn build() {}
}
