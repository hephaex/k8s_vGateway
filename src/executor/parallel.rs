//! Parallel test execution
//!
//! Enables concurrent execution of tests across multiple gateways.

#![allow(dead_code)]

use anyhow::Result;
use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tracing::{debug, info};

use crate::http::HttpClient;

use crate::models::{
    GatewayConfig, GatewayImpl, TestCase, TestResult, TestRoundSummary, TestStatus,
};
use crate::tests;

/// Parallel test executor
pub struct ParallelExecutor {
    max_concurrent: usize,
    timeout_secs: u64,
}

impl ParallelExecutor {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            timeout_secs: 30,
        }
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Run tests in parallel for a single gateway
    pub async fn run_tests_parallel(
        &self,
        gateway_ip: &str,
        gateway_config: &GatewayConfig,
        test_cases: Vec<TestCase>,
    ) -> Result<Vec<TestResult>> {
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        let client = Arc::new(HttpClient::with_timeout(self.timeout_secs)?);

        let gateway_ip = gateway_ip.to_string();
        let http_port = gateway_config.http_port;
        let https_port = gateway_config.https_port;
        let grpc_port = gateway_config.grpc_port.unwrap_or(9090);
        let hostname = gateway_config.hostname.clone();

        let mut handles = Vec::new();

        for test_case in test_cases {
            let semaphore = semaphore.clone();
            let _client = client.clone();
            let gateway_ip = gateway_ip.clone();
            let hostname = hostname.clone();

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();

                debug!("Starting parallel execution of {}", test_case);
                let _start = Instant::now();

                let result = tests::run_test(
                    test_case,
                    &gateway_ip,
                    http_port,
                    https_port,
                    grpc_port,
                    &hostname,
                )
                .await;

                match result {
                    Ok(r) => r,
                    Err(e) => TestResult::error(test_case, e.to_string()),
                }
            });

            handles.push(handle);
        }

        let results: Vec<TestResult> = join_all(handles)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Run all 17 tests in parallel
    pub async fn run_all_parallel(
        &self,
        gateway_ip: &str,
        gateway_config: &GatewayConfig,
    ) -> Result<TestRoundSummary> {
        info!(
            "Running all tests in parallel (max {} concurrent) for {}",
            self.max_concurrent, gateway_config.implementation
        );

        let start = Instant::now();
        let results = self
            .run_tests_parallel(gateway_ip, gateway_config, TestCase::all())
            .await?;

        // Sort results by test number
        let mut sorted_results = results;
        sorted_results.sort_by_key(|r| r.test_case.number());

        let summary =
            TestRoundSummary::new(1, gateway_config.implementation.name(), sorted_results);

        info!(
            "Parallel execution completed in {}ms - Pass: {}/{} ({:.1}%)",
            start.elapsed().as_millis(),
            summary.passed,
            summary.total,
            summary.pass_rate()
        );

        Ok(summary)
    }

    /// Run tests across multiple gateways in parallel
    pub async fn run_multi_gateway(
        &self,
        gateways: Vec<(GatewayImpl, String)>,
    ) -> Result<HashMap<GatewayImpl, TestRoundSummary>> {
        info!("Running parallel tests across {} gateways", gateways.len());

        let start = Instant::now();
        let mut handles = Vec::new();

        for (implementation, gateway_ip) in gateways {
            let max_concurrent = self.max_concurrent;
            let timeout_secs = self.timeout_secs;

            let handle = tokio::spawn(async move {
                let executor = ParallelExecutor::new(max_concurrent).with_timeout(timeout_secs);
                let config = GatewayConfig::new(implementation);

                let result = executor.run_all_parallel(&gateway_ip, &config).await;
                (implementation, result)
            });

            handles.push(handle);
        }

        let results = join_all(handles).await;
        let mut summaries = HashMap::new();

        for (impl_, summary) in results
            .into_iter()
            .flatten()
            .filter_map(|(i, r)| r.ok().map(|s| (i, s)))
        {
            summaries.insert(impl_, summary);
        }

        info!(
            "Multi-gateway parallel execution completed in {}ms",
            start.elapsed().as_millis()
        );

        Ok(summaries)
    }
}

impl Default for ParallelExecutor {
    fn default() -> Self {
        Self::new(4)
    }
}

/// Batch test runner for multiple rounds
pub struct BatchRunner {
    executor: ParallelExecutor,
    rounds: u32,
}

impl BatchRunner {
    pub fn new(max_concurrent: usize, rounds: u32) -> Self {
        Self {
            executor: ParallelExecutor::new(max_concurrent),
            rounds,
        }
    }

