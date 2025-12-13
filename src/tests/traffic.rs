//! Traffic management tests for Gateway API
//!
//! Tests 7-10: Canary Traffic, Rate Limiting, Timeout & Retry, Session Affinity

#![allow(dead_code)]

use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info};

use crate::http::HttpClient;
use crate::models::{TestCase, TestResult, TestStatus};

/// Test 7: Canary Traffic (Weighted Routing)
#[derive(Clone, Debug)]
pub struct CanaryTrafficTest {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub path: String,
    pub weights: Vec<WeightedBackend>,
    pub sample_size: usize,
    pub tolerance_percent: f64,
}

#[derive(Clone, Debug)]
pub struct WeightedBackend {
    pub name: String,
    pub weight: u32,
}

impl CanaryTrafficTest {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            path: "/".to_string(),
            weights: Vec::new(),
            sample_size: 100,
            tolerance_percent: 10.0,
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn add_backend(mut self, name: impl Into<String>, weight: u32) -> Self {
        self.weights.push(WeightedBackend {
            name: name.into(),
            weight,
        });
        self
    }

    pub fn sample_size(mut self, size: usize) -> Self {
        self.sample_size = size;
        self
    }

    pub fn tolerance(mut self, percent: f64) -> Self {
        self.tolerance_percent = percent;
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!(
            "Running Canary Traffic Test with {} samples",
            self.sample_size
        );
        let start = std::time::Instant::now();
        let mut details = Vec::new();

        // Calculate expected percentages
        let total_weight: u32 = self.weights.iter().map(|w| w.weight).sum();
        let expected: HashMap<String, f64> = self
            .weights
            .iter()
            .map(|w| {
                (
                    w.name.clone(),
                    (w.weight as f64 / total_weight as f64) * 100.0,
                )
            })
            .collect();

        // Count actual distribution
        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut failures = 0;

        for _ in 0..self.sample_size {
            let response = client
                .test_path_routing(&self.gateway_ip, self.gateway_port, &self.path)
                .await;

            match response {
                Ok(resp) if resp.is_success() => {
                    // Identify which backend responded
                    for backend in &self.weights {
                        if resp.body_contains(&backend.name) {
                            *counts.entry(backend.name.clone()).or_insert(0) += 1;
                            break;
                        }
                    }
                }
                _ => {
                    failures += 1;
                }
            }
        }

        // Analyze distribution
        let successful = self.sample_size - failures;
        let mut all_within_tolerance = true;

        for backend in &self.weights {
            let count = counts.get(&backend.name).copied().unwrap_or(0);
            let actual_percent = if successful > 0 {
                (count as f64 / successful as f64) * 100.0
            } else {
                0.0
            };
            let expected_percent = expected.get(&backend.name).copied().unwrap_or(0.0);
            let diff = (actual_percent - expected_percent).abs();

            if diff <= self.tolerance_percent {
                details.push(format!(
                    "✓ {} actual: {:.1}%, expected: {:.1}% (diff: {:.1}%)",
                    backend.name, actual_percent, expected_percent, diff
                ));
            } else {
                all_within_tolerance = false;
                details.push(format!(
                    "✗ {} actual: {:.1}%, expected: {:.1}% (diff: {:.1}% > tolerance {}%)",
                    backend.name, actual_percent, expected_percent, diff, self.tolerance_percent
                ));
            }
        }

        if failures > 0 {
            details.push(format!("⚠ {failures} requests failed"));
        }

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::CanaryTraffic,
            status: if all_within_tolerance && failures < self.sample_size / 10 {
                TestStatus::Pass
            } else {
                TestStatus::Fail
            },
            duration_ms: duration.as_millis() as u64,
            message: Some(details.join("\n")),
            details: None,
        })
    }
}

/// Test 8: Rate Limiting
#[derive(Clone, Debug)]
pub struct RateLimitingTest {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub path: String,
    pub requests_per_second: u32,
    pub burst_size: u32,
    pub test_duration_secs: u64,
}

