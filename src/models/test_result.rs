//! Test result models for Gateway API testing
//!
//! Defines test cases, results, and status types.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

/// All 17 test cases for Gateway API
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestCase {
    // Routing tests (1-3)
    HostRouting,
    PathRouting,
    HeaderRouting,

    // TLS tests (4-6)
    TlsTermination,
    HttpsRedirect,
    BackendTls,

    // Traffic management tests (7-10)
    CanaryTraffic,
    RateLimiting,
    TimeoutRetry,
    SessionAffinity,

    // Advanced tests (11-17)
    UrlRewrite,
    HeaderModifier,
    CrossNamespace,
    GrpcRouting,
    HealthCheck,
    LoadTest,
    FailoverRecovery,
}

impl TestCase {
    /// Get test case number (1-17)
    pub fn number(&self) -> u8 {
        match self {
            TestCase::HostRouting => 1,
            TestCase::PathRouting => 2,
            TestCase::HeaderRouting => 3,
            TestCase::TlsTermination => 4,
            TestCase::HttpsRedirect => 5,
            TestCase::BackendTls => 6,
            TestCase::CanaryTraffic => 7,
            TestCase::RateLimiting => 8,
            TestCase::TimeoutRetry => 9,
            TestCase::SessionAffinity => 10,
            TestCase::UrlRewrite => 11,
            TestCase::HeaderModifier => 12,
            TestCase::CrossNamespace => 13,
            TestCase::GrpcRouting => 14,
            TestCase::HealthCheck => 15,
            TestCase::LoadTest => 16,
            TestCase::FailoverRecovery => 17,
        }
    }

    /// Get test case name
    pub fn name(&self) -> &'static str {
        match self {
            TestCase::HostRouting => "Host Routing",
            TestCase::PathRouting => "Path Routing",
            TestCase::HeaderRouting => "Header Routing",
            TestCase::TlsTermination => "TLS Termination",
            TestCase::HttpsRedirect => "HTTPS Redirect",
            TestCase::BackendTls => "Backend TLS (mTLS)",
            TestCase::CanaryTraffic => "Canary Traffic",
            TestCase::RateLimiting => "Rate Limiting",
            TestCase::TimeoutRetry => "Timeout & Retry",
            TestCase::SessionAffinity => "Session Affinity",
            TestCase::UrlRewrite => "URL Rewrite",
            TestCase::HeaderModifier => "Header Modifier",
            TestCase::CrossNamespace => "Cross Namespace",
            TestCase::GrpcRouting => "gRPC Routing",
            TestCase::HealthCheck => "Health Check",
            TestCase::LoadTest => "Load Test",
            TestCase::FailoverRecovery => "Failover Recovery",
        }
    }

    /// Get test category
    pub fn category(&self) -> &'static str {
        match self {
            TestCase::HostRouting | TestCase::PathRouting | TestCase::HeaderRouting => "Routing",
            TestCase::TlsTermination | TestCase::HttpsRedirect | TestCase::BackendTls => "TLS",
            TestCase::CanaryTraffic
            | TestCase::RateLimiting
            | TestCase::TimeoutRetry
            | TestCase::SessionAffinity => "Traffic",
            _ => "Advanced",
        }
    }

    /// Get all test cases
    pub fn all() -> Vec<TestCase> {
        vec![
            TestCase::HostRouting,
            TestCase::PathRouting,
            TestCase::HeaderRouting,
            TestCase::TlsTermination,
            TestCase::HttpsRedirect,
            TestCase::BackendTls,
            TestCase::CanaryTraffic,
            TestCase::RateLimiting,
            TestCase::TimeoutRetry,
            TestCase::SessionAffinity,
            TestCase::UrlRewrite,
            TestCase::HeaderModifier,
            TestCase::CrossNamespace,
            TestCase::GrpcRouting,
            TestCase::HealthCheck,
            TestCase::LoadTest,
            TestCase::FailoverRecovery,
        ]
    }

    /// Parse from test number
    pub fn from_number(n: u8) -> Option<TestCase> {
        match n {
            1 => Some(TestCase::HostRouting),
            2 => Some(TestCase::PathRouting),
            3 => Some(TestCase::HeaderRouting),
            4 => Some(TestCase::TlsTermination),
            5 => Some(TestCase::HttpsRedirect),
            6 => Some(TestCase::BackendTls),
            7 => Some(TestCase::CanaryTraffic),
            8 => Some(TestCase::RateLimiting),
            9 => Some(TestCase::TimeoutRetry),
            10 => Some(TestCase::SessionAffinity),
            11 => Some(TestCase::UrlRewrite),
            12 => Some(TestCase::HeaderModifier),
            13 => Some(TestCase::CrossNamespace),
            14 => Some(TestCase::GrpcRouting),
            15 => Some(TestCase::HealthCheck),
            16 => Some(TestCase::LoadTest),
            17 => Some(TestCase::FailoverRecovery),
            _ => None,
        }
    }
}

