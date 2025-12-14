//! Benchmark execution engine
//!
//! Provides configurable load testing with various patterns.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, info};

use super::metrics::{Metrics, MetricsCollector};
use crate::http::HttpClient;
use crate::models::GatewayImpl;

/// Load pattern for benchmark
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LoadPattern {
    /// Constant load at specified RPS
    Constant { rps: u32 },
    /// Ramp up from start to end RPS over duration
    Ramp {
        start_rps: u32,
        end_rps: u32,
        duration_secs: u64,
    },
    /// Step increase in load
    Step {
        start_rps: u32,
        step_rps: u32,
        step_interval_secs: u64,
        max_rps: u32,
    },
    /// Spike pattern for stress testing
    Spike {
        base_rps: u32,
        spike_rps: u32,
        spike_duration_secs: u64,
    },
    /// Maximum throughput (as fast as possible)
    Max { concurrency: u32 },
}

impl Default for LoadPattern {
    fn default() -> Self {
        LoadPattern::Constant { rps: 100 }
    }
}

impl LoadPattern {
    /// Get target RPS at a given time offset
    pub fn rps_at(&self, elapsed_secs: f64, total_duration_secs: f64) -> u32 {
        match self {
            LoadPattern::Constant { rps } => *rps,
            LoadPattern::Ramp {
                start_rps,
                end_rps,
                duration_secs,
            } => {
                let progress = (elapsed_secs / *duration_secs as f64).min(1.0);
                let delta = *end_rps as f64 - *start_rps as f64;
                (*start_rps as f64 + delta * progress) as u32
            }
            LoadPattern::Step {
                start_rps,
                step_rps,
                step_interval_secs,
                max_rps,
            } => {
                let steps = (elapsed_secs / *step_interval_secs as f64) as u32;
                (*start_rps + steps * step_rps).min(*max_rps)
            }
            LoadPattern::Spike {
                base_rps,
                spike_rps,
                spike_duration_secs,
            } => {
                // Spike in the middle of the test
                let spike_start = total_duration_secs / 3.0;
                let spike_end = spike_start + *spike_duration_secs as f64;
                if elapsed_secs >= spike_start && elapsed_secs < spike_end {
                    *spike_rps
                } else {
                    *base_rps
                }
            }
            LoadPattern::Max { .. } => u32::MAX,
        }
    }
}

/// Benchmark configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Target gateway
    pub gateway: GatewayImpl,
    /// Gateway IP address
    pub gateway_ip: String,
    /// Gateway port
    pub port: u16,
    /// Target URL path
    pub path: String,
    /// Host header
    pub hostname: String,
    /// Load pattern
    pub pattern: LoadPattern,
    /// Test duration in seconds
    pub duration_secs: u64,
    /// Number of concurrent connections
    pub concurrency: u32,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Warmup duration in seconds
    pub warmup_secs: u64,
    /// Enable keep-alive
    pub keep_alive: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            gateway: GatewayImpl::Nginx,
            gateway_ip: "127.0.0.1".to_string(),
            port: 80,
            path: "/".to_string(),
            hostname: "example.com".to_string(),
            pattern: LoadPattern::default(),
            duration_secs: 60,
            concurrency: 10,
            timeout_ms: 5000,
            warmup_secs: 5,
            keep_alive: true,
        }
    }
}

impl BenchmarkConfig {
    /// Create with gateway and IP
    pub fn new(gateway: GatewayImpl, gateway_ip: impl Into<String>) -> Self {
        Self {
            gateway,
            gateway_ip: gateway_ip.into(),
            ..Default::default()
        }
    }

    /// Set load pattern
    pub fn with_pattern(mut self, pattern: LoadPattern) -> Self {
        self.pattern = pattern;
        self
    }

    /// Set duration
    pub fn with_duration(mut self, secs: u64) -> Self {
        self.duration_secs = secs;
        self
    }

