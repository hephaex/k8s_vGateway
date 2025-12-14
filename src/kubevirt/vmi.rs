//! VirtualMachineInstance resource management
//!
//! Provides monitoring and status checking for KubeVirt VMI resources.

use anyhow::{Context, Result};
use kube::api::{Api, ListParams};
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::k8s::K8sClient;

/// VirtualMachineInstance custom resource specification
#[derive(CustomResource, Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[kube(
    group = "kubevirt.io",
    version = "v1",
    kind = "VirtualMachineInstance",
    plural = "virtualmachineinstances",
    shortname = "vmi",
    namespaced,
    status = "VirtualMachineInstanceStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMachineInstanceSpec {
    /// Domain specification (same as VM template)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<serde_json::Value>,

    /// Networks
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub networks: Vec<serde_json::Value>,

    /// Volumes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<serde_json::Value>,
}

/// VMI Status
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMachineInstanceStatus {
    /// Current phase
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,

    /// Node name where VMI is running
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_name: Option<String>,

    /// Conditions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<VmiCondition>,

    /// Interfaces with IP addresses
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<VmiInterface>,

    /// Guest OS info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guest_os_info: Option<GuestOsInfo>,

    /// Active pods
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub active_pods: BTreeMap<String, String>,

    /// Migration state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_state: Option<MigrationState>,

    /// Launcher container image version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launcher_container_image_version: Option<String>,
}

/// VMI Condition
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VmiCondition {
    /// Condition type
    #[serde(rename = "type")]
    pub condition_type: String,

    /// Condition status
    pub status: String,

    /// Last probe time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_probe_time: Option<String>,

    /// Last transition time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_transition_time: Option<String>,

    /// Reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// VMI network interface
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VmiInterface {
    /// Interface name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// MAC address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,

    /// IP address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,

    /// IP addresses (multiple)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ip_addresses: Vec<String>,

    /// Interface name inside guest
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface_name: Option<String>,

    /// Info source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_source: Option<String>,
}

/// Guest OS information
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GuestOsInfo {
    /// Guest agent version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// OS name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// OS ID (e.g., "fedora", "ubuntu")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Kernel release
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_release: Option<String>,

    /// Kernel version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_version: Option<String>,

    /// Machine type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine: Option<String>,

    /// Pretty name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pretty_name: Option<String>,

    /// Version ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
}

/// Migration state
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MigrationState {
    /// Migration completed
    #[serde(default)]
    pub completed: bool,

    /// Migration failed
    #[serde(default)]
    pub failed: bool,

    /// Source node
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_node: Option<String>,

    /// Target node
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_node: Option<String>,

    /// Target pod
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_pod: Option<String>,

    /// Start timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_timestamp: Option<String>,

    /// End timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_timestamp: Option<String>,
}

/// VMI phases
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmiPhase {
    Pending,
    Scheduling,
    Scheduled,
    Running,
    Succeeded,
    Failed,
    Unknown,
}

impl VmiPhase {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => VmiPhase::Pending,
            "scheduling" => VmiPhase::Scheduling,
            "scheduled" => VmiPhase::Scheduled,
            "running" => VmiPhase::Running,
            "succeeded" => VmiPhase::Succeeded,
            "failed" => VmiPhase::Failed,
            _ => VmiPhase::Unknown,
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self, VmiPhase::Running)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, VmiPhase::Succeeded | VmiPhase::Failed)
    }
}

/// VMI Manager for monitoring and operations
pub struct VmiManager {
    client: K8sClient,
}

impl VmiManager {
    /// Create a new VMI manager
    pub fn new(client: K8sClient) -> Self {
        Self { client }
    }

    fn api(&self, namespace: &str) -> Api<VirtualMachineInstance> {
        Api::namespaced(self.client.client().clone(), namespace)
    }

    /// Get a VMI
    pub async fn get(&self, name: &str, namespace: &str) -> Result<VirtualMachineInstance> {
        let api = self.api(namespace);
        api.get(name)
            .await
            .context("Failed to get VirtualMachineInstance")
    }