impl RateLimitingTest {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            path: "/rate-limited".to_string(),
            requests_per_second: 10,
            burst_size: 5,
            test_duration_secs: 5,
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn with_limit(mut self, rps: u32, burst: u32) -> Self {
        self.requests_per_second = rps;
        self.burst_size = burst;
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!(
            "Running Rate Limiting Test (limit: {} rps, burst: {})",
            self.requests_per_second, self.burst_size
        );
        let start = std::time::Instant::now();
        let mut details = Vec::new();

        let total_requests =
            (self.requests_per_second as u64 * self.test_duration_secs * 2) as usize;
        let mut success_count = 0;
        let mut rate_limited_count = 0;
        let mut error_count = 0;

        // Send requests as fast as possible to trigger rate limiting
        for i in 0..total_requests {
            let response = client
                .test_path_routing(&self.gateway_ip, self.gateway_port, &self.path)
                .await;

            match response {
                Ok(resp) => {
                    if resp.is_success() {
                        success_count += 1;
                    } else if resp.status_code == 429 {
                        rate_limited_count += 1;
                    } else {
                        error_count += 1;
                    }
                }
                Err(_) => {
                    error_count += 1;
                }
            }

            // Small delay between requests
            if i % 10 == 0 {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }

        details.push(format!("Total requests: {total_requests}"));
        details.push(format!("Successful: {success_count}"));
        details.push(format!("Rate limited (429): {rate_limited_count}"));
        details.push(format!("Errors: {error_count}"));

        // Rate limiting is working if we got some 429s
        let status = if rate_limited_count > 0 {
            details.push("✓ Rate limiting is active".to_string());
            TestStatus::Pass
        } else if success_count == total_requests {
            details.push("✗ No rate limiting detected (all requests succeeded)".to_string());
            TestStatus::Fail
        } else {
            details.push("? Rate limiting status unclear".to_string());
            TestStatus::Fail
        };

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::RateLimiting,
            status,
            duration_ms: duration.as_millis() as u64,
            message: Some(details.join("\n")),
            details: None,
        })
    }
}

/// Test 9: Timeout & Retry
#[derive(Clone, Debug)]
pub struct TimeoutRetryTest {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub slow_path: String,
    pub expected_timeout_ms: u64,
    pub retry_path: String,
    pub expected_retries: u32,
}

impl TimeoutRetryTest {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            slow_path: "/slow".to_string(),
            expected_timeout_ms: 5000,
            retry_path: "/flaky".to_string(),
            expected_retries: 3,
        }
    }

    pub fn with_timeout_path(mut self, path: impl Into<String>, timeout_ms: u64) -> Self {
        self.slow_path = path.into();
        self.expected_timeout_ms = timeout_ms;
        self
    }

    pub fn with_retry_path(mut self, path: impl Into<String>, retries: u32) -> Self {
        self.retry_path = path.into();
        self.expected_retries = retries;
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!("Running Timeout & Retry Test");
        let start = std::time::Instant::now();
        let mut details = Vec::new();
        let mut all_passed = true;

        // Test timeout
        debug!("Testing timeout on path: {}", self.slow_path);
        let timeout_start = std::time::Instant::now();
        let response = client
            .test_path_routing(&self.gateway_ip, self.gateway_port, &self.slow_path)
            .await;

        let timeout_elapsed = timeout_start.elapsed().as_millis() as u64;

        match response {
            Ok(resp) => {
                if resp.status_code == 504 || resp.status_code == 408 {
                    details.push(format!(
                        "✓ Timeout triggered (status {}, took {}ms)",
                        resp.status_code, timeout_elapsed
                    ));
                } else if timeout_elapsed >= self.expected_timeout_ms {
                    details.push(format!("✓ Request timed out after {timeout_elapsed}ms"));
                } else {
                    all_passed = false;
                    details.push(format!(
                        "✗ Expected timeout but got status {} in {}ms",
                        resp.status_code, timeout_elapsed
                    ));
                }
            }
            Err(e) => {
                let err_str = e.to_string().to_lowercase();
                if err_str.contains("timeout") {
                    details.push(format!("✓ Request timed out: {e}"));
                } else {
                    all_passed = false;
                    details.push(format!("✗ Unexpected error: {e}"));
                }
            }
        }

        // Test retry (by checking if eventually succeeds on flaky endpoint)
        debug!("Testing retry on path: {}", self.retry_path);
        let response = client
            .test_path_routing(&self.gateway_ip, self.gateway_port, &self.retry_path)
            .await;

        match response {
            Ok(resp) => {
                if resp.is_success() {
                    // Check if response indicates retries occurred
                    let retry_count_header = resp.get_header("x-retry-count");
                    if let Some(count) = retry_count_header {
                        details.push(format!("✓ Retries working (count: {count})"));
                    } else {
                        details.push("✓ Retry endpoint responded successfully".to_string());
                    }
                } else {
                    details.push(format!(
                        "⚠ Retry endpoint returned status {} (may still be valid)",
                        resp.status_code
                    ));
                }
            }
            Err(e) => {
                all_passed = false;
                details.push(format!("✗ Retry test failed: {e}"));
            }
        }

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::TimeoutRetry,
            status: if all_passed {
                TestStatus::Pass
            } else {
                TestStatus::Fail
            },
            duration_ms: duration.as_millis() as u64,
            message: Some(details.join("\n")),
            details: None,
        })
    }
}

