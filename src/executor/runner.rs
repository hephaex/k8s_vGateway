//! Test execution runner
//!
//! Manages the execution of Gateway API tests.

#![allow(dead_code)]

use anyhow::{Context, Result};
use std::time::Instant;
use tracing::{error, info};

use crate::http::HttpClient;
use crate::models::{
    GatewayConfig, GatewayImpl, TestCase, TestConfig, TestResult, TestRoundSummary,
};
use crate::tests;

/// Test runner for Gateway API tests
pub struct TestRunner {
    config: TestConfig,
    client: HttpClient,
    gateway_ip: Option<String>,
}

impl TestRunner {
    /// Create a new test runner
    pub fn new(config: TestConfig) -> Result<Self> {
        let client = HttpClient::with_timeout(config.timeout_secs)?;
        Ok(Self {
            config,
            client,
            gateway_ip: None,
        })
    }

    /// Set gateway IP address
    pub fn with_gateway_ip(mut self, ip: impl Into<String>) -> Self {
        self.gateway_ip = Some(ip.into());
        self
    }

    /// Get the gateway IP (from config or discovery)
    pub fn gateway_ip(&self) -> &str {
        self.gateway_ip.as_deref().unwrap_or("127.0.0.1")
    }

    /// Run a single test case
    pub async fn run_test(&self, test_case: TestCase) -> TestResult {
        let gateway_ip = self.gateway_ip();
        let http_port = self.config.gateway.http_port;
        let https_port = self.config.gateway.https_port;
        let grpc_port = self.config.gateway.grpc_port.unwrap_or(9090);
        let hostname = &self.config.gateway.hostname;

        // Check if test should be skipped
        if self.config.skip_tests.contains(&test_case.number()) {
            return TestResult::skip(test_case, "Skipped by configuration");
        }

        info!("Running {}", test_case);

        let result = tests::run_test(
            test_case, gateway_ip, http_port, https_port, grpc_port, hostname,
        )
        .await;

        match result {
            Ok(result) => result,
            Err(e) => {
                error!("Test {} failed with error: {}", test_case, e);
                TestResult::error(test_case, e.to_string())
            }
        }
    }

    /// Run all test cases sequentially
    pub async fn run_all(&self) -> Result<TestRoundSummary> {
        info!(
            "Starting test round for {} Gateway",
            self.config.gateway.implementation
        );

        let start = Instant::now();
        let mut results = Vec::new();

        for test_case in TestCase::all() {
            let result = self.run_test(test_case).await;
            info!("  {}", result);
            results.push(result);
        }

        let summary = TestRoundSummary::new(1, self.config.gateway.implementation.name(), results);

        info!(
            "Test round completed in {}ms - Pass: {}/{} ({:.1}%)",
            start.elapsed().as_millis(),
            summary.passed,
            summary.total,
            summary.pass_rate()
        );

        Ok(summary)
    }

    /// Run multiple test rounds
    pub async fn run_rounds(&self, num_rounds: u32) -> Result<Vec<TestRoundSummary>> {
        info!(
            "Running {} rounds for {} Gateway",
            num_rounds, self.config.gateway.implementation
        );

        let mut summaries = Vec::new();

        for round in 1..=num_rounds {
            info!("=== Round {}/{} ===", round, num_rounds);

            let mut results = Vec::new();

            for test_case in TestCase::all() {
                let result = self.run_test(test_case).await;
                results.push(result);
            }

            let summary =
                TestRoundSummary::new(round, self.config.gateway.implementation.name(), results);

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

    /// Run specific test cases
    pub async fn run_tests(&self, test_cases: &[TestCase]) -> Result<TestRoundSummary> {
        info!(
            "Running {} selected tests for {} Gateway",
            test_cases.len(),
            self.config.gateway.implementation
        );

        let mut results = Vec::new();

        for &test_case in test_cases {
            let result = self.run_test(test_case).await;
            info!("  {}", result);
            results.push(result);
        }

        Ok(TestRoundSummary::new(
            1,
            self.config.gateway.implementation.name(),
            results,
        ))
    }
}

/// Multi-gateway test runner
pub struct MultiGatewayRunner {
    gateways: Vec<(GatewayImpl, String)>, // (implementation, IP)
    rounds: u32,
    timeout_secs: u64,
}

impl MultiGatewayRunner {
    pub fn new() -> Self {
        Self {
            gateways: Vec::new(),
            rounds: 1,
            timeout_secs: 30,
        }
    }

    pub fn add_gateway(mut self, implementation: GatewayImpl, ip: impl Into<String>) -> Self {
        self.gateways.push((implementation, ip.into()));
        self
    }

    pub fn rounds(mut self, rounds: u32) -> Self {
        self.rounds = rounds;
        self
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Run tests for all configured gateways
    pub async fn run_all(&self) -> Result<Vec<(GatewayImpl, Vec<TestRoundSummary>)>> {
        let mut all_results = Vec::new();

        for (implementation, ip) in &self.gateways {
            info!("Testing {} Gateway at {}", implementation, ip);

            let config =
                TestConfig::new(GatewayConfig::new(*implementation)).with_rounds(self.rounds);

            let runner = TestRunner::new(config)?.with_gateway_ip(ip);
            let summaries = runner.run_rounds(self.rounds).await?;

            all_results.push((*implementation, summaries));
        }

        Ok(all_results)
    }
}

impl Default for MultiGatewayRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick test runner for single gateway
pub async fn quick_test(gateway_ip: &str, implementation: GatewayImpl) -> Result<TestRoundSummary> {
    let config = TestConfig::new(GatewayConfig::new(implementation));
    let runner = TestRunner::new(config)?.with_gateway_ip(gateway_ip);
    runner.run_all().await
}

/// Run specific test by number
pub async fn run_test_by_number(
    gateway_ip: &str,
    implementation: GatewayImpl,
    test_number: u8,
) -> Result<TestResult> {
    let test_case = TestCase::from_number(test_number)
        .context(format!("Invalid test number: {test_number}"))?;

    let config = TestConfig::new(GatewayConfig::new(implementation));
    let runner = TestRunner::new(config)?.with_gateway_ip(gateway_ip);

    Ok(runner.run_test(test_case).await)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_runner_creation() {
        let config = TestConfig::new(GatewayConfig::new(GatewayImpl::Nginx));
        let runner = TestRunner::new(config);
        assert!(runner.is_ok());
    }

    #[test]
    fn test_multi_gateway_builder() {
        let runner = MultiGatewayRunner::new()
            .add_gateway(GatewayImpl::Nginx, "10.0.0.1")
            .add_gateway(GatewayImpl::Envoy, "10.0.0.2")
            .rounds(5);

        assert_eq!(runner.gateways.len(), 2);
        assert_eq!(runner.rounds, 5);
    }
}