    /// List VMIs
    pub async fn list(&self, namespace: &str) -> Result<Vec<VirtualMachineInstance>> {
        let api = self.api(namespace);
        let vmis = api
            .list(&ListParams::default())
            .await
            .context("Failed to list VirtualMachineInstances")?;
        Ok(vmis.items)
    }

    /// List VMIs with label selector
    pub async fn list_with_labels(
        &self,
        namespace: &str,
        labels: &str,
    ) -> Result<Vec<VirtualMachineInstance>> {
        let api = self.api(namespace);
        let vmis = api
            .list(&ListParams::default().labels(labels))
            .await
            .context("Failed to list VirtualMachineInstances")?;
        Ok(vmis.items)
    }

    /// Get VMI phase
    pub async fn get_phase(&self, name: &str, namespace: &str) -> Result<VmiPhase> {
        let vmi = self.get(name, namespace).await?;
        let phase = vmi
            .status
            .and_then(|s| s.phase)
            .unwrap_or_else(|| "unknown".to_string());
        Ok(VmiPhase::from_str(&phase))
    }

    /// Check if VMI is running
    pub async fn is_running(&self, name: &str, namespace: &str) -> Result<bool> {
        let phase = self.get_phase(name, namespace).await?;
        Ok(phase.is_running())
    }

    /// Get VMI IP address
    pub async fn get_ip(&self, name: &str, namespace: &str) -> Result<Option<String>> {
        let vmi = self.get(name, namespace).await?;

        if let Some(status) = vmi.status {
            // Try to find IP from interfaces
            for iface in &status.interfaces {
                if let Some(ref ip) = iface.ip_address {
                    if !ip.is_empty() {
                        return Ok(Some(ip.clone()));
                    }
                }
                if !iface.ip_addresses.is_empty() {
                    return Ok(Some(iface.ip_addresses[0].clone()));
                }
            }
        }

        Ok(None)
    }

    /// Wait for VMI to be running
    pub async fn wait_running(
        &self,
        name: &str,
        namespace: &str,
        timeout_secs: u64,
    ) -> Result<bool> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        info!(
            "Waiting for VMI {}/{} to be running (timeout: {}s)",
            namespace, name, timeout_secs
        );