    /// Run multiple rounds of parallel tests
    pub async fn run_rounds(
        &self,
        gateway_ip: &str,
        gateway_config: &GatewayConfig,
    ) -> Result<Vec<TestRoundSummary>> {
        info!(
            "Running {} rounds of parallel tests for {}",
            self.rounds, gateway_config.implementation
        );

        let mut summaries = Vec::new();

        for round in 1..=self.rounds {
            info!("=== Round {}/{} ===", round, self.rounds);

            let results = self
                .executor
                .run_tests_parallel(gateway_ip, gateway_config, TestCase::all())
                .await?;

            let mut sorted_results = results;
            sorted_results.sort_by_key(|r| r.test_case.number());

            let summary =
                TestRoundSummary::new(round, gateway_config.implementation.name(), sorted_results);

            info!(
                "Round {} completed: {}/{} passed ({:.1}%)",
                round,
                summary.passed,
                summary.total,
                summary.pass_rate()
            );

            summaries.push(summary);
        }

        Ok(summaries)
    }

    /// Aggregate results across multiple rounds
    pub fn aggregate_results(summaries: &[TestRoundSummary]) -> AggregateResult {
        let total_rounds = summaries.len() as u32;
        let mut test_stats: HashMap<TestCase, TestStats> = HashMap::new();

        for summary in summaries {
            for result in &summary.results {
                let stats = test_stats.entry(result.test_case).or_default();

                match result.status {
                    TestStatus::Pass => stats.passes += 1,
                    TestStatus::Fail => stats.failures += 1,
                    TestStatus::Skip => stats.skips += 1,
                    TestStatus::Error => stats.errors += 1,
                }
                stats.total_duration_ms += result.duration_ms;
            }
        }

        // Calculate pass rates
        let test_pass_rates: HashMap<TestCase, f64> = test_stats
            .iter()
            .map(|(tc, stats)| {
                let total = stats.passes + stats.failures + stats.errors;
                let rate = if total > 0 {
                    (stats.passes as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                (*tc, rate)
            })
            .collect();

        let overall_pass_rate =
            summaries.iter().map(|s| s.pass_rate()).sum::<f64>() / summaries.len() as f64;

        AggregateResult {
            total_rounds,
            test_stats,
            test_pass_rates,
            overall_pass_rate,
        }
    }
}

/// Statistics for a single test case across rounds
#[derive(Clone, Debug, Default)]
pub struct TestStats {
    pub passes: u32,
    pub failures: u32,
    pub skips: u32,
    pub errors: u32,
    pub total_duration_ms: u64,
}

impl TestStats {
    pub fn avg_duration_ms(&self) -> u64 {
        let total = self.passes + self.failures + self.errors;
        if total > 0 {
            self.total_duration_ms / total as u64
        } else {
            0
        }
    }
}

/// Aggregate results across multiple test rounds
#[derive(Clone, Debug)]
pub struct AggregateResult {
    pub total_rounds: u32,
    pub test_stats: HashMap<TestCase, TestStats>,
    pub test_pass_rates: HashMap<TestCase, f64>,
    pub overall_pass_rate: f64,
}

impl AggregateResult {
    /// Get tests sorted by pass rate (lowest first)
    pub fn flaky_tests(&self) -> Vec<(TestCase, f64)> {
        let mut tests: Vec<_> = self
            .test_pass_rates
            .iter()
            .map(|(tc, rate)| (*tc, *rate))
            .collect();
        tests.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        tests
    }

    /// Get tests that always pass
    pub fn stable_tests(&self) -> Vec<TestCase> {
        self.test_pass_rates
            .iter()
            .filter(|(_, rate)| **rate >= 100.0)
            .map(|(tc, _)| *tc)
            .collect()
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_parallel_executor_creation() {
        let executor = ParallelExecutor::new(8).with_timeout(60);
        assert_eq!(executor.max_concurrent, 8);
        assert_eq!(executor.timeout_secs, 60);
    }

    #[test]
    fn test_batch_runner_creation() {
        let runner = BatchRunner::new(4, 10);
        assert_eq!(runner.rounds, 10);
    }

    #[test]
    fn test_aggregate_results() {
        let results1 = vec![
            TestResult::pass(TestCase::HostRouting, 100),
            TestResult::fail(TestCase::PathRouting, 50, "failed"),
        ];
        let results2 = vec![
            TestResult::pass(TestCase::HostRouting, 120),
            TestResult::pass(TestCase::PathRouting, 60),
        ];

        let summaries = vec![
            TestRoundSummary::new(1, "nginx", results1),
            TestRoundSummary::new(2, "nginx", results2),
        ];

        let aggregate = BatchRunner::aggregate_results(&summaries);
        assert_eq!(aggregate.total_rounds, 2);
        assert_eq!(
            aggregate.test_pass_rates.get(&TestCase::HostRouting),
            Some(&100.0)
        );
        assert_eq!(
            aggregate.test_pass_rates.get(&TestCase::PathRouting),
            Some(&50.0)
        );
    }
}
