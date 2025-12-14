//! Gateway and Test profiles
//!
//! Provides predefined configurations for gateways and test suites.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::GatewayImpl;

/// Gateway profile with predefined settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GatewayProfile {
    /// Profile name
    pub name: String,
    /// Gateway implementation
    pub gateway: GatewayImpl,
    /// Default namespace
    pub namespace: String,
    /// Default HTTP port
    pub http_port: u16,
    /// Default HTTPS port
    pub https_port: u16,
    /// Default gRPC port
    pub grpc_port: Option<u16>,
    /// Default hostname
    pub hostname: String,
    /// Installation method
    pub install_method: InstallMethod,
    /// Helm chart settings
    pub helm: Option<HelmSettings>,
    /// Custom labels
    pub labels: HashMap<String, String>,
    /// Custom annotations
    pub annotations: HashMap<String, String>,
}

/// Installation method for gateway
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InstallMethod {
    Helm,
    Manifest,
    Operator,
    Custom,
}

/// Helm chart settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HelmSettings {
    /// Chart repository
    pub repo: String,
    /// Chart name
    pub chart: String,
    /// Chart version
    pub version: Option<String>,
    /// Custom values
    pub values: HashMap<String, serde_yaml::Value>,
}

impl GatewayProfile {
    /// Create a new gateway profile
    pub fn new(name: impl Into<String>, gateway: GatewayImpl) -> Self {
        Self {
            name: name.into(),
            gateway,
            namespace: "gateway-system".to_string(),
            http_port: 80,
            https_port: 443,
            grpc_port: Some(9090),
            hostname: "example.com".to_string(),
            install_method: InstallMethod::Helm,
            helm: None,
            labels: HashMap::new(),
            annotations: HashMap::new(),
        }
    }

    /// Get default profile for a gateway
    pub fn default_for(gateway: GatewayImpl) -> Self {
        match gateway {
            GatewayImpl::Nginx => Self::nginx_default(),
            GatewayImpl::Envoy => Self::envoy_default(),
            GatewayImpl::Istio => Self::istio_default(),
            GatewayImpl::Cilium => Self::cilium_default(),
            GatewayImpl::Kong => Self::kong_default(),
            GatewayImpl::Traefik => Self::traefik_default(),
            GatewayImpl::Kgateway => Self::kgateway_default(),
        }
    }

    /// NGINX Gateway Fabric default profile
    pub fn nginx_default() -> Self {
        let mut profile = Self::new("nginx-default", GatewayImpl::Nginx);
        profile.namespace = "nginx-gateway".to_string();
        profile.helm = Some(HelmSettings {
            repo: "oci://ghcr.io/nginxinc/charts".to_string(),
            chart: "nginx-gateway-fabric".to_string(),
            version: Some("1.4.0".to_string()),
            values: HashMap::new(),
        });
        profile
    }

    /// Envoy Gateway default profile
    pub fn envoy_default() -> Self {
        let mut profile = Self::new("envoy-default", GatewayImpl::Envoy);
        profile.namespace = "envoy-gateway-system".to_string();
        profile.helm = Some(HelmSettings {
            repo: "oci://docker.io/envoyproxy".to_string(),
            chart: "gateway-helm".to_string(),
            version: Some("v1.1.0".to_string()),
            values: HashMap::new(),
        });
        profile
    }

    /// Istio Gateway default profile
    pub fn istio_default() -> Self {
        let mut profile = Self::new("istio-default", GatewayImpl::Istio);
        profile.namespace = "istio-system".to_string();
        profile.install_method = InstallMethod::Operator;
        profile
    }

    /// Cilium Gateway default profile
    pub fn cilium_default() -> Self {
        let mut profile = Self::new("cilium-default", GatewayImpl::Cilium);
        profile.namespace = "kube-system".to_string();
        profile.helm = Some(HelmSettings {
            repo: "https://helm.cilium.io/".to_string(),
            chart: "cilium".to_string(),
            version: Some("1.16.0".to_string()),
            values: {
                let mut values = HashMap::new();
                values.insert(
                    "gatewayAPI.enabled".to_string(),
                    serde_yaml::Value::Bool(true),
                );
                values
            },
        });
        profile
    }

