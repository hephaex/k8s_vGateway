//! Gateway API test implementations
//!
//! This module contains all 17 test cases for Gateway API validation.
//!
//! ## Test Categories
//!
//! ### Routing Tests (1-3)
//! - Host Routing
//! - Path Routing
//! - Header Routing
//!
//! ### TLS Tests (4-6)
//! - TLS Termination
//! - HTTPS Redirect
//! - Backend TLS (mTLS)
//!
//! ### Traffic Management Tests (7-10)
//! - Canary Traffic (Weighted Routing)
//! - Rate Limiting
//! - Timeout & Retry
//! - Session Affinity
//!
//! ### Advanced Tests (11-17)
//! - URL Rewrite
//! - Header Modifier
//! - Cross Namespace
//! - gRPC Routing
//! - Health Check
//! - Load Test
//! - Failover Recovery

#![allow(dead_code)]

mod advanced;
mod routing;
mod tls;
mod traffic;

// Re-export routing tests
pub use routing::{HeaderRoutingTest, HostRoutingTest, PathRoutingTest, RoutingTestSuite};

// Re-export TLS tests
pub use tls::{BackendTlsTest, HttpsRedirectTest, TlsTerminationTest, TlsTestSuite};

// Re-export traffic tests
pub use traffic::{
    CanaryTrafficTest, RateLimitingTest, SessionAffinityTest, TimeoutRetryTest, TrafficTestSuite,
};

// Re-export advanced tests
pub use advanced::{
    AdvancedTestSuite, CrossNamespaceTest, FailoverRecoveryTest, GrpcRoutingTest,
    HeaderModifierTest, HealthCheckTest, LoadTest, UrlRewriteTest,
};

use crate::http::HttpClient;
use crate::models::{TestCase, TestResult};
use anyhow::Result;

/// Run all 17 test cases
pub async fn run_all_tests(
    gateway_ip: &str,
    http_port: u16,
    https_port: u16,
    grpc_port: u16,
    hostname: &str,
) -> Result<Vec<TestResult>> {
    let _client = HttpClient::new()?;
    let mut results = Vec::new();

    // Routing tests (1-3)
    let routing_suite = RoutingTestSuite::new(gateway_ip, http_port)?;
    results.extend(routing_suite.run_all().await?);

    // TLS tests (4-6)
    let tls_suite = TlsTestSuite::new(gateway_ip, http_port, https_port, hostname)?;
    results.extend(tls_suite.run_all().await?);

    // Traffic tests (7-10)
    let traffic_suite = TrafficTestSuite::new(gateway_ip, http_port)?;
    results.extend(traffic_suite.run_all().await?);

    // Advanced tests (11-17)
    let advanced_suite = AdvancedTestSuite::new(gateway_ip, http_port, grpc_port)?;
    results.extend(advanced_suite.run_all().await?);

    Ok(results)
}

/// Run a specific test case
pub async fn run_test(
    test_case: TestCase,
    gateway_ip: &str,
    http_port: u16,
    https_port: u16,
    grpc_port: u16,
    hostname: &str,
) -> Result<TestResult> {
    let client = HttpClient::new()?;

    match test_case {
        TestCase::HostRouting => {
            HostRoutingTest::new(gateway_ip, http_port)
                .add_hostname("app1.example.com", "app1")
                .add_hostname("app2.example.com", "app2")
                .run(&client)
                .await
        }
        TestCase::PathRouting => {
            PathRoutingTest::new(gateway_ip, http_port)
                .add_prefix("/api/v1", "api-v1")
                .add_prefix("/api/v2", "api-v2")
                .run(&client)
                .await
        }
        TestCase::HeaderRouting => {
            HeaderRoutingTest::new(gateway_ip, http_port)
                .add_header_rule("X-Version", "v1", "version-v1")
                .run(&client)
                .await
        }
        TestCase::TlsTermination => {
            TlsTerminationTest::new(gateway_ip, https_port, hostname)
                .run(&client)
                .await
        }
        TestCase::HttpsRedirect => {
            HttpsRedirectTest::new(gateway_ip, http_port, https_port)
                .run(&client)
                .await
        }
        TestCase::BackendTls => {
            BackendTlsTest::new(gateway_ip, https_port)
                .run(&client)
                .await
        }
        TestCase::CanaryTraffic => {
            CanaryTrafficTest::new(gateway_ip, http_port)
                .add_backend("stable", 90)
                .add_backend("canary", 10)
                .run(&client)
                .await
        }
        TestCase::RateLimiting => {
            RateLimitingTest::new(gateway_ip, http_port)
                .run(&client)
                .await
        }
        TestCase::TimeoutRetry => {
            TimeoutRetryTest::new(gateway_ip, http_port)
                .run(&client)
                .await
        }
        TestCase::SessionAffinity => {
            SessionAffinityTest::new(gateway_ip, http_port)
                .run(&client)
                .await
        }
        TestCase::UrlRewrite => {
            UrlRewriteTest::new(gateway_ip, http_port)
                .add_rewrite("/old-api", "/new-api")
                .run(&client)
                .await
        }
        TestCase::HeaderModifier => {
            HeaderModifierTest::new(gateway_ip, http_port)
                .expect_response_header("X-Gateway", "true")
                .run(&client)
                .await
        }
        TestCase::CrossNamespace => {
            CrossNamespaceTest::new(gateway_ip, http_port)
                .add_route("/ns-a", "namespace-a", "service-a")
                .run(&client)
                .await
        }
        TestCase::GrpcRouting => {
            GrpcRoutingTest::new(gateway_ip, grpc_port)
                .add_service("helloworld.Greeter", "SayHello", "grpc-backend")
                .run(&client)
                .await
        }
        TestCase::HealthCheck => {
            HealthCheckTest::new(gateway_ip, http_port)
                .run(&client)
                .await
        }
        TestCase::LoadTest => {
            LoadTest::new(gateway_ip, http_port)
                .concurrent_users(10)
                .total_requests(100)
                .run(&client)
                .await
        }
        TestCase::FailoverRecovery => {
            FailoverRecoveryTest::new(gateway_ip, http_port)
                .run(&client)
                .await
        }
    }
}
