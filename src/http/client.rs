//! HTTP client for Gateway API testing
//!
//! Provides a high-level HTTP client for testing Gateway implementations.

#![allow(dead_code)]

use anyhow::{Context, Result};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use tracing::debug;

/// HTTP client errors
#[derive(Error, Debug)]
pub enum HttpError {
    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    #[error("Connection refused to {0}")]
    ConnectionRefused(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("TLS error: {0}")]
    TlsError(String),
}

/// HTTP client for testing
#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    base_url: Option<String>,
    default_headers: HeaderMap,
    timeout_secs: u64,
}

impl HttpClient {
    /// Create a new HTTP client
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .danger_accept_invalid_certs(true)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url: None,
            default_headers: HeaderMap::new(),
            timeout_secs: 30,
        })
    }

    /// Create client with custom timeout
    pub fn with_timeout(timeout_secs: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .danger_accept_invalid_certs(true)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url: None,
            default_headers: HeaderMap::new(),
            timeout_secs,
        })
    }

    /// Set base URL for requests
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Add default header
    pub fn default_header(mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> Result<Self> {
        let header_name =
            HeaderName::from_bytes(key.as_ref().as_bytes()).context("Invalid header name")?;
        let header_value = HeaderValue::from_str(value.as_ref()).context("Invalid header value")?;
        self.default_headers.insert(header_name, header_value);
        Ok(self)
    }

    /// Build full URL
    fn build_url(&self, path: &str) -> String {
        match &self.base_url {
            Some(base) => {
                if path.starts_with("http://") || path.starts_with("https://") {
                    path.to_string()
                } else {
                    format!("{}{}", base.trim_end_matches('/'), path)
                }
            }
            None => path.to_string(),
        }
    }

    /// Send HTTP request
    pub async fn send(&self, request: HttpRequest) -> Result<HttpResponse> {
        let url = self.build_url(&request.url);
        debug!("Sending {} request to {}", request.method, url);

        let method =
            Method::from_bytes(request.method.as_bytes()).context("Invalid HTTP method")?;

        let mut req_builder = self.client.request(method, &url);

        // Add default headers
        for (key, value) in &self.default_headers {
            req_builder = req_builder.header(key, value);
        }

        // Add request headers
        for (key, value) in &request.headers {
            req_builder = req_builder.header(key.as_str(), value.as_str());
        }

        // Add body if present
        if let Some(body) = &request.body {
            req_builder = req_builder.body(body.clone());
        }

        let start = std::time::Instant::now();

        let response = req_builder.send().await.map_err(|e| {
            if e.is_timeout() {
                anyhow::anyhow!(HttpError::Timeout(self.timeout_secs))
            } else if e.is_connect() {
                anyhow::anyhow!(HttpError::ConnectionRefused(url.clone()))
            } else {
                anyhow::anyhow!(HttpError::RequestFailed(e.to_string()))
            }
        })?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let status = response.status();
        let headers = response.headers().clone();

        // Extract response headers
        let mut response_headers = HashMap::new();
        for (key, value) in headers.iter() {
            if let Ok(v) = value.to_str() {
                response_headers.insert(key.to_string(), v.to_string());
            }
        }

        let body = response
            .text()
            .await
            .context("Failed to read response body")?;

        debug!(
            "Response: {} {} in {}ms",
            status.as_u16(),
            status.canonical_reason().unwrap_or(""),
            duration_ms
        );

        Ok(HttpResponse {
            status_code: status.as_u16(),
            headers: response_headers,
            body,
            duration_ms,
        })
    }

    /// Convenience method for GET request
    pub async fn get(&self, url: &str) -> Result<HttpResponse> {
        self.send(HttpRequest::get(url)).await
    }

    /// GET with custom headers
    pub async fn get_with_headers(
        &self,
        url: &str,
        headers: HashMap<String, String>,
    ) -> Result<HttpResponse> {
        self.send(HttpRequest::get(url).headers(headers)).await
    }

    /// GET with Host header
    pub async fn get_with_host(&self, url: &str, hostname: &str) -> Result<HttpResponse> {
        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), hostname.to_string());
        self.get_with_headers(url, headers).await
    }

    /// Convenience method for POST request
    pub async fn post(&self, url: &str, body: impl Into<String>) -> Result<HttpResponse> {
        self.send(HttpRequest::post(url).body(body)).await
    }

    /// Test host routing
    pub async fn test_host_routing(
        &self,
        ip: &str,
        port: u16,
        hostname: &str,
    ) -> Result<HttpResponse> {
        let url = format!("http://{ip}:{port}/");
        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), hostname.to_string());
        self.get_with_headers(&url, headers).await
    }

    /// Test path routing
    pub async fn test_path_routing(&self, ip: &str, port: u16, path: &str) -> Result<HttpResponse> {
        let url = format!("http://{ip}:{port}{path}");
        self.get(&url).await
    }

    /// Test header routing
    pub async fn test_header_routing(
        &self,
        ip: &str,
        port: u16,
        header_name: &str,
        header_value: &str,
    ) -> Result<HttpResponse> {
        let url = format!("http://{ip}:{port}/");
        let mut headers = HashMap::new();
        headers.insert(header_name.to_string(), header_value.to_string());
        self.get_with_headers(&url, headers).await
    }

    /// Test HTTPS endpoint
    pub async fn test_https(&self, ip: &str, port: u16, path: &str) -> Result<HttpResponse> {
        let url = format!("https://{ip}:{port}{path}");
        self.get(&url).await
    }

    /// Test redirect
    pub async fn test_redirect(&self, url: &str) -> Result<(u16, Option<String>)> {
        // Don't follow redirects for this test
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .build()
            .context("Failed to create client")?;

        let response = client.get(url).send().await?;
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Ok((status, location))
    }

    /// Run load test with concurrent requests
    pub async fn load_test(
        &self,
        url: &str,
        concurrent: usize,
        total: usize,
    ) -> Result<LoadTestResult> {
        use futures::future::join_all;

        let mut handles = Vec::new();
        let requests_per_task = total / concurrent;

        for _ in 0..concurrent {
            let client = self.clone();
            let url = url.to_string();

            let handle = tokio::spawn(async move {
                let mut successes = 0;
                let mut failures = 0;
                let mut total_duration = 0u64;

                for _ in 0..requests_per_task {
                    match client.get(&url).await {
                        Ok(resp) if resp.is_success() => {
                            successes += 1;
                            total_duration += resp.duration_ms;
                        }
                        _ => {
                            failures += 1;
                        }
                    }
                }

                (successes, failures, total_duration)
            });

            handles.push(handle);
        }

        let results = join_all(handles).await;

        let mut total_successes = 0;
        let mut total_failures = 0;
        let mut total_duration = 0u64;

        for (s, f, d) in results.into_iter().flatten() {
            total_successes += s;
            total_failures += f;
            total_duration += d;
        }

        let avg_duration = if total_successes > 0 {
            total_duration / total_successes as u64
        } else {
            0
        };

        Ok(LoadTestResult {
            total_requests: total,
            successes: total_successes,
            failures: total_failures,
            avg_duration_ms: avg_duration,
        })
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default HTTP client")
    }
}

