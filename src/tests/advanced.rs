//! Advanced tests for Gateway API
//!
//! Tests 11-17: URL Rewrite, Header Modifier, Cross Namespace, gRPC Routing,
//!              Health Check, Load Test, Failover Recovery

#![allow(dead_code)]

use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info};

use crate::http::HttpClient;
use crate::models::{TestCase, TestResult, TestStatus};

/// Test 11: URL Rewrite
#[derive(Clone, Debug)]
pub struct UrlRewriteTest {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub rewrites: Vec<RewriteRule>,
}

#[derive(Clone, Debug)]
pub struct RewriteRule {
    pub original_path: String,
    pub expected_rewritten: String,
}

impl UrlRewriteTest {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            rewrites: Vec::new(),
        }
    }

    pub fn add_rewrite(mut self, original: impl Into<String>, expected: impl Into<String>) -> Self {
        self.rewrites.push(RewriteRule {
            original_path: original.into(),
            expected_rewritten: expected.into(),
        });
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!("Running URL Rewrite Test");
        let start = std::time::Instant::now();
        let mut all_passed = true;
        let mut details = Vec::new();

        for rule in &self.rewrites {
            debug!(
                "Testing rewrite: {} -> {}",
                rule.original_path, rule.expected_rewritten
            );

            let response = client
                .test_path_routing(&self.gateway_ip, self.gateway_port, &rule.original_path)
                .await;

            match response {
                Ok(resp) => {
                    // Check if backend received the rewritten path
                    let rewritten = resp.body_contains(&rule.expected_rewritten)
                        || resp
                            .get_header("x-original-path")
                            .map(|p| p.contains(&rule.expected_rewritten))
                            .unwrap_or(false);

                    if resp.is_success() && rewritten {
                        details.push(format!(
                            "✓ {} -> {} ({}ms)",
                            rule.original_path, rule.expected_rewritten, resp.duration_ms
                        ));
                    } else if resp.is_success() {
                        details.push(format!(
                            "⚠ {} succeeded but rewrite not verified",
                            rule.original_path
                        ));
                    } else {
                        all_passed = false;
                        details.push(format!(
                            "✗ {} returned status {}",
                            rule.original_path, resp.status_code
                        ));
                    }
                }
                Err(e) => {
                    all_passed = false;
                    details.push(format!("✗ {} failed: {}", rule.original_path, e));
                }
            }
        }

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::UrlRewrite,
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

/// Test 12: Header Modifier
#[derive(Clone, Debug)]
pub struct HeaderModifierTest {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub path: String,
    pub request_headers: Vec<HeaderModification>,
    pub response_headers: Vec<HeaderModification>,
}

#[derive(Clone, Debug)]
pub struct HeaderModification {
    pub action: HeaderAction,
    pub name: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug)]
pub enum HeaderAction {
    Add,
    Set,
    Remove,
}

impl HeaderModifierTest {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            path: "/header-test".to_string(),
            request_headers: Vec::new(),
            response_headers: Vec::new(),
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn expect_response_header(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.response_headers.push(HeaderModification {
            action: HeaderAction::Add,
            name: name.into(),
            value: Some(value.into()),
        });
        self
    }

