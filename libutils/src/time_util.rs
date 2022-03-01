
use std::time::SystemTime;

const USEC_INFINITY: u128 = u128::MAX;


pub fn timespec_load(systime: SystemTime) -> u128 {
    match systime.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => {
            return d.as_micros();
        },
        Err(_) => {
            return USEC_INFINITY;
        }
    }
}