/// HTTP request builder
#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl HttpRequest {
    pub fn new(method: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            url: url.into(),
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn get(url: impl Into<String>) -> Self {
        Self::new("GET", url)
    }

    pub fn post(url: impl Into<String>) -> Self {
        Self::new("POST", url)
    }

    pub fn put(url: impl Into<String>) -> Self {
        Self::new("PUT", url)
    }

    pub fn delete(url: impl Into<String>) -> Self {
        Self::new("DELETE", url)
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers.extend(headers);
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }
}

/// HTTP response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub duration_ms: u64,
}

impl HttpResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    pub fn is_redirect(&self) -> bool {
        (300..400).contains(&self.status_code)
    }

    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status_code)
    }

    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status_code)
    }

    pub fn get_header(&self, name: &str) -> Option<&String> {
        self.headers.get(&name.to_lowercase())
    }

    pub fn body_contains(&self, text: &str) -> bool {
        self.body.contains(text)
    }
}

/// Load test result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadTestResult {
    pub total_requests: usize,
    pub successes: usize,
    pub failures: usize,
    pub avg_duration_ms: u64,
}

impl LoadTestResult {
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successes as f64 / self.total_requests as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_request_builder() {
        let req = HttpRequest::get("http://example.com")
            .header("Host", "example.com")
            .header("X-Custom", "value");

        assert_eq!(req.method, "GET");
        assert_eq!(req.headers.len(), 2);
    }

    #[test]
    fn test_http_response() {
        let resp = HttpResponse {
            status_code: 200,
            headers: HashMap::new(),
            body: "Hello World".to_string(),
            duration_ms: 100,
        };

        assert!(resp.is_success());
        assert!(!resp.is_redirect());
        assert!(resp.body_contains("Hello"));
    }

    #[test]
    fn test_load_test_result() {
        let result = LoadTestResult {
            total_requests: 100,
            successes: 90,
            failures: 10,
            avg_duration_ms: 50,
        };

        assert_eq!(result.success_rate(), 90.0);
    }
}