/// Test 10: Session Affinity
#[derive(Clone, Debug)]
pub struct SessionAffinityTest {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub path: String,
    pub num_requests: usize,
    pub affinity_type: AffinityType,
}

#[derive(Clone, Debug)]
pub enum AffinityType {
    Cookie,
    Header,
    SourceIp,
}

impl SessionAffinityTest {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            path: "/session".to_string(),
            num_requests: 10,
            affinity_type: AffinityType::Cookie,
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn with_affinity_type(mut self, affinity_type: AffinityType) -> Self {
        self.affinity_type = affinity_type;
        self
    }

    pub fn num_requests(mut self, count: usize) -> Self {
        self.num_requests = count;
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!("Running Session Affinity Test ({:?})", self.affinity_type);
        let start = std::time::Instant::now();
        let mut details = Vec::new();

        // First request to get session cookie
        let first_response = client
            .test_path_routing(&self.gateway_ip, self.gateway_port, &self.path)
            .await?;

        if !first_response.is_success() {
            return Ok(TestResult {
                test_case: TestCase::SessionAffinity,
                status: TestStatus::Fail,
                duration_ms: start.elapsed().as_millis() as u64,
                message: Some(format!(
                    "First request failed with status {}",
                    first_response.status_code
                )),
                details: None,
            });
        }

        // Extract session info from first response
        let session_cookie = first_response.get_header("set-cookie");
        let first_backend = extract_backend_id(&first_response.body);

        if first_backend.is_none() {
            details.push("⚠ Could not identify backend from response".to_string());
        }

        // Subsequent requests should go to same backend
        let mut same_backend_count = 0;
        let mut different_backend_count = 0;
        let mut headers = HashMap::new();

        // Include cookie if available
        if let Some(cookie) = session_cookie {
            headers.insert("Cookie".to_string(), cookie.clone());
        }

        for _ in 1..self.num_requests {
            let response = client
                .get_with_headers(
                    &format!(
                        "http://{}:{}{}",
                        self.gateway_ip, self.gateway_port, self.path
                    ),
                    headers.clone(),
                )
                .await;

            if let Ok(resp) = response {
                if resp.is_success() {
                    let backend = extract_backend_id(&resp.body);
                    if backend == first_backend {
                        same_backend_count += 1;
                    } else {
                        different_backend_count += 1;
                    }
                }
            }
        }

        details.push(format!("Same backend: {same_backend_count}"));
        details.push(format!("Different backend: {different_backend_count}"));

        // Session affinity is working if most requests go to same backend
        let affinity_rate = same_backend_count as f64 / (self.num_requests - 1) as f64;
        let status = if affinity_rate >= 0.9 {
            details.push(format!(
                "✓ Session affinity working ({:.1}% consistency)",
                affinity_rate * 100.0
            ));
            TestStatus::Pass
        } else if affinity_rate >= 0.5 {
            details.push(format!(
                "⚠ Partial session affinity ({:.1}% consistency)",
                affinity_rate * 100.0
            ));
            TestStatus::Fail
        } else {
            details.push(format!(
                "✗ No session affinity detected ({:.1}% consistency)",
                affinity_rate * 100.0
            ));
            TestStatus::Fail
        };

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::SessionAffinity,
            status,
            duration_ms: duration.as_millis() as u64,
            message: Some(details.join("\n")),
            details: None,
        })
    }
}

