use std::time::{Duration, Instant};

/// Simple stopwatch helper for wall-clock measurements.
pub struct Stopwatch {
    start: Instant,
}

impl Stopwatch {
    pub fn start_new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}
