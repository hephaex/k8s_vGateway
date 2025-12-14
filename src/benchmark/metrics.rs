//! Performance metrics collection and analysis
//!
//! Provides latency percentiles, throughput calculation, and statistical analysis.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Latency percentiles (p50, p90, p95, p99, p999)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Percentiles {
    /// 50th percentile (median)
    pub p50: f64,
    /// 90th percentile
    pub p90: f64,
    /// 95th percentile
    pub p95: f64,
    /// 99th percentile
    pub p99: f64,
    /// 99.9th percentile
    pub p999: f64,
}

impl Percentiles {
    /// Calculate percentiles from sorted latencies (in milliseconds)
    pub fn from_sorted(latencies: &[f64]) -> Self {
        if latencies.is_empty() {
            return Self::default();
        }

        Self {
            p50: percentile(latencies, 50.0),
            p90: percentile(latencies, 90.0),
            p95: percentile(latencies, 95.0),
            p99: percentile(latencies, 99.0),
            p999: percentile(latencies, 99.9),
        }
    }

    /// Format as table row
    pub fn format_row(&self) -> String {
        format!(
            "{:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2}",
            self.p50, self.p90, self.p95, self.p99, self.p999
        )
    }
}

/// Calculate percentile value from sorted array
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }

    let idx = (p / 100.0) * (sorted.len() - 1) as f64;
    let lower = idx.floor() as usize;
    let upper = idx.ceil() as usize;
    let fraction = idx - lower as f64;

    if upper >= sorted.len() {
        sorted[sorted.len() - 1]
    } else {
        sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
    }
}

/// Latency statistics
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LatencyStats {
    /// Minimum latency in milliseconds
    pub min: f64,
    /// Maximum latency in milliseconds
    pub max: f64,
    /// Mean latency in milliseconds
    pub mean: f64,
    /// Standard deviation in milliseconds
    pub std_dev: f64,
    /// Latency percentiles
    pub percentiles: Percentiles,
    /// Total number of samples
    pub count: usize,
}

impl LatencyStats {
    /// Calculate statistics from latency samples (in milliseconds)
    pub fn from_samples(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }

        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let min = sorted[0];
        let max = sorted[sorted.len() - 1];
        let sum: f64 = sorted.iter().sum();
        let mean = sum / sorted.len() as f64;

        let variance: f64 = sorted.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / sorted.len() as f64;
        let std_dev = variance.sqrt();

        let percentiles = Percentiles::from_sorted(&sorted);

        Self {
            min,
            max,
            mean,
            std_dev,
            percentiles,
            count: sorted.len(),
        }
    }

    /// Format as summary string
    pub fn format_summary(&self) -> String {
        format!(
            "min={:.2}ms max={:.2}ms mean={:.2}ms std={:.2}ms p95={:.2}ms p99={:.2}ms",
            self.min, self.max, self.mean, self.std_dev, self.percentiles.p95, self.percentiles.p99
        )
    }
}

/// Throughput statistics
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ThroughputStats {
    /// Requests per second
    pub rps: f64,
    /// Successful requests per second
    pub success_rps: f64,
    /// Total requests
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Total duration in seconds
    pub duration_secs: f64,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
}

impl ThroughputStats {
    /// Create from request counts and duration
    pub fn new(total: u64, successful: u64, duration: Duration) -> Self {
        let duration_secs = duration.as_secs_f64();
        let success_rate = if total > 0 {
            successful as f64 / total as f64
        } else {
            0.0
        };

        Self {
            rps: total as f64 / duration_secs,
            success_rps: successful as f64 / duration_secs,
            total_requests: total,
            successful_requests: successful,
            failed_requests: total - successful,
            duration_secs,
            success_rate,
        }
    }

    /// Format as summary string
    pub fn format_summary(&self) -> String {
        format!(
            "rps={:.1} success_rate={:.1}% total={} success={} failed={}",
            self.rps,
            self.success_rate * 100.0,
            self.total_requests,
            self.successful_requests,
            self.failed_requests
        )
    }
}

/// Combined performance metrics
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Metrics {
    /// Latency statistics
    pub latency: LatencyStats,
    /// Throughput statistics
    pub throughput: ThroughputStats,
    /// Error rate by type
    pub errors: ErrorStats,
}

/// Error statistics
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ErrorStats {
    /// Connection errors
    pub connection_errors: u64,
    /// Timeout errors
    pub timeout_errors: u64,
    /// HTTP 4xx errors
    pub client_errors: u64,
    /// HTTP 5xx errors
    pub server_errors: u64,
    /// Other errors
    pub other_errors: u64,
}

