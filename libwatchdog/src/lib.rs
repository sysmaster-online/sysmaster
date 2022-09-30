//! hardware watchdog
pub mod hardware;

/// register timer
pub fn register_timer() {}

/// set enable state
pub fn event_source_set_enabled(_enable: bool) {}
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
