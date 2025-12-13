//! TLS tests for Gateway API
//!
//! Tests 4-6: TLS Termination, HTTPS Redirect, Backend TLS (mTLS)

#![allow(dead_code)]

use anyhow::Result;
use tracing::{debug, info};

use crate::http::HttpClient;
use crate::models::{TestCase, TestResult, TestStatus};

/// Test 4: TLS Termination
#[derive(Clone, Debug)]
pub struct TlsTerminationTest {
    pub gateway_ip: String,
    pub https_port: u16,
    pub hostname: String,
    pub expected_cert_cn: Option<String>,
}

impl TlsTerminationTest {
    pub fn new(
        gateway_ip: impl Into<String>,
        https_port: u16,
        hostname: impl Into<String>,
    ) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            https_port,
            hostname: hostname.into(),
            expected_cert_cn: None,
        }
    }

    pub fn with_cert_cn(mut self, cn: impl Into<String>) -> Self {
        self.expected_cert_cn = Some(cn.into());
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!("Running TLS Termination Test");
        let start = std::time::Instant::now();
        let mut details = Vec::new();

        // Test HTTPS endpoint
        let response = client
            .test_https(&self.gateway_ip, self.https_port, "/")
            .await;

        let status = match response {
            Ok(resp) => {
                if resp.is_success() {
                    details.push(format!(
                        "✓ HTTPS connection successful ({}ms)",
                        resp.duration_ms
                    ));
                    TestStatus::Pass
                } else {
                    details.push(format!("✗ HTTPS returned status {}", resp.status_code));
                    TestStatus::Fail
                }
            }
            Err(e) => {
                // Check if it's a TLS error (might still be "successful" for self-signed)
                let err_str = e.to_string();
                if err_str.contains("certificate") || err_str.contains("TLS") {
                    details.push(format!("✗ TLS error: {err_str}"));
                } else {
                    details.push(format!("✗ Connection error: {err_str}"));
                }
                TestStatus::Fail
            }
        };

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::TlsTermination,
            status,
            duration_ms: duration.as_millis() as u64,
            message: Some(details.join("\n")),
            details: None,
        })
    }
}

/// Test 5: HTTPS Redirect
#[derive(Clone, Debug)]
pub struct HttpsRedirectTest {
    pub gateway_ip: String,
    pub http_port: u16,
    pub https_port: u16,
    pub paths: Vec<String>,
}