    /// Set concurrency
    pub fn with_concurrency(mut self, concurrency: u32) -> Self {
        self.concurrency = concurrency;
        self
    }

    /// Set target path
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Set hostname
    pub fn with_hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = hostname.into();
        self
    }

    /// Get full URL
    pub fn url(&self) -> String {
        format!("http://{}:{}{}", self.gateway_ip, self.port, self.path)
    }
}

/// Benchmark result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Configuration used
    pub config: BenchmarkConfig,
    /// Performance metrics
    pub metrics: Metrics,
    /// Benchmark start time (Unix timestamp)
    pub start_time: u64,
    /// Benchmark end time (Unix timestamp)
    pub end_time: u64,
    /// Whether warmup was performed
    pub warmup_performed: bool,
}

impl BenchmarkResult {
    /// Format as summary string
    pub fn format_summary(&self) -> String {
        format!(
            "{} Benchmark Results:\n\
             Duration: {:.1}s | Requests: {} | RPS: {:.1}\n\
             Latency: {}\n\
             Throughput: {}",
            self.config.gateway.name(),
            self.metrics.throughput.duration_secs,
            self.metrics.throughput.total_requests,
            self.metrics.throughput.rps,
            self.metrics.latency.format_summary(),
            self.metrics.throughput.format_summary()
        )
    }
}