/// Extract backend identifier from response body
fn extract_backend_id(body: &str) -> Option<String> {
    // Try to find backend identifier patterns
    // Common patterns: "pod-xxx", "backend-xxx", "server: xxx"
    if let Some(start) = body.find("pod-") {
        let end = body[start..]
            .find(|c: char| !c.is_alphanumeric() && c != '-')
            .map(|i| start + i)
            .unwrap_or(body.len().min(start + 50));
        return Some(body[start..end].to_string());
    }
    if let Some(start) = body.find("backend-") {
        let end = body[start..]
            .find(|c: char| !c.is_alphanumeric() && c != '-')
            .map(|i| start + i)
            .unwrap_or(body.len().min(start + 50));
        return Some(body[start..end].to_string());
    }
    None
}

/// Combined traffic test runner
pub struct TrafficTestSuite {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub client: HttpClient,
}

impl TrafficTestSuite {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Result<Self> {
        Ok(Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            client: HttpClient::new()?,
        })
    }

    pub async fn run_all(&self) -> Result<Vec<TestResult>> {
        let mut results = Vec::new();

        // Canary traffic test
        let canary_test = CanaryTrafficTest::new(&self.gateway_ip, self.gateway_port)
            .add_backend("stable", 90)
            .add_backend("canary", 10)
            .sample_size(100);
        results.push(canary_test.run(&self.client).await?);

        // Rate limiting test
        let rate_test =
            RateLimitingTest::new(&self.gateway_ip, self.gateway_port).with_limit(10, 5);
        results.push(rate_test.run(&self.client).await?);

        // Timeout & retry test
        let timeout_test = TimeoutRetryTest::new(&self.gateway_ip, self.gateway_port);
        results.push(timeout_test.run(&self.client).await?);

        // Session affinity test
        let session_test =
            SessionAffinityTest::new(&self.gateway_ip, self.gateway_port).num_requests(10);
        results.push(session_test.run(&self.client).await?);

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canary_builder() {
        let test = CanaryTrafficTest::new("10.0.0.1", 80)
            .add_backend("stable", 90)
            .add_backend("canary", 10)
            .sample_size(200)
            .tolerance(15.0);

        assert_eq!(test.weights.len(), 2);
        assert_eq!(test.sample_size, 200);
        assert_eq!(test.tolerance_percent, 15.0);
    }

    #[test]
    fn test_rate_limiting_builder() {
        let test = RateLimitingTest::new("10.0.0.1", 80)
            .with_path("/api")
            .with_limit(100, 10);

        assert_eq!(test.requests_per_second, 100);
        assert_eq!(test.burst_size, 10);
    }

    #[test]
    fn test_session_affinity_builder() {
        let test = SessionAffinityTest::new("10.0.0.1", 80)
            .with_affinity_type(AffinityType::Cookie)
            .num_requests(20);

        assert_eq!(test.num_requests, 20);
    }

    #[test]
    fn test_extract_backend_id() {
        assert_eq!(
            extract_backend_id("Server: pod-abc123-xyz"),
            Some("pod-abc123-xyz".to_string())
        );
        assert_eq!(
            extract_backend_id("backend-v1 responding"),
            Some("backend-v1".to_string())
        );
        assert_eq!(extract_backend_id("no backend info"), None);
    }
}
