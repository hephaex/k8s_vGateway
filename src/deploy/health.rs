//! Health checking for Gateway API components
//!
//! Provides readiness and health verification for gateways.

use anyhow::Result;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;
use tracing::{debug, info};

use crate::http::HttpClient;
use crate::models::GatewayImpl;

/// Health check configuration
#[derive(Clone, Debug)]
pub struct HealthCheckConfig {
    /// Timeout for individual checks
    pub check_timeout_secs: u64,

    /// Overall timeout for health check
    pub total_timeout_secs: u64,

    /// Interval between retries
    pub retry_interval_secs: u64,

    /// Number of successful checks required
    pub success_threshold: u32,

    /// HTTP health endpoint path
    pub health_path: String,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_timeout_secs: 10,
            total_timeout_secs: 120,
            retry_interval_secs: 5,
            success_threshold: 3,
            health_path: "/healthz".to_string(),
        }
    }
}

impl HealthCheckConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check_timeout(mut self, secs: u64) -> Self {
        self.check_timeout_secs = secs;
        self
    }

    pub fn total_timeout(mut self, secs: u64) -> Self {
        self.total_timeout_secs = secs;
        self
    }

    pub fn retry_interval(mut self, secs: u64) -> Self {
        self.retry_interval_secs = secs;
        self
    }
}

/// Health checker for gateway components
pub struct HealthChecker {
    config: HealthCheckConfig,
    http_client: HttpClient,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(config: HealthCheckConfig) -> Result<Self> {
        let http_client = HttpClient::with_timeout(config.check_timeout_secs)?;
        Ok(Self {
            config,
            http_client,
        })
    }

    /// Check overall gateway health
    pub async fn check_gateway(&self, gateway: GatewayImpl, ip: &str, port: u16) -> HealthStatus {
        info!(
            "Checking health of {} gateway at {}:{}",
            gateway.name(),
            ip,
            port
        );

        let mut checks = Vec::new();

        // Check GatewayClass
        checks.push(self.check_gateway_class(gateway).await);

        // Check HTTP connectivity
        checks.push(self.check_http_connectivity(ip, port).await);

        // Check pods
        checks.push(self.check_pods(gateway).await);

        // Aggregate results
        let passed = checks.iter().filter(|c| c.passed).count();
        let total = checks.len();

        HealthStatus {
            gateway,
            healthy: passed == total,
            checks,
            message: if passed == total {
                "All health checks passed".to_string()
            } else {
                format!("{passed}/{total} checks passed")
            },
        }
    }

