use std::time::{Duration, Instant};

/// Records named timing measurements for compiler phases.
#[derive(Default)]
pub struct Profiler {
    phases: Vec<PhaseTiming>,
}

impl Profiler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_phase<F, T>(&mut self, name: impl Into<String>, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let name = name.into();
        let start = Instant::now();
        let output = f();
        let duration = start.elapsed();
        self.phases.push(PhaseTiming { name, duration });
        output
    }

    pub fn push_phase(&mut self, name: impl Into<String>, duration: Duration) {
        self.phases.push(PhaseTiming {
            name: name.into(),
            duration,
        });
    }

    pub fn phases(&self) -> &[PhaseTiming] {
        &self.phases
    }
}

#[derive(Clone, Debug)]
pub struct PhaseTiming {
    pub name: String,
    pub duration: Duration,
}
