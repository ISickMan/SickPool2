use std::time::{SystemTime, UNIX_EPOCH};

pub fn time_now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}