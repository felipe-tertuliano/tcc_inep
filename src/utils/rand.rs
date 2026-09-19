use std::time::{SystemTime, UNIX_EPOCH};

pub fn rand(limit: u32) -> u32 {
    if limit > 0 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        nanos % limit
    } else {
        0
    }
}