        loop {
            if start.elapsed() > timeout {
                warn!(
                    "Timeout waiting for VMI {}/{} to be running",
                    namespace, name
                );
                return Ok(false);
            }

            match self.get_phase(name, namespace).await {
                Ok(phase) => {
                    debug!("VMI {}/{} phase: {:?}", namespace, name, phase);

                    if phase.is_running() {
                        info!("VMI {}/{} is running", namespace, name);
                        return Ok(true);
                    }

                    if phase.is_terminal() {
                        warn!(
                            "VMI {}/{} is in terminal phase: {:?}",
                            namespace, name, phase
                        );
                        return Ok(false);
                    }
                }
                Err(e) => {
                    debug!("Error getting VMI phase: {}", e);
                }
            }

            sleep(Duration::from_secs(5)).await;
        }
    }

    /// Wait for VMI to have an IP address
    pub async fn wait_for_ip(
        &self,
        name: &str,
        namespace: &str,
        timeout_secs: u64,
    ) -> Result<Option<String>> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        info!(
            "Waiting for VMI {}/{} to get IP address (timeout: {}s)",
            namespace, name, timeout_secs
        );

        loop {
            if start.elapsed() > timeout {
                warn!(
                    "Timeout waiting for VMI {}/{} to get IP address",
                    namespace, name
                );
                return Ok(None);
            }

            if let Ok(Some(ip)) = self.get_ip(name, namespace).await {
                info!("VMI {}/{} has IP: {}", namespace, name, ip);
                return Ok(Some(ip));
            }

            sleep(Duration::from_secs(5)).await;
        }
    }

    /// Get VMI conditions
    pub async fn get_conditions(&self, name: &str, namespace: &str) -> Result<Vec<VmiCondition>> {
        let vmi = self.get(name, namespace).await?;
        Ok(vmi.status.map(|s| s.conditions).unwrap_or_default())
    }

    /// Check if VMI has a specific condition
    pub async fn has_condition(
        &self,
        name: &str,
        namespace: &str,
        condition_type: &str,
        status: &str,
    ) -> Result<bool> {
        let conditions = self.get_conditions(name, namespace).await?;

        Ok(conditions
            .iter()
            .any(|c| c.condition_type == condition_type && c.status.eq_ignore_ascii_case(status)))
    }

    /// Get node where VMI is running
    pub async fn get_node(&self, name: &str, namespace: &str) -> Result<Option<String>> {
        let vmi = self.get(name, namespace).await?;
        Ok(vmi.status.and_then(|s| s.node_name))
    }

    /// Get guest OS info
    pub async fn get_guest_os_info(
        &self,
        name: &str,
        namespace: &str,
    ) -> Result<Option<GuestOsInfo>> {
        let vmi = self.get(name, namespace).await?;
        Ok(vmi.status.and_then(|s| s.guest_os_info))
    }

    /// Get VMI summary
    pub async fn get_summary(&self, name: &str, namespace: &str) -> Result<VmiSummary> {
        let vmi = self.get(name, namespace).await?;
        let status = vmi.status.unwrap_or_default();

        let phase = status
            .phase
            .as_deref()
            .map(VmiPhase::from_str)
            .unwrap_or(VmiPhase::Unknown);

        let ip = status.interfaces.first().and_then(|i| i.ip_address.clone());

        Ok(VmiSummary {
            name: vmi.metadata.name.unwrap_or_default(),
            namespace: vmi.metadata.namespace.unwrap_or_default(),
            phase,
            node: status.node_name,
            ip,
            conditions: status.conditions,
        })
    }
}

/// Summary of VMI state
#[derive(Clone, Debug)]
pub struct VmiSummary {
    pub name: String,
    pub namespace: String,
    pub phase: VmiPhase,
    pub node: Option<String>,
    pub ip: Option<String>,
    pub conditions: Vec<VmiCondition>,
}

impl VmiSummary {
    pub fn is_ready(&self) -> bool {
        self.phase.is_running()
            && self
                .conditions
                .iter()
                .any(|c| c.condition_type == "Ready" && c.status.eq_ignore_ascii_case("true"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vmi_phase_from_str() {
        assert_eq!(VmiPhase::from_str("Running"), VmiPhase::Running);
        assert_eq!(VmiPhase::from_str("pending"), VmiPhase::Pending);
        assert_eq!(VmiPhase::from_str("FAILED"), VmiPhase::Failed);
        assert_eq!(VmiPhase::from_str("unknown-phase"), VmiPhase::Unknown);
    }

    #[test]
    fn test_vmi_phase_predicates() {
        assert!(VmiPhase::Running.is_running());
        assert!(!VmiPhase::Pending.is_running());

        assert!(VmiPhase::Succeeded.is_terminal());
        assert!(VmiPhase::Failed.is_terminal());
        assert!(!VmiPhase::Running.is_terminal());
    }

    #[test]
    fn test_vmi_interface() {
        let iface = VmiInterface {
            name: Some("default".to_string()),
            ip_address: Some("10.244.0.5".to_string()),
            mac: Some("52:54:00:12:34:56".to_string()),
            ..Default::default()
        };

        assert_eq!(iface.ip_address.as_deref(), Some("10.244.0.5"));
    }

    #[test]
    fn test_vmi_condition() {
        let cond = VmiCondition {
            condition_type: "Ready".to_string(),
            status: "True".to_string(),
            reason: Some("VMIReady".to_string()),
            ..Default::default()
        };

        assert_eq!(cond.condition_type, "Ready");
        assert_eq!(cond.status, "True");
    }
}
