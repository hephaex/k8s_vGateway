//! Gateway resource management
//!
//! Provides CRUD operations for Gateway resources.

#![allow(dead_code)]

use anyhow::{Context, Result};
use kube::api::{Api, ListParams, PostParams};
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::K8sClient;

/// Gateway custom resource specification
#[derive(CustomResource, Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[kube(
    group = "gateway.networking.k8s.io",
    version = "v1",
    kind = "Gateway",
    namespaced
)]
#[kube(status = "GatewayStatus")]
pub struct GatewaySpec {
    /// GatewayClass name
    #[serde(rename = "gatewayClassName")]
    pub gateway_class_name: String,

    /// Listeners for the gateway
    #[serde(default)]
    pub listeners: Vec<ListenerSpec>,

    /// Addresses for the gateway
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub addresses: Vec<AddressSpec>,
}

/// Gateway listener specification
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct ListenerSpec {
    /// Listener name
    pub name: String,

    /// Port number
    pub port: u16,

    /// Protocol (HTTP, HTTPS, TLS, TCP, UDP)
    pub protocol: String,

    /// Hostname for this listener
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,

    /// TLS configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsConfig>,

    /// Allowed routes
    #[serde(rename = "allowedRoutes", skip_serializing_if = "Option::is_none")]
    pub allowed_routes: Option<AllowedRoutes>,
}

/// TLS configuration for listener
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct TlsConfig {
    /// TLS mode (Terminate, Passthrough)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,

    /// Certificate references
    #[serde(rename = "certificateRefs", default)]
    pub certificate_refs: Vec<CertificateRef>,
}

/// Certificate reference
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct CertificateRef {
    /// Name of the secret
    pub name: String,

    /// Kind (usually "Secret")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,

    /// Namespace of the secret
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

/// Allowed routes configuration
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct AllowedRoutes {
    /// Namespaces from which routes may be attached
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespaces: Option<RouteNamespaces>,

    /// Kinds of routes that may be attached
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<RouteGroupKind>>,
}

/// Route namespace selector
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct RouteNamespaces {
    /// From: All, Same, Selector
    pub from: String,
}

/// Route group kind
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct RouteGroupKind {
    /// API group
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,

    /// Kind name
    pub kind: String,
}

/// Gateway address specification
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct AddressSpec {
    /// Address type (IPAddress, Hostname)
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub address_type: Option<String>,

    /// Address value
    pub value: String,
}

/// Gateway status
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct GatewayStatus {
    /// Addresses assigned to the gateway
    #[serde(default)]
    pub addresses: Vec<AddressSpec>,

    /// Conditions
    #[serde(default)]
    pub conditions: Vec<GatewayCondition>,

    /// Listener statuses
    #[serde(default)]
    pub listeners: Vec<ListenerStatus>,
}

/// Gateway condition
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct GatewayCondition {
    /// Condition type
    #[serde(rename = "type")]
    pub condition_type: String,

    /// Status (True, False, Unknown)
    pub status: String,

    /// Reason for the condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Human-readable message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Listener status
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct ListenerStatus {
    /// Listener name
    pub name: String,

    /// Number of attached routes
    #[serde(rename = "attachedRoutes")]
    pub attached_routes: i32,

    /// Conditions
    #[serde(default)]
    pub conditions: Vec<GatewayCondition>,
}

/// Gateway resource manager
pub struct GatewayManager {
    client: K8sClient,
}

impl GatewayManager {
    /// Create a new gateway manager
    pub fn new(client: K8sClient) -> Self {
        Self { client }
    }

    /// Get gateway API
    fn api(&self, namespace: &str) -> Api<Gateway> {
        Api::namespaced(self.client.client().clone(), namespace)
    }

    /// Create a gateway
    pub async fn create(&self, gateway: &Gateway, namespace: &str) -> Result<Gateway> {
        let api = self.api(namespace);
        api.create(&PostParams::default(), gateway)
            .await
            .context("Failed to create Gateway")
    }

    /// Get a gateway by name
    pub async fn get(&self, name: &str, namespace: &str) -> Result<Gateway> {
        let api = self.api(namespace);
        api.get(name).await.context("Failed to get Gateway")
    }

    /// List gateways in namespace
    pub async fn list(&self, namespace: &str) -> Result<Vec<Gateway>> {
        let api = self.api(namespace);
        let list = api
            .list(&ListParams::default())
            .await
            .context("Failed to list Gateways")?;
        Ok(list.items)
    }

    /// Delete a gateway
    pub async fn delete(&self, name: &str, namespace: &str) -> Result<()> {
        let api = self.api(namespace);
        api.delete(name, &Default::default())
            .await
            .context("Failed to delete Gateway")?;
        Ok(())
    }

    /// Check if gateway is ready
    pub async fn is_gateway_ready(&self, name: &str, namespace: &str) -> Result<bool> {
        let gateway = self.get(name, namespace).await?;

        if let Some(status) = &gateway.status {
            let ready = status
                .conditions
                .iter()
                .any(|c| c.condition_type == "Accepted" && c.status == "True");
            return Ok(ready);
        }

        Ok(false)
    }

    /// Get gateway IP address
    pub async fn get_gateway_ip(&self, name: &str, namespace: &str) -> Result<Option<String>> {
        let gateway = self.get(name, namespace).await?;

        if let Some(status) = &gateway.status {
            for addr in &status.addresses {
                if addr.address_type.as_deref() == Some("IPAddress") {
                    return Ok(Some(addr.value.clone()));
                }
            }
            // Fallback to first address
            if let Some(addr) = status.addresses.first() {
                return Ok(Some(addr.value.clone()));
            }
        }

        Ok(None)
    }

    /// Wait for gateway to be ready
    pub async fn wait_ready(&self, name: &str, namespace: &str, timeout_secs: u64) -> Result<bool> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            if self.is_gateway_ready(name, namespace).await? {
                info!("Gateway {} is ready", name);
                return Ok(true);
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        warn!(
            "Gateway {} did not become ready within {}s",
            name, timeout_secs
        );
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_spec() {
        let spec = GatewaySpec {
            gateway_class_name: "nginx".to_string(),
            listeners: vec![ListenerSpec {
                name: "http".to_string(),
                port: 80,
                protocol: "HTTP".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };

        assert_eq!(spec.gateway_class_name, "nginx");
        assert_eq!(spec.listeners.len(), 1);
    }

    #[test]
    fn test_listener_spec() {
        let listener = ListenerSpec {
            name: "https".to_string(),
            port: 443,
            protocol: "HTTPS".to_string(),
            hostname: Some("example.com".to_string()),
            tls: Some(TlsConfig {
                mode: Some("Terminate".to_string()),
                certificate_refs: vec![CertificateRef {
                    name: "tls-secret".to_string(),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };

        assert_eq!(listener.port, 443);
        assert!(listener.tls.is_some());
    }
}