impl fmt::Display for TestCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Test {}: {}", self.number(), self.name())
    }
}

/// Test execution status
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestStatus {
    Pass,
    Fail,
    Skip,
    Error,
}

impl TestStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            TestStatus::Pass => "✓",
            TestStatus::Fail => "✗",
            TestStatus::Skip => "○",
            TestStatus::Error => "!",
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, TestStatus::Pass)
    }
}

impl fmt::Display for TestStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestStatus::Pass => write!(f, "PASS"),
            TestStatus::Fail => write!(f, "FAIL"),
            TestStatus::Skip => write!(f, "SKIP"),
            TestStatus::Error => write!(f, "ERROR"),
        }
    }
}

/// Result of a single test execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestResult {
    pub test_case: TestCase,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub message: Option<String>,
    pub details: Option<serde_json::Value>,
}

impl TestResult {
    pub fn pass(test_case: TestCase, duration_ms: u64) -> Self {
        Self {
            test_case,
            status: TestStatus::Pass,
            duration_ms,
            message: None,
            details: None,
        }
    }

    pub fn fail(test_case: TestCase, duration_ms: u64, message: impl Into<String>) -> Self {
        Self {
            test_case,
            status: TestStatus::Fail,
            duration_ms,
            message: Some(message.into()),
            details: None,
        }
    }

    pub fn skip(test_case: TestCase, reason: impl Into<String>) -> Self {
        Self {
            test_case,
            status: TestStatus::Skip,
            duration_ms: 0,
            message: Some(reason.into()),
            details: None,
        }
    }

    pub fn error(test_case: TestCase, error: impl Into<String>) -> Self {
        Self {
            test_case,
            status: TestStatus::Error,
            duration_ms: 0,
            message: Some(error.into()),
            details: None,
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl fmt::Display for TestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} [{}ms]",
            self.status.symbol(),
            self.test_case,
            self.duration_ms
        )?;
        if let Some(msg) = &self.message {
            write!(f, " - {msg}")?;
        }
        Ok(())
    }
}

/// Summary of test round execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestRoundSummary {
    pub round: u32,
    pub gateway: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub errors: usize,
    pub total_duration_ms: u64,
    pub results: Vec<TestResult>,
}

impl TestRoundSummary {
    pub fn new(round: u32, gateway: impl Into<String>, results: Vec<TestResult>) -> Self {
        let total = results.len();
        let passed = results
            .iter()
            .filter(|r| r.status == TestStatus::Pass)
            .count();
        let failed = results
            .iter()
            .filter(|r| r.status == TestStatus::Fail)
            .count();
        let skipped = results
            .iter()
            .filter(|r| r.status == TestStatus::Skip)
            .count();
        let errors = results
            .iter()
            .filter(|r| r.status == TestStatus::Error)
            .count();
        let total_duration_ms = results.iter().map(|r| r.duration_ms).sum();

        Self {
            round,
            gateway: gateway.into(),
            total,
            passed,
            failed,
            skipped,
            errors,
            total_duration_ms,
            results,
        }
    }

    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.passed as f64 / self.total as f64) * 100.0
        }
    }

    pub fn is_all_passed(&self) -> bool {
        self.passed == self.total
    }
}

impl fmt::Display for TestRoundSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Round {} - {} Gateway", self.round, self.gateway)?;
        writeln!(f, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;
        for result in &self.results {
            writeln!(f, "  {result}")?;
        }
        writeln!(f, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;
        writeln!(
            f,
            "Total: {} | Pass: {} | Fail: {} | Skip: {} | Error: {}",
            self.total, self.passed, self.failed, self.skipped, self.errors
        )?;
        writeln!(
            f,
            "Pass Rate: {:.1}% | Duration: {}ms",
            self.pass_rate(),
            self.total_duration_ms
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_numbers() {
        assert_eq!(TestCase::HostRouting.number(), 1);
        assert_eq!(TestCase::FailoverRecovery.number(), 17);
    }

    #[test]
    fn test_case_from_number() {
        assert_eq!(TestCase::from_number(1), Some(TestCase::HostRouting));
        assert_eq!(TestCase::from_number(17), Some(TestCase::FailoverRecovery));
        assert_eq!(TestCase::from_number(18), None);
    }

    #[test]
    fn test_all_cases() {
        let all = TestCase::all();
        assert_eq!(all.len(), 17);
    }

    #[test]
    fn test_result_creation() {
        let result = TestResult::pass(TestCase::HostRouting, 100);
        assert!(result.status.is_success());
        assert_eq!(result.duration_ms, 100);
    }

    #[test]
    fn test_round_summary() {
        let results = vec![
            TestResult::pass(TestCase::HostRouting, 100),
            TestResult::fail(TestCase::PathRouting, 50, "Path not found"),
            TestResult::skip(TestCase::GrpcRouting, "gRPC not configured"),
        ];

        let summary = TestRoundSummary::new(1, "nginx", results);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.skipped, 1);
    }
}