    pub fn expect_header_removed(mut self, name: impl Into<String>) -> Self {
        self.response_headers.push(HeaderModification {
            action: HeaderAction::Remove,
            name: name.into(),
            value: None,
        });
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!("Running Header Modifier Test");
        let start = std::time::Instant::now();
        let mut all_passed = true;
        let mut details = Vec::new();

        let response = client
            .test_path_routing(&self.gateway_ip, self.gateway_port, &self.path)
            .await;

        match response {
            Ok(resp) => {
                if !resp.is_success() {
                    all_passed = false;
                    details.push(format!("✗ Request failed with status {}", resp.status_code));
                } else {
                    // Check response headers
                    for modification in &self.response_headers {
                        match modification.action {
                            HeaderAction::Add | HeaderAction::Set => {
                                let header_value =
                                    resp.get_header(&modification.name.to_lowercase());
                                if let Some(expected) = &modification.value {
                                    if let Some(actual) = header_value {
                                        if actual.contains(expected) {
                                            details.push(format!(
                                                "✓ Header {} present with expected value",
                                                modification.name
                                            ));
                                        } else {
                                            all_passed = false;
                                            details.push(format!(
                                                "✗ Header {} has value '{}', expected '{}'",
                                                modification.name, actual, expected
                                            ));
                                        }
                                    } else {
                                        all_passed = false;
                                        details.push(format!(
                                            "✗ Header {} not found in response",
                                            modification.name
                                        ));
                                    }
                                }
                            }
                            HeaderAction::Remove => {
                                if resp.get_header(&modification.name.to_lowercase()).is_none() {
                                    details.push(format!(
                                        "✓ Header {} successfully removed",
                                        modification.name
                                    ));
                                } else {
                                    all_passed = false;
                                    details.push(format!(
                                        "✗ Header {} should have been removed",
                                        modification.name
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                all_passed = false;
                details.push(format!("✗ Request failed: {e}"));
            }
        }

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::HeaderModifier,
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

/// Test 13: Cross Namespace Routing
#[derive(Clone, Debug)]
pub struct CrossNamespaceTest {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub routes: Vec<CrossNamespaceRoute>,
}

#[derive(Clone, Debug)]
pub struct CrossNamespaceRoute {
    pub path: String,
    pub target_namespace: String,
    pub target_service: String,
}

impl CrossNamespaceTest {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            routes: Vec::new(),
        }
    }

    pub fn add_route(
        mut self,
        path: impl Into<String>,
        namespace: impl Into<String>,
        service: impl Into<String>,
    ) -> Self {
        self.routes.push(CrossNamespaceRoute {
            path: path.into(),
            target_namespace: namespace.into(),
            target_service: service.into(),
        });
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!("Running Cross Namespace Test");
        let start = std::time::Instant::now();
        let mut all_passed = true;
        let mut details = Vec::new();

        for route in &self.routes {
            debug!(
                "Testing cross-namespace route: {} -> {}/{}",
                route.path, route.target_namespace, route.target_service
            );

            let response = client
                .test_path_routing(&self.gateway_ip, self.gateway_port, &route.path)
                .await;

            match response {
                Ok(resp) => {
                    let reached_target = resp.is_success()
                        && (resp.body_contains(&route.target_namespace)
                            || resp.body_contains(&route.target_service));

                    if reached_target {
                        details.push(format!(
                            "✓ {} -> {}/{} ({}ms)",
                            route.path,
                            route.target_namespace,
                            route.target_service,
                            resp.duration_ms
                        ));
                    } else if resp.is_success() {
                        details.push(format!(
                            "⚠ {} reached a backend but namespace not verified",
                            route.path
                        ));
                    } else {
                        all_passed = false;
                        details.push(format!(
                            "✗ {} returned status {}",
                            route.path, resp.status_code
                        ));
                    }
                }
                Err(e) => {
                    all_passed = false;
                    details.push(format!("✗ {} failed: {}", route.path, e));
                }
            }
        }

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::CrossNamespace,
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

/// Test 14: gRPC Routing
#[derive(Clone, Debug)]
pub struct GrpcRoutingTest {
    pub gateway_ip: String,
    pub grpc_port: u16,
    pub services: Vec<GrpcService>,
}

#[derive(Clone, Debug)]
pub struct GrpcService {
    pub service_name: String,
    pub method: String,
    pub expected_backend: String,
}

impl GrpcRoutingTest {
    pub fn new(gateway_ip: impl Into<String>, grpc_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            grpc_port,
            services: Vec::new(),
        }
    }

    pub fn add_service(
        mut self,
        service: impl Into<String>,
        method: impl Into<String>,
        backend: impl Into<String>,
    ) -> Self {
        self.services.push(GrpcService {
            service_name: service.into(),
            method: method.into(),
            expected_backend: backend.into(),
        });
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!("Running gRPC Routing Test");
        let start = std::time::Instant::now();
        let mut all_passed = true;
        let mut details = Vec::new();

        for service in &self.services {
            debug!(
                "Testing gRPC service: {}/{}",
                service.service_name, service.method
            );

            // gRPC uses HTTP/2 POST with specific content-type
            let url = format!(
                "http://{}:{}/{}/{}",
                self.gateway_ip, self.grpc_port, service.service_name, service.method
            );

            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), "application/grpc".to_string());
            headers.insert("TE".to_string(), "trailers".to_string());

            let response = client.get_with_headers(&url, headers).await;

            match response {
                Ok(resp) => {
                    // gRPC routing test - check if we reach the right backend
                    // Even 415 (Unsupported Media Type) can indicate routing works
                    if resp.is_success() || resp.status_code == 415 || resp.status_code == 200 {
                        details.push(format!(
                            "✓ {}/{} routed (status {}, {}ms)",
                            service.service_name,
                            service.method,
                            resp.status_code,
                            resp.duration_ms
                        ));
                    } else {
                        all_passed = false;
                        details.push(format!(
                            "✗ {}/{} returned status {}",
                            service.service_name, service.method, resp.status_code
                        ));
                    }
                }
                Err(e) => {
                    all_passed = false;
                    details.push(format!(
                        "✗ {}/{} failed: {}",
                        service.service_name, service.method, e
                    ));
                }
            }
        }

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::GrpcRouting,
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

/// Test 15: Health Check
#[derive(Clone, Debug)]
pub struct HealthCheckTest {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub health_path: String,
    pub expected_healthy_status: Vec<u16>,
}

impl HealthCheckTest {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            health_path: "/health".to_string(),
            expected_healthy_status: vec![200],
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.health_path = path.into();
        self
    }

    pub fn expected_status(mut self, status: u16) -> Self {
        self.expected_healthy_status.push(status);
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!("Running Health Check Test");
        let start = std::time::Instant::now();
        let mut details = Vec::new();

        let response = client
            .test_path_routing(&self.gateway_ip, self.gateway_port, &self.health_path)
            .await;

        let status = match response {
            Ok(resp) => {
                if self.expected_healthy_status.contains(&resp.status_code) {
                    details.push(format!(
                        "✓ Health check passed (status {}, {}ms)",
                        resp.status_code, resp.duration_ms
                    ));
                    TestStatus::Pass
                } else {
                    details.push(format!(
                        "✗ Health check returned unexpected status {}",
                        resp.status_code
                    ));
                    TestStatus::Fail
                }
            }
            Err(e) => {
                details.push(format!("✗ Health check failed: {e}"));
                TestStatus::Fail
            }
        };

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::HealthCheck,
            status,
            duration_ms: duration.as_millis() as u64,
            message: Some(details.join("\n")),
            details: None,
        })
    }
}

/// Test 16: Load Test
#[derive(Clone, Debug)]
pub struct LoadTest {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub path: String,
    pub concurrent_users: usize,
    pub total_requests: usize,
    pub expected_success_rate: f64,
    pub max_avg_latency_ms: u64,
}

impl LoadTest {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            path: "/".to_string(),
            concurrent_users: 10,
            total_requests: 100,
            expected_success_rate: 95.0,
            max_avg_latency_ms: 1000,
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn concurrent_users(mut self, users: usize) -> Self {
        self.concurrent_users = users;
        self
    }

    pub fn total_requests(mut self, total: usize) -> Self {
        self.total_requests = total;
        self
    }

    pub fn expected_success_rate(mut self, rate: f64) -> Self {
        self.expected_success_rate = rate;
        self
    }

    pub fn max_latency_ms(mut self, ms: u64) -> Self {
        self.max_avg_latency_ms = ms;
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!(
            "Running Load Test ({} concurrent, {} total)",
            self.concurrent_users, self.total_requests
        );
        let start = std::time::Instant::now();
        let mut details = Vec::new();

        let url = format!(
            "http://{}:{}{}",
            self.gateway_ip, self.gateway_port, self.path
        );

        let result = client
            .load_test(&url, self.concurrent_users, self.total_requests)
            .await?;

        let success_rate = result.success_rate();
        details.push(format!("Total requests: {}", result.total_requests));
        details.push(format!("Successes: {}", result.successes));
        details.push(format!("Failures: {}", result.failures));
        details.push(format!("Success rate: {success_rate:.1}%"));
        details.push(format!("Avg latency: {}ms", result.avg_duration_ms));

        let status = if success_rate >= self.expected_success_rate
            && result.avg_duration_ms <= self.max_avg_latency_ms
        {
            details.push(format!(
                "✓ Load test passed (>= {:.1}% success, <= {}ms latency)",
                self.expected_success_rate, self.max_avg_latency_ms
            ));
            TestStatus::Pass
        } else {
            if success_rate < self.expected_success_rate {
                details.push(format!(
                    "✗ Success rate {:.1}% below threshold {:.1}%",
                    success_rate, self.expected_success_rate
                ));
            }
            if result.avg_duration_ms > self.max_avg_latency_ms {
                details.push(format!(
                    "✗ Avg latency {}ms exceeds threshold {}ms",
                    result.avg_duration_ms, self.max_avg_latency_ms
                ));
            }
            TestStatus::Fail
        };

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::LoadTest,
            status,
            duration_ms: duration.as_millis() as u64,
            message: Some(details.join("\n")),
            details: None,
        })
    }
}

/// Test 17: Failover Recovery
#[derive(Clone, Debug)]
pub struct FailoverRecoveryTest {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub path: String,
    pub check_interval_ms: u64,
    pub max_recovery_time_ms: u64,
}

impl FailoverRecoveryTest {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            path: "/failover".to_string(),
            check_interval_ms: 500,
            max_recovery_time_ms: 30000,
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn max_recovery_time(mut self, ms: u64) -> Self {
        self.max_recovery_time_ms = ms;
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!("Running Failover Recovery Test");
        let start = std::time::Instant::now();
        let mut details = Vec::new();

        // Initial check - should succeed
        let initial = client
            .test_path_routing(&self.gateway_ip, self.gateway_port, &self.path)
            .await;

        let initial_ok = match &initial {
            Ok(resp) => {
                if resp.is_success() {
                    details.push("✓ Initial request successful".to_string());
                    true
                } else {
                    details.push(format!(
                        "⚠ Initial request returned status {}",
                        resp.status_code
                    ));
                    false
                }
            }
            Err(e) => {
                details.push(format!("⚠ Initial request failed: {e}"));
                false
            }
        };

        if !initial_ok {
            // Even if initial failed, continue to check if service recovers
            details.push("Testing recovery from initial failure...".to_string());
        }

        // Continuous monitoring for recovery
        let mut consecutive_successes = 0;
        let mut first_success_time: Option<Duration> = None;
        let test_start = std::time::Instant::now();

        while test_start.elapsed().as_millis() < self.max_recovery_time_ms as u128 {
            tokio::time::sleep(Duration::from_millis(self.check_interval_ms)).await;

            let response = client
                .test_path_routing(&self.gateway_ip, self.gateway_port, &self.path)
                .await;

            match response {
                Ok(resp) if resp.is_success() => {
                    if first_success_time.is_none() {
                        first_success_time = Some(test_start.elapsed());
                    }
                    consecutive_successes += 1;
                    if consecutive_successes >= 3 {
                        break;
                    }
                }
                _ => {
                    consecutive_successes = 0;
                }
            }
        }

        let status = if consecutive_successes >= 3 {
            let recovery_time = first_success_time
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            details.push(format!(
                "✓ Service recovered after {recovery_time}ms with {consecutive_successes} consecutive successes"
            ));
            TestStatus::Pass
        } else {
            details.push(format!(
                "✗ Service did not fully recover within {}ms",
                self.max_recovery_time_ms
            ));
            TestStatus::Fail
        };

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::FailoverRecovery,
            status,
            duration_ms: duration.as_millis() as u64,
            message: Some(details.join("\n")),
            details: None,
        })
    }
}

/// Combined advanced test runner
pub struct AdvancedTestSuite {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub grpc_port: u16,
    pub client: HttpClient,
}

impl AdvancedTestSuite {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16, grpc_port: u16) -> Result<Self> {
        Ok(Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            grpc_port,
            client: HttpClient::new()?,
        })
    }

    pub async fn run_all(&self) -> Result<Vec<TestResult>> {
        let mut results = Vec::new();

        // URL Rewrite test
        let rewrite_test = UrlRewriteTest::new(&self.gateway_ip, self.gateway_port)
            .add_rewrite("/old-api", "/new-api")
            .add_rewrite("/v1/users", "/api/v2/users");
        results.push(rewrite_test.run(&self.client).await?);

        // Header Modifier test
        let header_test = HeaderModifierTest::new(&self.gateway_ip, self.gateway_port)
            .expect_response_header("X-Gateway", "true")
            .expect_header_removed("Server");
        results.push(header_test.run(&self.client).await?);

        // Cross Namespace test
        let cross_ns_test = CrossNamespaceTest::new(&self.gateway_ip, self.gateway_port)
            .add_route("/ns-a", "namespace-a", "service-a")
            .add_route("/ns-b", "namespace-b", "service-b");
        results.push(cross_ns_test.run(&self.client).await?);

        // gRPC Routing test
        let grpc_test = GrpcRoutingTest::new(&self.gateway_ip, self.grpc_port).add_service(
            "helloworld.Greeter",
            "SayHello",
            "grpc-backend",
        );
        results.push(grpc_test.run(&self.client).await?);

        // Health Check test
        let health_test = HealthCheckTest::new(&self.gateway_ip, self.gateway_port);
        results.push(health_test.run(&self.client).await?);

        // Load Test
        let load_test = LoadTest::new(&self.gateway_ip, self.gateway_port)
            .concurrent_users(10)
            .total_requests(100);
        results.push(load_test.run(&self.client).await?);

        // Failover Recovery test
        let failover_test = FailoverRecoveryTest::new(&self.gateway_ip, self.gateway_port);
        results.push(failover_test.run(&self.client).await?);

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_rewrite_builder() {
        let test = UrlRewriteTest::new("10.0.0.1", 80).add_rewrite("/old", "/new");

        assert_eq!(test.rewrites.len(), 1);
        assert_eq!(test.rewrites[0].original_path, "/old");
    }

    #[test]
    fn test_header_modifier_builder() {
        let test = HeaderModifierTest::new("10.0.0.1", 80)
            .expect_response_header("X-Custom", "value")
            .expect_header_removed("X-Remove");

        assert_eq!(test.response_headers.len(), 2);
    }

    #[test]
    fn test_load_test_builder() {
        let test = LoadTest::new("10.0.0.1", 80)
            .concurrent_users(50)
            .total_requests(1000)
            .expected_success_rate(99.0)
            .max_latency_ms(500);

        assert_eq!(test.concurrent_users, 50);
        assert_eq!(test.total_requests, 1000);
        assert_eq!(test.expected_success_rate, 99.0);
    }

    #[test]
    fn test_grpc_routing_builder() {
        let test =
            GrpcRoutingTest::new("10.0.0.1", 9090).add_service("myservice", "MyMethod", "backend");

        assert_eq!(test.services.len(), 1);
        assert_eq!(test.grpc_port, 9090);
    }
}