    /// Kong Gateway default profile
    pub fn kong_default() -> Self {
        let mut profile = Self::new("kong-default", GatewayImpl::Kong);
        profile.namespace = "kong".to_string();
        profile.helm = Some(HelmSettings {
            repo: "https://charts.konghq.com".to_string(),
            chart: "kong".to_string(),
            version: Some("2.41.0".to_string()),
            values: HashMap::new(),
        });
        profile
    }

    /// Traefik Gateway default profile
    pub fn traefik_default() -> Self {
        let mut profile = Self::new("traefik-default", GatewayImpl::Traefik);
        profile.namespace = "traefik".to_string();
        profile.helm = Some(HelmSettings {
            repo: "https://traefik.github.io/charts".to_string(),
            chart: "traefik".to_string(),
            version: Some("30.0.0".to_string()),
            values: {
                let mut values = HashMap::new();
                values.insert(
                    "providers.kubernetesGateway.enabled".to_string(),
                    serde_yaml::Value::Bool(true),
                );
                values
            },
        });
        profile
    }

    /// kgateway default profile
    pub fn kgateway_default() -> Self {
        let mut profile = Self::new("kgateway-default", GatewayImpl::Kgateway);
        profile.namespace = "gloo-system".to_string();
        profile.helm = Some(HelmSettings {
            repo: "https://storage.googleapis.com/solo-public-helm".to_string(),
            chart: "gloo".to_string(),
            version: Some("1.17.0".to_string()),
            values: HashMap::new(),
        });
        profile
    }

    /// Set namespace
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Set hostname
    pub fn with_hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = hostname.into();
        self
    }

    /// Set ports
    pub fn with_ports(mut self, http: u16, https: u16, grpc: Option<u16>) -> Self {
        self.http_port = http;
        self.https_port = https;
        self.grpc_port = grpc;
        self
    }

    /// Add label
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }
}

/// Test profile - collection of tests to run
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestProfile {
    /// Profile name
    pub name: String,
    /// Description
    pub description: String,
    /// Test numbers to include
    pub tests: Vec<u8>,
    /// Number of rounds
    pub rounds: u32,
    /// Run in parallel
    pub parallel: bool,
    /// Timeout per test in seconds
    pub timeout_secs: u64,
    /// Tags for filtering
    pub tags: Vec<String>,
}

