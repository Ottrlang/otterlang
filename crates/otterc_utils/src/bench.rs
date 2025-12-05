use std::fmt;
use std::time::{Duration, Instant};

/// A simple benchmarking utility for measuring code performance
pub struct Benchmark {
    name: String,
    iterations: usize,
    samples: Vec<Duration>,
}

impl Benchmark {
    pub fn new(name: impl Into<String>, iterations: usize) -> Self {
        Self {
            name: name.into(),
            iterations,
            samples: Vec::with_capacity(iterations),
        }
    }

    /// Run a benchmark with the given function
    pub fn run<F, T>(&mut self, mut f: F) -> BenchmarkResult
    where
        F: FnMut() -> T,
    {
        self.samples.clear();

        // Warmup
        for _ in 0..std::cmp::min(self.iterations / 10, 10) {
            let _ = f();
        }

        // Actual measurements
        for _ in 0..self.iterations {
            let start = Instant::now();
            let _ = f();
            let duration = start.elapsed();
            self.samples.push(duration);
        }

        BenchmarkResult {
            name: self.name.clone(),
            iterations: self.iterations,
            samples: self.samples.clone(),
        }
    }
}

/// Result of a benchmark run
#[derive(Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: usize,
    pub samples: Vec<Duration>,
}

impl BenchmarkResult {
    pub fn mean(&self) -> Duration {
        let total: Duration = self.samples.iter().sum();
        total / self.iterations as u32
    }

    pub fn median(&self) -> Duration {
        let mut sorted = self.samples.clone();
        sorted.sort();
        sorted[sorted.len() / 2]
    }

    pub fn min(&self) -> Duration {
        *self.samples.iter().min().unwrap_or(&Duration::ZERO)
    }

    pub fn max(&self) -> Duration {
        *self.samples.iter().max().unwrap_or(&Duration::ZERO)
    }

    pub fn std_dev(&self) -> Duration {
        let mean = self.mean();
        let variance: f64 = self
            .samples
            .iter()
            .map(|&sample| {
                let diff = sample.as_secs_f64() - mean.as_secs_f64();
                diff * diff
            })
            .sum::<f64>()
            / self.iterations as f64;
        Duration::from_secs_f64(variance.sqrt())
    }

    pub fn throughput(&self, items: usize) -> f64 {
        items as f64 / self.mean().as_secs_f64()
    }
}

impl fmt::Display for BenchmarkResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Benchmark: {}", self.name)?;
        writeln!(f, "  Iterations: {}", self.iterations)?;
        writeln!(f, "  Mean:       {:?}", self.mean())?;
        writeln!(f, "  Median:     {:?}", self.median())?;
        writeln!(f, "  Min:        {:?}", self.min())?;
        writeln!(f, "  Max:        {:?}", self.max())?;
        writeln!(f, "  Std Dev:    {:?}", self.std_dev())?;
        Ok(())
    }
}

/// Compare two benchmark results
pub fn compare_benchmarks(baseline: &BenchmarkResult, current: &BenchmarkResult) -> f64 {
    let baseline_mean = baseline.mean().as_secs_f64();
    let current_mean = current.mean().as_secs_f64();
    ((current_mean - baseline_mean) / baseline_mean) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark() {
        let mut bench = Benchmark::new("test", 10);
        let result = bench.run(|| {
            std::thread::sleep(Duration::from_micros(10));
        });

        assert!(result.mean() >= Duration::from_micros(10));
    }
}