impl ErrorStats {
    /// Total error count
    pub fn total(&self) -> u64 {
        self.connection_errors + self.timeout_errors + self.client_errors + self.server_errors + self.other_errors
    }

    /// Record an error by status code or type
    pub fn record(&mut self, status_code: Option<u16>, is_timeout: bool, is_connection_error: bool) {
        if is_timeout {
            self.timeout_errors += 1;
        } else if is_connection_error {
            self.connection_errors += 1;
        } else if let Some(code) = status_code {
            match code {
                400..=499 => self.client_errors += 1,
                500..=599 => self.server_errors += 1,
                _ => self.other_errors += 1,
            }
        } else {
            self.other_errors += 1;
        }
    }
}

/// Real-time metrics collector
pub struct MetricsCollector {
    /// Latency samples in milliseconds
    latencies: Vec<f64>,
    /// Start time
    start_time: Instant,
    /// Successful request count
    success_count: u64,
    /// Failed request count
    fail_count: u64,
    /// Error statistics
    errors: ErrorStats,
}

impl MetricsCollector {
    /// Create a new collector
    pub fn new() -> Self {
        Self {
            latencies: Vec::new(),
            start_time: Instant::now(),
            success_count: 0,
            fail_count: 0,
            errors: ErrorStats::default(),
        }
    }

    /// Record a successful request
    pub fn record_success(&mut self, latency_ms: f64) {
        self.latencies.push(latency_ms);
        self.success_count += 1;
    }

    /// Record a failed request
    pub fn record_failure(&mut self, latency_ms: f64, status_code: Option<u16>, is_timeout: bool, is_connection_error: bool) {
        self.latencies.push(latency_ms);
        self.fail_count += 1;
        self.errors.record(status_code, is_timeout, is_connection_error);
    }

    /// Record a request result
    pub fn record(&mut self, latency_ms: f64, success: bool, status_code: Option<u16>) {
        if success {
            self.record_success(latency_ms);
        } else {
            self.record_failure(latency_ms, status_code, false, false);
        }
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> Metrics {
        let duration = self.start_time.elapsed();
        let total = self.success_count + self.fail_count;

        Metrics {
            latency: LatencyStats::from_samples(&self.latencies),
            throughput: ThroughputStats::new(total, self.success_count, duration),
            errors: self.errors.clone(),
        }
    }

    /// Finalize and return metrics
    pub fn finalize(self) -> Metrics {
        let duration = self.start_time.elapsed();
        let total = self.success_count + self.fail_count;

        Metrics {
            latency: LatencyStats::from_samples(&self.latencies),
            throughput: ThroughputStats::new(total, self.success_count, duration),
            errors: self.errors,
        }
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get current request count
    pub fn request_count(&self) -> u64 {
        self.success_count + self.fail_count
    }

    /// Get current RPS
    pub fn current_rps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.request_count() as f64 / elapsed
        } else {
            0.0
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentiles() {
        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let p = Percentiles::from_sorted(&data);

        assert!((p.p50 - 50.0).abs() < 1.0);
        assert!((p.p90 - 90.0).abs() < 1.0);
        assert!((p.p95 - 95.0).abs() < 1.0);
        assert!((p.p99 - 99.0).abs() < 1.0);
    }

    #[test]
    fn test_latency_stats() {
        let samples: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = LatencyStats::from_samples(&samples);

        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
        assert_eq!(stats.mean, 3.0);
        assert_eq!(stats.count, 5);
    }

    #[test]
    fn test_throughput_stats() {
        let stats = ThroughputStats::new(1000, 950, Duration::from_secs(10));

        assert!((stats.rps - 100.0).abs() < 0.1);
        assert!((stats.success_rate - 0.95).abs() < 0.01);
        assert_eq!(stats.failed_requests, 50);
    }

    #[test]
    fn test_metrics_collector() {
        let mut collector = MetricsCollector::new();

        for i in 0..100 {
            if i % 10 == 0 {
                collector.record_failure(10.0, Some(500), false, false);
            } else {
                collector.record_success(5.0);
            }
        }

        let metrics = collector.finalize();
        assert_eq!(metrics.throughput.total_requests, 100);
        assert_eq!(metrics.throughput.failed_requests, 10);
        assert_eq!(metrics.errors.server_errors, 10);
    }

    #[test]
    fn test_error_stats() {
        let mut errors = ErrorStats::default();
        errors.record(Some(404), false, false);
        errors.record(Some(500), false, false);
        errors.record(None, true, false);
        errors.record(None, false, true);

        assert_eq!(errors.client_errors, 1);
        assert_eq!(errors.server_errors, 1);
        assert_eq!(errors.timeout_errors, 1);
        assert_eq!(errors.connection_errors, 1);
        assert_eq!(errors.total(), 4);
    }
}
