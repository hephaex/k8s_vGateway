//! Routing tests for Gateway API
//!
//! Tests 1-3: Host Routing, Path Routing, Header Routing

#![allow(dead_code)]

use anyhow::Result;
use tracing::{debug, info};

use crate::http::HttpClient;
use crate::models::{TestCase, TestResult, TestStatus};

/// Test 1: Host-based routing
#[derive(Clone, Debug)]
pub struct HostRoutingTest {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub hostnames: Vec<HostnameMapping>,
}

#[derive(Clone, Debug)]
pub struct HostnameMapping {
    pub hostname: String,
    pub expected_backend: String,
}

impl HostRoutingTest {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            hostnames: Vec::new(),
        }
    }

    pub fn add_hostname(
        mut self,
        hostname: impl Into<String>,
        expected_backend: impl Into<String>,
    ) -> Self {
        self.hostnames.push(HostnameMapping {
            hostname: hostname.into(),
            expected_backend: expected_backend.into(),
        });
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!("Running Host Routing Test");
        let start = std::time::Instant::now();
        let mut all_passed = true;
        let mut details = Vec::new();

        for mapping in &self.hostnames {
            debug!("Testing hostname: {}", mapping.hostname);

            let response = client
                .test_host_routing(&self.gateway_ip, self.gateway_port, &mapping.hostname)
                .await;

            match response {
                Ok(resp) => {
                    let passed = resp.is_success() && resp.body_contains(&mapping.expected_backend);

                    if passed {
                        details.push(format!(
                            "✓ {} -> {} ({}ms)",
                            mapping.hostname, mapping.expected_backend, resp.duration_ms
                        ));
                    } else {
                        all_passed = false;
                        details.push(format!(
                            "✗ {} expected {} but got status {}",
                            mapping.hostname, mapping.expected_backend, resp.status_code
                        ));
                    }
                }
                Err(e) => {
                    all_passed = false;
                    details.push(format!("✗ {} failed: {}", mapping.hostname, e));
                }
            }
        }

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::HostRouting,
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

/// Test 2: Path-based routing
#[derive(Clone, Debug)]
pub struct PathRoutingTest {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub paths: Vec<PathMapping>,
}

#[derive(Clone, Debug)]
pub struct PathMapping {
    pub path: String,
    pub expected_backend: String,
    pub match_type: PathMatchType,
}

#[derive(Clone, Debug)]
pub enum PathMatchType {
    Exact,
    Prefix,
    RegularExpression,
}

impl PathRoutingTest {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            paths: Vec::new(),
        }
    }

    pub fn add_path(
        mut self,
        path: impl Into<String>,
        expected_backend: impl Into<String>,
        match_type: PathMatchType,
    ) -> Self {
        self.paths.push(PathMapping {
            path: path.into(),
            expected_backend: expected_backend.into(),
            match_type,
        });
        self
    }

    pub fn add_prefix(
        self,
        prefix: impl Into<String>,
        expected_backend: impl Into<String>,
    ) -> Self {
        self.add_path(prefix, expected_backend, PathMatchType::Prefix)
    }

    pub fn add_exact(self, path: impl Into<String>, expected_backend: impl Into<String>) -> Self {
        self.add_path(path, expected_backend, PathMatchType::Exact)
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!("Running Path Routing Test");
        let start = std::time::Instant::now();
        let mut all_passed = true;
        let mut details = Vec::new();

        for mapping in &self.paths {
            debug!("Testing path: {}", mapping.path);

            let response = client
                .test_path_routing(&self.gateway_ip, self.gateway_port, &mapping.path)
                .await;

            match response {
                Ok(resp) => {
                    let passed = resp.is_success() && resp.body_contains(&mapping.expected_backend);

                    if passed {
                        details.push(format!(
                            "✓ {} -> {} ({}ms)",
                            mapping.path, mapping.expected_backend, resp.duration_ms
                        ));
                    } else {
                        all_passed = false;
                        details.push(format!(
                            "✗ {} expected {} but got status {}",
                            mapping.path, mapping.expected_backend, resp.status_code
                        ));
                    }
                }
                Err(e) => {
                    all_passed = false;
                    details.push(format!("✗ {} failed: {}", mapping.path, e));
                }
            }
        }

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::PathRouting,
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

/// Test 3: Header-based routing
#[derive(Clone, Debug)]
pub struct HeaderRoutingTest {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub header_rules: Vec<HeaderRule>,
}

#[derive(Clone, Debug)]
pub struct HeaderRule {
    pub header_name: String,
    pub header_value: String,
    pub expected_backend: String,
    pub match_type: HeaderMatchType,
}

#[derive(Clone, Debug)]
pub enum HeaderMatchType {
    Exact,
    RegularExpression,
}

impl HeaderRoutingTest {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            header_rules: Vec::new(),
        }
    }

    pub fn add_header_rule(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        expected_backend: impl Into<String>,
    ) -> Self {
        self.header_rules.push(HeaderRule {
            header_name: name.into(),
            header_value: value.into(),
            expected_backend: expected_backend.into(),
            match_type: HeaderMatchType::Exact,
        });
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!("Running Header Routing Test");
        let start = std::time::Instant::now();
        let mut all_passed = true;
        let mut details = Vec::new();

        for rule in &self.header_rules {
            debug!("Testing header: {}={}", rule.header_name, rule.header_value);

            let response = client
                .test_header_routing(
                    &self.gateway_ip,
                    self.gateway_port,
                    &rule.header_name,
                    &rule.header_value,
                )
                .await;

            match response {
                Ok(resp) => {
                    let passed = resp.is_success() && resp.body_contains(&rule.expected_backend);

                    if passed {
                        details.push(format!(
                            "✓ {}={} -> {} ({}ms)",
                            rule.header_name,
                            rule.header_value,
                            rule.expected_backend,
                            resp.duration_ms
                        ));
                    } else {
                        all_passed = false;
                        details.push(format!(
                            "✗ {}={} expected {} but got status {}",
                            rule.header_name,
                            rule.header_value,
                            rule.expected_backend,
                            resp.status_code
                        ));
                    }
                }
                Err(e) => {
                    all_passed = false;
                    details.push(format!(
                        "✗ {}={} failed: {}",
                        rule.header_name, rule.header_value, e
                    ));
                }
            }
        }

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::HeaderRouting,
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

/// Combined routing test runner
pub struct RoutingTestSuite {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub client: HttpClient,
}

impl RoutingTestSuite {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Result<Self> {
        Ok(Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            client: HttpClient::new()?,
        })
    }

    pub async fn run_all(&self) -> Result<Vec<TestResult>> {
        let mut results = Vec::new();

        // Host routing test with default hostnames
        let host_test = HostRoutingTest::new(&self.gateway_ip, self.gateway_port)
            .add_hostname("app1.example.com", "app1")
            .add_hostname("app2.example.com", "app2");
        results.push(host_test.run(&self.client).await?);

        // Path routing test with default paths
        let path_test = PathRoutingTest::new(&self.gateway_ip, self.gateway_port)
            .add_prefix("/api/v1", "api-v1")
            .add_prefix("/api/v2", "api-v2")
            .add_exact("/health", "health");
        results.push(path_test.run(&self.client).await?);

        // Header routing test with default rules
        let header_test = HeaderRoutingTest::new(&self.gateway_ip, self.gateway_port)
            .add_header_rule("X-Version", "v1", "version-v1")
            .add_header_rule("X-Version", "v2", "version-v2");
        results.push(header_test.run(&self.client).await?);

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_routing_builder() {
        let test = HostRoutingTest::new("10.0.0.1", 80)
            .add_hostname("foo.example.com", "foo-backend")
            .add_hostname("bar.example.com", "bar-backend");

        assert_eq!(test.hostnames.len(), 2);
        assert_eq!(test.hostnames[0].hostname, "foo.example.com");
    }

    #[test]
    fn test_path_routing_builder() {
        let test = PathRoutingTest::new("10.0.0.1", 80)
            .add_prefix("/api", "api-backend")
            .add_exact("/health", "health-backend");

        assert_eq!(test.paths.len(), 2);
    }

    #[test]
    fn test_header_routing_builder() {
        let test =
            HeaderRoutingTest::new("10.0.0.1", 80).add_header_rule("X-Env", "prod", "prod-backend");

        assert_eq!(test.header_rules.len(), 1);
        assert_eq!(test.header_rules[0].header_name, "X-Env");
    }
}