    /// Check if GatewayClass is accepted
    async fn check_gateway_class(&self, gateway: GatewayImpl) -> HealthCheck {
        let name = "GatewayClass";
        let gateway_class = gateway.gateway_class();

        let output = Command::new("kubectl")
            .args([
                "get",
                "gatewayclass",
                gateway_class,
                "-o",
                "jsonpath={.status.conditions[?(@.type=='Accepted')].status}",
            ])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                let status = String::from_utf8_lossy(&o.stdout);
                if status.trim() == "True" {
                    HealthCheck::pass(name, "GatewayClass is accepted")
                } else {
                    HealthCheck::fail(name, format!("GatewayClass status: {}", status.trim()))
                }
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                HealthCheck::fail(name, format!("GatewayClass not found: {stderr}"))
            }
            Err(e) => HealthCheck::fail(name, format!("kubectl error: {e}")),
        }
    }

    /// Check HTTP connectivity to gateway
    async fn check_http_connectivity(&self, ip: &str, port: u16) -> HealthCheck {
        let name = "HTTP Connectivity";
        let url = format!("http://{ip}:{port}/");

        debug!("Checking HTTP connectivity to {}", url);

        match self.http_client.get(&url).await {
            Ok(response) => {
                // Any response (including 404) means gateway is reachable
                HealthCheck::pass(
                    name,
                    format!("Gateway reachable (status: {})", response.status_code),
                )
            }
            Err(e) => {
                // Connection refused or timeout
                HealthCheck::fail(name, format!("Cannot connect: {e}"))
            }
        }
    }

    /// Check if gateway pods are running
    async fn check_pods(&self, gateway: GatewayImpl) -> HealthCheck {
        let name = "Pods";
        let label_selector = gateway.pod_selector();

        let output = Command::new("kubectl")
            .args([
                "get",
                "pods",
                "-l",
                label_selector,
                "-A",
                "-o",
                "jsonpath={.items[*].status.phase}",
            ])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                let phases = String::from_utf8_lossy(&o.stdout);
                let phases: Vec<&str> = phases.split_whitespace().collect();

                if phases.is_empty() {
                    HealthCheck::fail(name, "No pods found")
                } else {
                    let running = phases.iter().filter(|p| *p == &"Running").count();
                    if running == phases.len() {
                        HealthCheck::pass(name, format!("{running} pods running"))
                    } else {
                        HealthCheck::fail(
                            name,
                            format!("{}/{} pods running", running, phases.len()),
                        )
                    }
                }
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                HealthCheck::fail(name, format!("Failed to get pods: {stderr}"))
            }
            Err(e) => HealthCheck::fail(name, format!("kubectl error: {e}")),
        }
    }

    /// Check Gateway resource status
    pub async fn check_gateway_resource(&self, name: &str, namespace: &str) -> HealthCheck {
        let check_name = "Gateway Resource";

        let output = Command::new("kubectl")
            .args([
                "get",
                "gateway",
                name,
                "-n",
                namespace,
                "-o",
                "jsonpath={.status.conditions[?(@.type=='Accepted')].status}",
            ])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                let status = String::from_utf8_lossy(&o.stdout);
                if status.trim() == "True" {
                    HealthCheck::pass(check_name, "Gateway is accepted")
                } else {
                    HealthCheck::fail(check_name, format!("Gateway status: {}", status.trim()))
                }
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                HealthCheck::fail(check_name, format!("Gateway not found: {stderr}"))
            }
            Err(e) => HealthCheck::fail(check_name, format!("kubectl error: {e}")),
        }
    }

    /// Check HTTPRoute status
    pub async fn check_httproute(&self, name: &str, namespace: &str) -> HealthCheck {
        let check_name = "HTTPRoute";

        let output = Command::new("kubectl")
            .args([
                "get",
                "httproute",
                name,
                "-n",
                namespace,
                "-o",
                "jsonpath={.status.parents[*].conditions[?(@.type=='Accepted')].status}",
            ])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                let status = String::from_utf8_lossy(&o.stdout);
                if status.contains("True") {
                    HealthCheck::pass(check_name, "HTTPRoute is accepted")
                } else {
                    HealthCheck::fail(check_name, format!("HTTPRoute status: {}", status.trim()))
                }
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                HealthCheck::fail(check_name, format!("HTTPRoute not found: {stderr}"))
            }
            Err(e) => HealthCheck::fail(check_name, format!("kubectl error: {e}")),
        }
    }

    /// Wait for gateway to become healthy
    pub async fn wait_healthy(
        &self,
        gateway: GatewayImpl,
        ip: &str,
        port: u16,
    ) -> Result<HealthStatus> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(self.config.total_timeout_secs);
        let mut success_count = 0u32;

        info!(
            "Waiting for {} to become healthy (timeout: {}s)",
            gateway.name(),
            self.config.total_timeout_secs
        );

        loop {
            if start.elapsed() > timeout {
                return Ok(HealthStatus {
                    gateway,
                    healthy: false,
                    checks: vec![HealthCheck::fail("Timeout", "Health check timed out")],
                    message: format!("Timeout after {}s", self.config.total_timeout_secs),
                });
            }

            let status = self.check_gateway(gateway, ip, port).await;

            if status.healthy {
                success_count += 1;
                if success_count >= self.config.success_threshold {
                    info!(
                        "{} is healthy after {} checks",
                        gateway.name(),
                        success_count
                    );
                    return Ok(status);
                }
                debug!(
                    "Health check passed ({}/{})",
                    success_count, self.config.success_threshold
                );
            } else {
                success_count = 0;
                debug!("Health check failed, retrying...");
            }

            sleep(Duration::from_secs(self.config.retry_interval_secs)).await;
        }
    }

    /// Quick connectivity check
    pub async fn ping(&self, ip: &str, port: u16) -> bool {
        let url = format!("http://{ip}:{port}/");
        self.http_client.get(&url).await.is_ok()
    }

    /// Check TLS connectivity
    pub async fn check_tls(&self, ip: &str, port: u16, hostname: &str) -> HealthCheck {
        let name = "TLS Connectivity";
        let _url = format!("https://{ip}:{port}/");

        // Use curl for TLS check with SNI
        let output = Command::new("curl")
            .args([
                "-s",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                "--resolve",
                &format!("{hostname}:{port}:{ip}"),
                "-k", // Allow self-signed certs
                &format!("https://{hostname}:{port}/"),
            ])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                let code = String::from_utf8_lossy(&o.stdout);
                HealthCheck::pass(name, format!("TLS connection OK (status: {})", code.trim()))
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                HealthCheck::fail(name, format!("TLS connection failed: {stderr}"))
            }
            Err(e) => HealthCheck::fail(name, format!("curl error: {e}")),
        }
    }
}

/// Health status of a gateway
#[derive(Clone, Debug)]
pub struct HealthStatus {
    /// Gateway implementation
    pub gateway: GatewayImpl,

    /// Overall health status
    pub healthy: bool,

    /// Individual health checks
    pub checks: Vec<HealthCheck>,

    /// Status message
    pub message: String,
}