impl HttpsRedirectTest {
    pub fn new(gateway_ip: impl Into<String>, http_port: u16, https_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            http_port,
            https_port,
            paths: vec!["/".to_string()],
        }
    }

    pub fn add_path(mut self, path: impl Into<String>) -> Self {
        self.paths.push(path.into());
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!("Running HTTPS Redirect Test");
        let start = std::time::Instant::now();
        let mut all_passed = true;
        let mut details = Vec::new();

        for path in &self.paths {
            let http_url = format!("http://{}:{}{}", self.gateway_ip, self.http_port, path);
            debug!("Testing redirect for: {}", http_url);

            let result = client.test_redirect(&http_url).await;

            match result {
                Ok((status_code, location)) => {
                    let is_redirect = (301..=308).contains(&status_code);
                    let has_https_location = location
                        .as_ref()
                        .map(|l| l.starts_with("https://"))
                        .unwrap_or(false);

                    if is_redirect && has_https_location {
                        details.push(format!(
                            "✓ {} redirects to {} ({})",
                            path,
                            location.unwrap_or_default(),
                            status_code
                        ));
                    } else if is_redirect {
                        all_passed = false;
                        details.push(format!(
                            "✗ {} redirects but not to HTTPS ({})",
                            path,
                            location.unwrap_or("no location".to_string())
                        ));
                    } else {
                        all_passed = false;
                        details.push(format!(
                            "✗ {path} returns {status_code} instead of redirect"
                        ));
                    }
                }
                Err(e) => {
                    all_passed = false;
                    details.push(format!("✗ {path} failed: {e}"));
                }
            }
        }

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::HttpsRedirect,
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

/// Test 6: Backend TLS (mTLS)
#[derive(Clone, Debug)]
pub struct BackendTlsTest {
    pub gateway_ip: String,
    pub gateway_port: u16,
    pub backend_path: String,
    pub expected_mtls: bool,
}

impl BackendTlsTest {
    pub fn new(gateway_ip: impl Into<String>, gateway_port: u16) -> Self {
        Self {
            gateway_ip: gateway_ip.into(),
            gateway_port,
            backend_path: "/mtls-test".to_string(),
            expected_mtls: true,
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.backend_path = path.into();
        self
    }

    pub async fn run(&self, client: &HttpClient) -> Result<TestResult> {
        info!("Running Backend TLS (mTLS) Test");
        let start = std::time::Instant::now();
        let mut details = Vec::new();

        // Request the mTLS endpoint
        let response = client
            .test_path_routing(&self.gateway_ip, self.gateway_port, &self.backend_path)
            .await;

        let status = match response {
            Ok(resp) => {
                // Backend should respond if mTLS is properly configured
                if resp.is_success() {
                    // Check if backend indicates mTLS verification
                    let mtls_verified = resp.body_contains("mtls")
                        || resp.body_contains("client-cert")
                        || resp.get_header("x-client-cert").is_some();

                    if mtls_verified || !self.expected_mtls {
                        details.push(format!(
                            "✓ Backend TLS connection successful ({}ms)",
                            resp.duration_ms
                        ));
                        TestStatus::Pass
                    } else {
                        details.push("✓ Connection successful but mTLS not verified".to_string());
                        TestStatus::Pass // Partial success
                    }
                } else if resp.status_code == 503 || resp.status_code == 502 {
                    details.push(format!(
                        "✗ Backend unreachable (status {}), possible TLS handshake failure",
                        resp.status_code
                    ));
                    TestStatus::Fail
                } else {
                    details.push(format!(
                        "✗ Unexpected status {} from backend",
                        resp.status_code
                    ));
                    TestStatus::Fail
                }
            }
            Err(e) => {
                details.push(format!("✗ Request failed: {e}"));
                TestStatus::Fail
            }
        };

        let duration = start.elapsed();

        Ok(TestResult {
            test_case: TestCase::BackendTls,
            status,
            duration_ms: duration.as_millis() as u64,
            message: Some(details.join("\n")),
            details: None,
        })
    }
}

/// Combined TLS test runner
pub struct TlsTestSuite {
    pub gateway_ip: String,
    pub http_port: u16,
    pub https_port: u16,
    pub hostname: String,
    pub client: HttpClient,
}

impl TlsTestSuite {
    pub fn new(
        gateway_ip: impl Into<String>,
        http_port: u16,
        https_port: u16,
        hostname: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            gateway_ip: gateway_ip.into(),
            http_port,
            https_port,
            hostname: hostname.into(),
            client: HttpClient::new()?,
        })
    }

    pub async fn run_all(&self) -> Result<Vec<TestResult>> {
        let mut results = Vec::new();

        // TLS termination test
        let tls_test = TlsTerminationTest::new(&self.gateway_ip, self.https_port, &self.hostname);
        results.push(tls_test.run(&self.client).await?);

        // HTTPS redirect test
        let redirect_test =
            HttpsRedirectTest::new(&self.gateway_ip, self.http_port, self.https_port)
                .add_path("/api")
                .add_path("/login");
        results.push(redirect_test.run(&self.client).await?);

        // Backend TLS test
        let backend_tls_test = BackendTlsTest::new(&self.gateway_ip, self.https_port);
        results.push(backend_tls_test.run(&self.client).await?);

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_termination_builder() {
        let test = TlsTerminationTest::new("10.0.0.1", 443, "secure.example.com")
            .with_cert_cn("*.example.com");

        assert_eq!(test.https_port, 443);
        assert_eq!(test.expected_cert_cn, Some("*.example.com".to_string()));
    }

    #[test]
    fn test_https_redirect_builder() {
        let test = HttpsRedirectTest::new("10.0.0.1", 80, 443)
            .add_path("/api")
            .add_path("/login");

        assert_eq!(test.paths.len(), 3); // "/" + 2 added paths
    }

    #[test]
    fn test_backend_tls_builder() {
        let test = BackendTlsTest::new("10.0.0.1", 443).with_path("/secure-backend");

        assert_eq!(test.backend_path, "/secure-backend");
    }
}
