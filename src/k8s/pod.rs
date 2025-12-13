//! Pod management for test execution
//!
//! Provides pod creation and curl execution capabilities.

#![allow(dead_code)]

use anyhow::{Context, Result};
use k8s_openapi::api::core::v1::{Container, Pod, PodSpec};
use kube::api::{Api, DeleteParams, ListParams, PostParams};
use kube::runtime::wait::{await_condition, conditions::is_pod_running};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use tracing::debug;

use super::K8sClient;

/// Pod manager for test operations
pub struct PodManager {
    client: K8sClient,
}

impl PodManager {
    pub fn new(client: K8sClient) -> Self {
        Self { client }
    }

    fn api(&self, namespace: &str) -> Api<Pod> {
        Api::namespaced(self.client.client().clone(), namespace)
    }

    /// Create a test pod
    pub async fn create_test_pod(&self, config: &TestPodConfig) -> Result<Pod> {
        let pod = Pod {
            metadata: kube::core::ObjectMeta {
                name: Some(config.name.clone()),
                namespace: Some(config.namespace.clone()),
                labels: Some(config.labels.clone()),
                ..Default::default()
            },
            spec: Some(PodSpec {
                containers: vec![Container {
                    name: "test".to_string(),
                    image: Some(config.image.clone()),
                    command: Some(vec!["sleep".to_string(), "infinity".to_string()]),
                    ..Default::default()
                }],
                restart_policy: Some("Never".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let api = self.api(&config.namespace);
        api.create(&PostParams::default(), &pod)
            .await
            .context("Failed to create test pod")
    }

    /// Wait for pod to be running
    pub async fn wait_running(&self, name: &str, namespace: &str, timeout_secs: u64) -> Result<()> {
        let api = self.api(namespace);

        let cond = await_condition(api, name, is_pod_running());
        tokio::time::timeout(Duration::from_secs(timeout_secs), cond)
            .await
            .context("Timeout waiting for pod")?
            .context("Error waiting for pod")?;

        Ok(())
    }

    /// Execute command in pod using kubectl
    pub async fn exec_in_pod(
        &self,
        name: &str,
        namespace: &str,
        command: Vec<String>,
    ) -> Result<String> {
        // Use kubectl exec as a fallback since kube-rs exec requires ws feature
        let mut kubectl_args = vec![
            "exec".to_string(),
            "-n".to_string(),
            namespace.to_string(),
            name.to_string(),
            "--".to_string(),
        ];
        kubectl_args.extend(command);

        let output = tokio::process::Command::new("kubectl")
            .args(&kubectl_args)
            .output()
            .await
            .context("Failed to execute kubectl")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("kubectl exec failed: {stderr}")
        }
    }

    /// Execute curl command in pod
    pub async fn curl(
        &self,
        pod_name: &str,
        namespace: &str,
        url: &str,
        options: &CurlOptions,
    ) -> Result<CurlResponse> {
        let mut command = vec!["curl".to_string()];

        // Add options
        if options.insecure {
            command.push("-k".to_string());
        }
        if options.follow_redirects {
            command.push("-L".to_string());
        }
        if let Some(timeout) = options.timeout_secs {
            command.push("-m".to_string());
            command.push(timeout.to_string());
        }

        // Add headers
        for (key, value) in &options.headers {
            command.push("-H".to_string());
            command.push(format!("{key}: {value}"));
        }

        // Add method if not GET
        if options.method != "GET" {
            command.push("-X".to_string());
            command.push(options.method.clone());
        }

        // Add body
        if let Some(body) = &options.body {
            command.push("-d".to_string());
            command.push(body.clone());
        }

        // Include response headers
        command.push("-i".to_string());

        // Add URL
        command.push(url.to_string());

        debug!("Executing curl: {:?}", command);

        let output = self.exec_in_pod(pod_name, namespace, command).await?;

        // Parse response
        let (status_code, headers, body) = parse_curl_response(&output)?;

        Ok(CurlResponse {
            status_code,
            headers,
            body,
        })
    }

    /// Delete pod
    pub async fn delete_pod(&self, name: &str, namespace: &str) -> Result<()> {
        let api = self.api(namespace);
        api.delete(name, &DeleteParams::default())
            .await
            .context("Failed to delete pod")?;
        Ok(())
    }

    /// List pods with label selector
    pub async fn list_pods(
        &self,
        namespace: &str,
        label_selector: Option<&str>,
    ) -> Result<Vec<Pod>> {
        let api = self.api(namespace);
        let params = match label_selector {
            Some(selector) => ListParams::default().labels(selector),
            None => ListParams::default(),
        };
        let list = api.list(&params).await.context("Failed to list pods")?;
        Ok(list.items)
    }
}

/// Test pod configuration
#[derive(Clone, Debug)]
pub struct TestPodConfig {
    pub name: String,
    pub namespace: String,
    pub image: String,
    pub labels: BTreeMap<String, String>,
}

impl TestPodConfig {
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), "gateway-test".to_string());

        Self {
            name: name.into(),
            namespace: namespace.into(),
            image: "curlimages/curl:latest".to_string(),
            labels,
        }
    }

    pub fn with_image(mut self, image: impl Into<String>) -> Self {
        self.image = image.into();
        self
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }
}

/// Curl command options
#[derive(Clone, Debug, Default)]
pub struct CurlOptions {
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub timeout_secs: Option<u64>,
    pub insecure: bool,
    pub follow_redirects: bool,
}

impl CurlOptions {
    pub fn new() -> Self {
        Self {
            method: "GET".to_string(),
            ..Default::default()
        }
    }

    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = method.into();
        self
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    pub fn insecure(mut self) -> Self {
        self.insecure = true;
        self
    }

    pub fn follow_redirects(mut self) -> Self {
        self.follow_redirects = true;
        self
    }
}

/// Curl response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurlResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl CurlResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    pub fn get_header(&self, name: &str) -> Option<&String> {
        self.headers.get(&name.to_lowercase())
    }
}

/// Parse curl -i output into status, headers, and body
fn parse_curl_response(output: &str) -> Result<(u16, HashMap<String, String>, String)> {
    let mut lines = output.lines();

    // Parse status line
    let status_line = lines.next().unwrap_or("");
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Parse headers
    let mut headers = HashMap::new();
    let mut body_start = false;
    let mut body_lines = Vec::new();

    for line in lines {
        if body_start {
            body_lines.push(line);
        } else if line.is_empty() || line == "\r" {
            body_start = true;
        } else if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }

    let body = body_lines.join("\n");

    Ok((status_code, headers, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curl_options_builder() {
        let options = CurlOptions::new()
            .method("POST")
            .header("Content-Type", "application/json")
            .body(r#"{"key": "value"}"#)
            .timeout(30)
            .insecure();

        assert_eq!(options.method, "POST");
        assert!(options.insecure);
        assert_eq!(options.timeout_secs, Some(30));
    }

    #[test]
    fn test_parse_curl_response() {
        let output = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello World";
        let (status, headers, body) = parse_curl_response(output).unwrap();

        assert_eq!(status, 200);
        assert_eq!(headers.get("content-type"), Some(&"text/plain".to_string()));
        assert_eq!(body, "Hello World");
    }

    #[test]
    fn test_test_pod_config() {
        let config = TestPodConfig::new("test-pod", "default")
            .with_image("alpine:latest")
            .with_label("test", "true");

        assert_eq!(config.name, "test-pod");
        assert_eq!(config.image, "alpine:latest");
        assert_eq!(config.labels.get("test").map(|s| s.as_str()), Some("true"));
    }
}