impl HealthStatus {
    /// Format as table
    pub fn format_table(&self) -> String {
        let mut output = String::new();

        output.push_str("\n┌─────────────────────────────────────────────────────────────┐\n");
        output.push_str(&format!(
            "│ {} Health Status: {}                            │\n",
            self.gateway.name(),
            if self.healthy {
                "✓ Healthy"
            } else {
                "✗ Unhealthy"
            }
        ));
        output.push_str("├─────────────────────────────────────────────────────────────┤\n");

        for check in &self.checks {
            let status = if check.passed { "✓" } else { "✗" };
            output.push_str(&format!(
                "│ {} {:20} {:35} │\n",
                status,
                check.name,
                truncate(&check.message, 35)
            ));
        }

        output.push_str("└─────────────────────────────────────────────────────────────┘\n");

        output
    }
}

/// Individual health check result
#[derive(Clone, Debug)]
pub struct HealthCheck {
    /// Check name
    pub name: String,

    /// Whether check passed
    pub passed: bool,

    /// Result message
    pub message: String,
}

impl HealthCheck {
    pub fn pass(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            message: message.into(),
        }
    }

    pub fn fail(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            message: message.into(),
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Pre-flight checks before running tests
pub struct PreFlightChecker {
    health_checker: HealthChecker,
}

impl PreFlightChecker {
    pub fn new(config: HealthCheckConfig) -> Result<Self> {
        Ok(Self {
            health_checker: HealthChecker::new(config)?,
        })
    }

    /// Run all pre-flight checks
    pub async fn run(&self, gateway: GatewayImpl, ip: &str, port: u16) -> PreFlightResult {
        info!("Running pre-flight checks for {}", gateway.name());

        let mut checks = Vec::new();

        // Check kubectl availability
        checks.push(self.check_kubectl().await);

        // Check cluster connectivity
        checks.push(self.check_cluster().await);

        // Check Gateway API CRDs
        checks.push(self.check_gateway_api_crds().await);

        // Check gateway health
        let health = self.health_checker.check_gateway(gateway, ip, port).await;
        checks.extend(health.checks);

        let passed = checks.iter().filter(|c| c.passed).count();
        let total = checks.len();

        PreFlightResult {
            passed: passed == total,
            checks,
            message: if passed == total {
                "All pre-flight checks passed. Ready to run tests.".to_string()
            } else {
                format!("{passed}/{total} checks passed. Some issues found.")
            },
        }
    }

    async fn check_kubectl(&self) -> HealthCheck {
        let output = Command::new("kubectl").arg("version").output().await;

        match output {
            Ok(o) if o.status.success() => HealthCheck::pass("kubectl", "kubectl is available"),
            _ => HealthCheck::fail("kubectl", "kubectl not found or not working"),
        }
    }

    async fn check_cluster(&self) -> HealthCheck {
        let output = Command::new("kubectl")
            .args(["cluster-info"])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => HealthCheck::pass("Cluster", "Cluster is reachable"),
            _ => HealthCheck::fail("Cluster", "Cannot connect to cluster"),
        }
    }

    async fn check_gateway_api_crds(&self) -> HealthCheck {
        let output = Command::new("kubectl")
            .args(["get", "crd", "gateways.gateway.networking.k8s.io"])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                HealthCheck::pass("Gateway API", "Gateway API CRDs installed")
            }
            _ => HealthCheck::fail("Gateway API", "Gateway API CRDs not found"),
        }
    }
}

/// Pre-flight check result
#[derive(Clone, Debug)]
pub struct PreFlightResult {
    /// Whether all checks passed
    pub passed: bool,

    /// Individual checks
    pub checks: Vec<HealthCheck>,

    /// Result message
    pub message: String,
}

impl PreFlightResult {
    /// Format as table
    pub fn format_table(&self) -> String {
        let mut output = String::new();

        output.push_str("\n┌─────────────────────────────────────────────────────────────┐\n");
        output.push_str("│ Pre-Flight Checks                                           │\n");
        output.push_str("├─────────────────────────────────────────────────────────────┤\n");

        for check in &self.checks {
            let status = if check.passed { "✓" } else { "✗" };
            output.push_str(&format!(
                "│ {} {:20} {:35} │\n",
                status,
                check.name,
                truncate(&check.message, 35)
            ));
        }

        output.push_str("├─────────────────────────────────────────────────────────────┤\n");
        output.push_str(&format!(
            "│ Result: {}                                              │\n",
            if self.passed { "READY" } else { "BLOCKED" }
        ));
        output.push_str("└─────────────────────────────────────────────────────────────┘\n");

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check_config() {
        let config = HealthCheckConfig::new()
            .check_timeout(15)
            .total_timeout(180)
            .retry_interval(10);

        assert_eq!(config.check_timeout_secs, 15);
        assert_eq!(config.total_timeout_secs, 180);
        assert_eq!(config.retry_interval_secs, 10);
    }

    #[test]
    fn test_health_check() {
        let pass = HealthCheck::pass("test", "passed");
        assert!(pass.passed);

        let fail = HealthCheck::fail("test", "failed");
        assert!(!fail.passed);
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is long", 10), "this is...");
    }
}