impl TestProfile {
    /// Create a new test profile
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            tests: Vec::new(),
            rounds: 1,
            parallel: false,
            timeout_secs: 30,
            tags: Vec::new(),
        }
    }

    /// All tests profile (1-17)
    pub fn all() -> Self {
        Self {
            name: "all".to_string(),
            description: "Run all 17 test cases".to_string(),
            tests: (1..=17).collect(),
            rounds: 1,
            parallel: true,
            timeout_secs: 30,
            tags: vec!["comprehensive".to_string()],
        }
    }

    /// Quick smoke test profile
    pub fn smoke() -> Self {
        Self {
            name: "smoke".to_string(),
            description: "Quick smoke tests for basic functionality".to_string(),
            tests: vec![1, 2, 3, 4], // Basic routing tests
            rounds: 1,
            parallel: false,
            timeout_secs: 30,
            tags: vec!["quick".to_string(), "smoke".to_string()],
        }
    }

    /// Routing tests profile
    pub fn routing() -> Self {
        Self {
            name: "routing".to_string(),
            description: "HTTP routing test cases".to_string(),
            tests: vec![1, 2, 3, 4, 5], // Host, path, header, method, query routing
            rounds: 1,
            parallel: true,
            timeout_secs: 30,
            tags: vec!["routing".to_string()],
        }
    }

    /// TLS tests profile
    pub fn tls() -> Self {
        Self {
            name: "tls".to_string(),
            description: "TLS and security test cases".to_string(),
            tests: vec![6, 7, 8], // TLS termination, redirect, backend TLS
            rounds: 1,
            parallel: false,
            timeout_secs: 60,
            tags: vec!["tls".to_string(), "security".to_string()],
        }
    }

    /// Traffic management tests profile
    pub fn traffic() -> Self {
        Self {
            name: "traffic".to_string(),
            description: "Traffic management test cases".to_string(),
            tests: vec![9, 10, 11, 12], // Canary, rate limit, retry, timeout
            rounds: 3,
            parallel: false,
            timeout_secs: 60,
            tags: vec!["traffic".to_string()],
        }
    }

    /// Advanced features tests profile
    pub fn advanced() -> Self {
        Self {
            name: "advanced".to_string(),
            description: "Advanced features test cases".to_string(),
            tests: vec![13, 14, 15, 16, 17], // Session, rewrite, gRPC, load test, observability
            rounds: 1,
            parallel: false,
            timeout_secs: 120,
            tags: vec!["advanced".to_string()],
        }
    }

    /// Performance test profile
    pub fn performance() -> Self {
        Self {
            name: "performance".to_string(),
            description: "Performance and load testing".to_string(),
            tests: vec![16], // Load test
            rounds: 10,
            parallel: false,
            timeout_secs: 300,
            tags: vec!["performance".to_string(), "load".to_string()],
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set test numbers
    pub fn with_tests(mut self, tests: Vec<u8>) -> Self {
        self.tests = tests;
        self
    }

    /// Set rounds
    pub fn with_rounds(mut self, rounds: u32) -> Self {
        self.rounds = rounds;
        self
    }

    /// Set parallel execution
    pub fn parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    /// Add tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Get predefined profiles
    pub fn predefined() -> Vec<TestProfile> {
        vec![
            Self::all(),
            Self::smoke(),
            Self::routing(),
            Self::tls(),
            Self::traffic(),
            Self::advanced(),
            Self::performance(),
        ]
    }

    /// Find profile by name
    pub fn find(name: &str) -> Option<TestProfile> {
        Self::predefined().into_iter().find(|p| p.name == name)
    }
}

/// Profile manager for loading/saving profiles
pub struct ProfileManager {
    gateway_profiles: HashMap<String, GatewayProfile>,
    test_profiles: HashMap<String, TestProfile>,
}

impl ProfileManager {
    /// Create a new profile manager with defaults
    pub fn new() -> Self {
        let mut manager = Self {
            gateway_profiles: HashMap::new(),
            test_profiles: HashMap::new(),
        };

        // Load default gateway profiles
        for gateway in GatewayImpl::all() {
            let profile = GatewayProfile::default_for(gateway);
            manager
                .gateway_profiles
                .insert(profile.name.clone(), profile);
        }

        // Load default test profiles
        for profile in TestProfile::predefined() {
            manager.test_profiles.insert(profile.name.clone(), profile);
        }

        manager
    }

    /// Get gateway profile by name
    pub fn gateway_profile(&self, name: &str) -> Option<&GatewayProfile> {
        self.gateway_profiles.get(name)
    }

    /// Get test profile by name
    pub fn test_profile(&self, name: &str) -> Option<&TestProfile> {
        self.test_profiles.get(name)
    }

    /// Add gateway profile
    pub fn add_gateway_profile(&mut self, profile: GatewayProfile) {
        self.gateway_profiles.insert(profile.name.clone(), profile);
    }

    /// Add test profile
    pub fn add_test_profile(&mut self, profile: TestProfile) {
        self.test_profiles.insert(profile.name.clone(), profile);
    }

    /// List gateway profiles
    pub fn list_gateway_profiles(&self) -> Vec<&GatewayProfile> {
        self.gateway_profiles.values().collect()
    }

    /// List test profiles
    pub fn list_test_profiles(&self) -> Vec<&TestProfile> {
        self.test_profiles.values().collect()
    }
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_profile_default() {
        let profile = GatewayProfile::default_for(GatewayImpl::Nginx);
        assert_eq!(profile.gateway, GatewayImpl::Nginx);
        assert_eq!(profile.namespace, "nginx-gateway");
    }

    #[test]
    fn test_test_profile_smoke() {
        let profile = TestProfile::smoke();
        assert_eq!(profile.name, "smoke");
        assert_eq!(profile.tests, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_test_profile_all() {
        let profile = TestProfile::all();
        assert_eq!(profile.tests.len(), 17);
    }

    #[test]
    fn test_profile_manager() {
        let manager = ProfileManager::new();
        assert!(manager.gateway_profile("nginx-default").is_some());
        assert!(manager.test_profile("smoke").is_some());
    }

    #[test]
    fn test_predefined_profiles() {
        let profiles = TestProfile::predefined();
        assert!(profiles.len() >= 6);
    }

    #[test]
    fn test_find_profile() {
        let profile = TestProfile::find("routing");
        assert!(profile.is_some());
        assert_eq!(profile.unwrap().name, "routing");
    }
}
