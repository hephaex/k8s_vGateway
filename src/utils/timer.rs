//! Timer utilities
//!
//! Provides timing and measurement helpers.

#![allow(dead_code)]

use std::time::{Duration, Instant};

/// Simple timer for measuring elapsed time
#[derive(Debug)]
pub struct Timer {
    start: Instant,
    label: String,
}

impl Timer {
    /// Create and start a new timer
    pub fn start(label: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            label: label.into(),
        }
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed().as_millis() as u64
    }

    /// Get elapsed time in seconds
    pub fn elapsed_secs(&self) -> f64 {
        self.elapsed().as_secs_f64()
    }

    /// Stop timer and return elapsed time
    pub fn stop(self) -> Duration {
        let elapsed = self.elapsed();
        tracing::debug!("{}: {}ms", self.label, elapsed.as_millis());
        elapsed
    }
}

/// Stopwatch with lap timing
#[derive(Debug)]
pub struct Stopwatch {
    start: Instant,
    laps: Vec<(String, Duration)>,
}

impl Stopwatch {
    /// Create a new stopwatch
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            laps: Vec::new(),
        }
    }

    /// Record a lap
    pub fn lap(&mut self, label: impl Into<String>) {
        let elapsed = self.start.elapsed();
        self.laps.push((label.into(), elapsed));
    }

    /// Get total elapsed time
    pub fn total(&self) -> Duration {
        self.start.elapsed()
    }

    /// Get all laps
    pub fn laps(&self) -> &[(String, Duration)] {
        &self.laps
    }

    /// Get lap times (duration of each lap, not cumulative)
    pub fn lap_times(&self) -> Vec<(String, Duration)> {
        let mut result = Vec::new();
        let mut prev = Duration::ZERO;

        for (label, cumulative) in &self.laps {
            let lap_time = *cumulative - prev;
            result.push((label.clone(), lap_time));
            prev = *cumulative;
        }

        result
    }

    /// Format laps as string
    pub fn format(&self) -> String {
        let mut output = String::new();
        for (label, duration) in self.lap_times() {
            output.push_str(&format!("{}: {}ms\n", label, duration.as_millis()));
        }
        output.push_str(&format!("Total: {}ms", self.total().as_millis()));
        output
    }
}

impl Default for Stopwatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_timer() {
        let timer = Timer::start("test");
        sleep(Duration::from_millis(10));
        let elapsed = timer.elapsed_ms();
        assert!(elapsed >= 10);
    }

    #[test]
    fn test_stopwatch() {
        let mut sw = Stopwatch::new();
        sleep(Duration::from_millis(10));
        sw.lap("first");
        sleep(Duration::from_millis(10));
        sw.lap("second");

        assert_eq!(sw.laps().len(), 2);

        let lap_times = sw.lap_times();
        assert_eq!(lap_times.len(), 2);
    }
}
