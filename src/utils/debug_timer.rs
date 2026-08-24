use std::time::{Duration, Instant};

pub struct DebugTimer {
    _instant: Instant,
    _duration: Duration,
}

impl DebugTimer {
    pub fn new() -> Self {
        Self { 
            _instant: Instant::now(),
            _duration: Duration::ZERO,
        }
    }

    pub fn lap(&mut self) -> Duration {
        let old = self._duration;
        self._duration = self._instant.elapsed();
        self._duration.abs_diff(old)
    }
}