/// Benchmark runner
pub struct BenchmarkRunner {
    config: BenchmarkConfig,
    http_client: HttpClient,
    running: Arc<AtomicBool>,
    request_count: Arc<AtomicU64>,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner
    pub fn new(config: BenchmarkConfig) -> Self {
        let timeout_secs = config.timeout_ms / 1000;
        let http_client =
            HttpClient::with_timeout(timeout_secs.max(1)).expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
            running: Arc::new(AtomicBool::new(false)),
            request_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Run the benchmark
    pub async fn run(&self) -> Result<BenchmarkResult> {
        info!(
            "Starting benchmark for {} at {}",
            self.config.gateway.name(),
            self.config.url()
        );

        // Warmup phase
        let warmup_performed = if self.config.warmup_secs > 0 {
            info!("Warmup phase: {} seconds", self.config.warmup_secs);
            self.warmup().await?;
            true
        } else {
            false
        };

        let start_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Main benchmark
        self.running.store(true, Ordering::SeqCst);
        let metrics = self.run_load_test().await?;
        self.running.store(false, Ordering::SeqCst);

        let end_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        info!(
            "Benchmark complete: {} requests, {:.1} RPS, p99={:.2}ms",
            metrics.throughput.total_requests,
            metrics.throughput.rps,
            metrics.latency.percentiles.p99
        );

        Ok(BenchmarkResult {
            config: self.config.clone(),
            metrics,
            start_time,
            end_time,
            warmup_performed,
        })
    }

    /// Warmup phase
    async fn warmup(&self) -> Result<()> {
        let url = self.config.url();
        let warmup_duration = Duration::from_secs(self.config.warmup_secs);
        let start = Instant::now();

        while start.elapsed() < warmup_duration {
            let _ = self.http_client.get(&url).await;
            sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Run the main load test
    async fn run_load_test(&self) -> Result<Metrics> {
        let collector = Arc::new(Mutex::new(MetricsCollector::new()));
        let duration = Duration::from_secs(self.config.duration_secs);

        match &self.config.pattern {
            LoadPattern::Max { concurrency } => {
                self.run_max_throughput(*concurrency, duration, collector.clone())
                    .await?;
            }
            _ => {
                self.run_rate_limited(duration, collector.clone()).await?;
            }
        }

        let metrics = collector.lock().await.snapshot();
        Ok(metrics)
    }

    /// Run with rate limiting
    async fn run_rate_limited(
        &self,
        duration: Duration,
        collector: Arc<Mutex<MetricsCollector>>,
    ) -> Result<()> {
        let url = self.config.url();
        let hostname = self.config.hostname.clone();
        let start = Instant::now();
        let total_duration_secs = duration.as_secs_f64();

        let mut handles = Vec::new();
        let concurrency = self.config.concurrency.min(100);

        for _ in 0..concurrency {
            let url = url.clone();
            let hostname = hostname.clone();
            let collector = collector.clone();
            let client = self.http_client.clone();
            let running = self.running.clone();
            let pattern = self.config.pattern.clone();

            let handle = tokio::spawn(async move {
                while running.load(Ordering::SeqCst) {
                    let elapsed = start.elapsed();
                    if elapsed >= duration {
                        break;
                    }

                    let elapsed_secs = elapsed.as_secs_f64();
                    let target_rps = pattern.rps_at(elapsed_secs, total_duration_secs);

                    // Calculate delay for rate limiting
                    let delay_ms = if target_rps > 0 && target_rps < u32::MAX {
                        1000 / target_rps
                    } else {
                        0
                    };

                    let request_start = Instant::now();
                    let result = client.get_with_host(&url, &hostname).await;
                    let latency_ms = request_start.elapsed().as_secs_f64() * 1000.0;

                    let mut coll = collector.lock().await;
                    match result {
                        Ok(resp) => {
                            let success = resp.status_code >= 200 && resp.status_code < 400;
                            coll.record(latency_ms, success, Some(resp.status_code));
                        }
                        Err(_) => {
                            coll.record_failure(latency_ms, None, false, true);
                        }
                    }
                    drop(coll);

                    if delay_ms > 0 {
                        sleep(Duration::from_millis(delay_ms as u64)).await;
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all workers
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }

    /// Run at maximum throughput
    async fn run_max_throughput(
        &self,
        concurrency: u32,
        duration: Duration,
        collector: Arc<Mutex<MetricsCollector>>,
    ) -> Result<()> {
        let url = self.config.url();
        let hostname = self.config.hostname.clone();
        let start = Instant::now();

        let mut handles = Vec::new();

        for _ in 0..concurrency {
            let url = url.clone();
            let hostname = hostname.clone();
            let collector = collector.clone();
            let client = self.http_client.clone();
            let running = self.running.clone();

            let handle = tokio::spawn(async move {
                while running.load(Ordering::SeqCst) && start.elapsed() < duration {
                    let request_start = Instant::now();
                    let result = client.get_with_host(&url, &hostname).await;
                    let latency_ms = request_start.elapsed().as_secs_f64() * 1000.0;

                    let mut coll = collector.lock().await;
                    match result {
                        Ok(resp) => {
                            let success = resp.status_code >= 200 && resp.status_code < 400;
                            coll.record(latency_ms, success, Some(resp.status_code));
                        }
                        Err(_) => {
                            coll.record_failure(latency_ms, None, false, true);
                        }
                    }
                }
            });

            handles.push(handle);
        }

        // Progress reporting
        let collector_progress = collector.clone();
        let progress_handle = tokio::spawn(async move {
            while start.elapsed() < duration {
                sleep(Duration::from_secs(5)).await;
                let coll = collector_progress.lock().await;
                let elapsed = coll.elapsed().as_secs_f64();
                let count = coll.request_count();
                let rps = coll.current_rps();
                debug!(
                    "Progress: {:.0}s elapsed, {} requests, {:.1} RPS",
                    elapsed, count, rps
                );
            }
        });

        // Wait for all workers
        for handle in handles {
            let _ = handle.await;
        }
        progress_handle.abort();

        Ok(())
    }

    /// Stop the benchmark
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Compare multiple gateway benchmarks
pub struct BenchmarkComparison {
    results: Vec<BenchmarkResult>,
}

impl BenchmarkComparison {
    /// Create new comparison
    pub fn new(results: Vec<BenchmarkResult>) -> Self {
        Self { results }
    }

    /// Get results sorted by RPS (descending)
    pub fn by_rps(&self) -> Vec<&BenchmarkResult> {
        let mut sorted: Vec<_> = self.results.iter().collect();
        sorted.sort_by(|a, b| {
            b.metrics
                .throughput
                .rps
                .partial_cmp(&a.metrics.throughput.rps)
                .unwrap()
        });
        sorted
    }

    /// Get results sorted by p99 latency (ascending)
    pub fn by_latency(&self) -> Vec<&BenchmarkResult> {
        let mut sorted: Vec<_> = self.results.iter().collect();
        sorted.sort_by(|a, b| {
            a.metrics
                .latency
                .percentiles
                .p99
                .partial_cmp(&b.metrics.latency.percentiles.p99)
                .unwrap()
        });
        sorted
    }

    /// Format comparison table
    pub fn format_table(&self) -> String {
        let mut output = String::new();
        output.push_str(
            "\n┌────────────────────────┬──────────┬──────────┬──────────┬──────────┬──────────┐\n",
        );
        output.push_str(
            "│ Gateway                │      RPS │  p50(ms) │  p95(ms) │  p99(ms) │ Success% │\n",
        );
        output.push_str(
            "├────────────────────────┼──────────┼──────────┼──────────┼──────────┼──────────┤\n",
        );

        for result in self.by_rps() {
            output.push_str(&format!(
                "│ {:22} │ {:>8.1} │ {:>8.2} │ {:>8.2} │ {:>8.2} │ {:>7.1}% │\n",
                result.config.gateway.name(),
                result.metrics.throughput.rps,
                result.metrics.latency.percentiles.p50,
                result.metrics.latency.percentiles.p95,
                result.metrics.latency.percentiles.p99,
                result.metrics.throughput.success_rate * 100.0
            ));
        }

        output.push_str(
            "└────────────────────────┴──────────┴──────────┴──────────┴──────────┴──────────┘\n",
        );
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_pattern_constant() {
        let pattern = LoadPattern::Constant { rps: 100 };
        assert_eq!(pattern.rps_at(0.0, 60.0), 100);
        assert_eq!(pattern.rps_at(30.0, 60.0), 100);
        assert_eq!(pattern.rps_at(60.0, 60.0), 100);
    }

    #[test]
    fn test_load_pattern_ramp() {
        let pattern = LoadPattern::Ramp {
            start_rps: 100,
            end_rps: 200,
            duration_secs: 60,
        };
        assert_eq!(pattern.rps_at(0.0, 60.0), 100);
        assert_eq!(pattern.rps_at(30.0, 60.0), 150);
        assert_eq!(pattern.rps_at(60.0, 60.0), 200);
    }

    #[test]
    fn test_load_pattern_step() {
        let pattern = LoadPattern::Step {
            start_rps: 100,
            step_rps: 50,
            step_interval_secs: 10,
            max_rps: 300,
        };
        assert_eq!(pattern.rps_at(0.0, 60.0), 100);
        assert_eq!(pattern.rps_at(10.0, 60.0), 150);
        assert_eq!(pattern.rps_at(50.0, 60.0), 300); // Capped at max
    }

    #[test]
    fn test_benchmark_config() {
        let config = BenchmarkConfig::new(GatewayImpl::Nginx, "10.0.0.1")
            .with_pattern(LoadPattern::Constant { rps: 500 })
            .with_duration(120)
            .with_concurrency(20);

        assert_eq!(config.gateway, GatewayImpl::Nginx);
        assert_eq!(config.gateway_ip, "10.0.0.1");
        assert_eq!(config.duration_secs, 120);
        assert_eq!(config.concurrency, 20);
    }

    #[test]
    fn test_benchmark_url() {
        let config =
            BenchmarkConfig::new(GatewayImpl::Envoy, "192.168.1.100").with_path("/api/test");

        assert_eq!(config.url(), "http://192.168.1.100:80/api/test");
    }
}
