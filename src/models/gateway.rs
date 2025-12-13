//! Gateway implementation models
//!
//! Defines the 7 Gateway implementations being tested.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported Gateway implementations
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GatewayImpl {
    Nginx,
    Envoy,
    Istio,
    Cilium,
    Kong,
    Traefik,
    Kgateway,
}

impl GatewayImpl {
    /// Get gateway display name
    pub fn name(&self) -> &'static str {
        match self {
            GatewayImpl::Nginx => "NGINX Gateway Fabric",
            GatewayImpl::Envoy => "Envoy Gateway",
            GatewayImpl::Istio => "Istio Gateway",
            GatewayImpl::Cilium => "Cilium Gateway",
            GatewayImpl::Kong => "Kong Gateway",
            GatewayImpl::Traefik => "Traefik Gateway",
            GatewayImpl::Kgateway => "kgateway",
        }
    }

    /// Check if ARM64 is supported
    pub fn supports_arm64(&self) -> bool {
        !matches!(self, GatewayImpl::Kgateway)
    }

    /// Get GatewayClass name
    pub fn gateway_class(&self) -> &'static str {
        match self {
            GatewayImpl::Nginx => "nginx",
            GatewayImpl::Envoy => "eg",
            GatewayImpl::Istio => "istio",
            GatewayImpl::Cilium => "cilium",
            GatewayImpl::Kong => "kong",
            GatewayImpl::Traefik => "traefik",
            GatewayImpl::Kgateway => "kgateway",
        }
    }

    /// Get all gateway implementations
    pub fn all() -> Vec<GatewayImpl> {
        vec![
            GatewayImpl::Nginx,
            GatewayImpl::Envoy,
            GatewayImpl::Istio,
            GatewayImpl::Cilium,
            GatewayImpl::Kong,
            GatewayImpl::Traefik,
            GatewayImpl::Kgateway,
        ]
    }

    /// Get ARM64 compatible gateways
    pub fn arm64_compatible() -> Vec<GatewayImpl> {
        Self::all()
            .into_iter()
            .filter(|g| g.supports_arm64())
            .collect()
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<GatewayImpl> {
        match s.to_lowercase().as_str() {
            "nginx" | "nginx-gateway-fabric" => Some(GatewayImpl::Nginx),
            "envoy" | "envoy-gateway" | "eg" => Some(GatewayImpl::Envoy),
            "istio" | "istio-gateway" => Some(GatewayImpl::Istio),
            "cilium" | "cilium-gateway" => Some(GatewayImpl::Cilium),
            "kong" | "kong-gateway" => Some(GatewayImpl::Kong),
            "traefik" | "traefik-gateway" => Some(GatewayImpl::Traefik),
            "kgateway" | "gloo" => Some(GatewayImpl::Kgateway),
            _ => None,
        }
    }
}

impl fmt::Display for GatewayImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Gateway configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub implementation: GatewayImpl,
    pub namespace: String,
    pub name: String,
    pub http_port: u16,
    pub https_port: u16,
    pub grpc_port: Option<u16>,
    pub hostname: String,
}

impl GatewayConfig {
    pub fn new(implementation: GatewayImpl) -> Self {
        Self {
            implementation,
            namespace: "default".to_string(),
            name: format!("{}-gateway", implementation.gateway_class()),
            http_port: 80,
            https_port: 443,
            grpc_port: Some(9090),
            hostname: "example.com".to_string(),
        }
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = hostname.into();
        self
    }

    pub fn with_ports(mut self, http: u16, https: u16, grpc: Option<u16>) -> Self {
        self.http_port = http;
        self.https_port = https;
        self.grpc_port = grpc;
        self
    }
}

/// Gateway test configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestConfig {
    pub gateway: GatewayConfig,
    pub rounds: u32,
    pub parallel: bool,
    pub timeout_secs: u64,
    pub skip_tests: Vec<u8>,
}

impl TestConfig {
    pub fn new(gateway: GatewayConfig) -> Self {
        Self {
            gateway,
            rounds: 1,
            parallel: false,
            timeout_secs: 30,
            skip_tests: Vec::new(),
        }
    }

    pub fn with_rounds(mut self, rounds: u32) -> Self {
        self.rounds = rounds;
        self
    }

    pub fn parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    pub fn skip_test(mut self, test_number: u8) -> Self {
        self.skip_tests.push(test_number);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_impl() {
        assert_eq!(GatewayImpl::Nginx.name(), "NGINX Gateway Fabric");
        assert!(GatewayImpl::Nginx.supports_arm64());
        assert!(!GatewayImpl::Kgateway.supports_arm64());
    }

    #[test]
    fn test_gateway_from_str() {
        assert_eq!(GatewayImpl::from_str("nginx"), Some(GatewayImpl::Nginx));
        assert_eq!(GatewayImpl::from_str("ENVOY"), Some(GatewayImpl::Envoy));
        assert_eq!(GatewayImpl::from_str("unknown"), None);
    }

    #[test]
    fn test_all_gateways() {
        let all = GatewayImpl::all();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn test_arm64_compatible() {
        let arm64 = GatewayImpl::arm64_compatible();
        assert_eq!(arm64.len(), 6);
        assert!(!arm64.contains(&GatewayImpl::Kgateway));
    }

    #[test]
    fn test_gateway_config() {
        let config = GatewayConfig::new(GatewayImpl::Nginx)
            .with_namespace("gateway-system")
            .with_hostname("test.example.com");

        assert_eq!(config.namespace, "gateway-system");
        assert_eq!(config.hostname, "test.example.com");
    }
}
