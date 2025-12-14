//! Benchmarking and performance metrics module
//!
//! Provides latency measurements, throughput testing, and performance
//! comparison across Gateway API implementations.

#![allow(dead_code)]
#![allow(unused_imports)]

mod metrics;
mod report;
mod runner;

pub use metrics::{LatencyStats, Metrics, MetricsCollector, Percentiles, ThroughputStats};
pub use report::{BenchmarkReport, ReportFormat as BenchmarkReportFormat};
pub use runner::{BenchmarkConfig, BenchmarkResult, BenchmarkRunner, LoadPattern};
