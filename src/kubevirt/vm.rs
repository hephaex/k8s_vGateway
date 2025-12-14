//! VirtualMachine resource management
//!
//! Provides CRUD operations for KubeVirt VirtualMachine resources.

use anyhow::{Context, Result};
use kube::api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams};
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::k8s::K8sClient;

/// VirtualMachine custom resource specification
#[derive(CustomResource, Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[kube(
    group = "kubevirt.io",
    version = "v1",
    kind = "VirtualMachine",
    plural = "virtualmachines",
    shortname = "vm",
    namespaced,
    status = "VirtualMachineStatus"
)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMachineSpec {
    /// Whether the VM should be running
    #[serde(default)]
    pub running: bool,

    /// Run strategy for the VM
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_strategy: Option<String>,

    /// Template for the VMI
    pub template: VmiTemplate,
}

/// VMI Template specification
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VmiTemplate {
    /// Metadata for the VMI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<VmiTemplateMetadata>,

    /// Spec for the VMI
    pub spec: VmiTemplateSpec,
}

/// VMI Template metadata
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VmiTemplateMetadata {
    /// Labels for the VMI
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,

    /// Annotations for the VMI
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
}

/// VMI Template spec
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VmiTemplateSpec {
    /// Domain specification
    pub domain: DomainSpec,

    /// Networks configuration
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub networks: Vec<Network>,

    /// Volumes configuration
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<Volume>,

    /// Termination grace period
    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination_grace_period_seconds: Option<i64>,
}

/// Domain specification for the VM
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DomainSpec {
    /// CPU configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<CpuSpec>,

    /// Memory configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemorySpec>,

    /// Resources configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesSpec>,

    /// Devices configuration
    pub devices: DevicesSpec,

    /// Machine type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine: Option<MachineSpec>,
}

/// CPU specification
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CpuSpec {
    /// Number of CPU cores
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cores: Option<u32>,

    /// Number of CPU sockets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sockets: Option<u32>,

    /// Number of threads per core
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<u32>,

    /// CPU model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Dedicated CPU
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedicated_cpu_placement: Option<bool>,
}

/// Memory specification
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemorySpec {
    /// Guest memory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guest: Option<String>,

    /// Hugepages configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hugepages: Option<HugepagesSpec>,
}

/// Hugepages specification
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HugepagesSpec {
    /// Page size (e.g., "2Mi", "1Gi")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<String>,
}

/// Resources specification
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesSpec {
    /// Resource requests
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub requests: BTreeMap<String, String>,

    /// Resource limits
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub limits: BTreeMap<String, String>,
}

/// Devices specification
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DevicesSpec {
    /// Disk devices
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disks: Vec<Disk>,

    /// Network interfaces
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<Interface>,

    /// Whether to auto-attach graphics device
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_attach_graphics_device: Option<bool>,

    /// Whether to auto-attach memory balloon
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_attach_mem_balloon: Option<bool>,

    /// RNG device
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rng: Option<RngDevice>,
}

/// RNG device
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct RngDevice {}

/// Disk device
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Disk {
    /// Disk name (must match volume name)
    pub name: String,

    /// Disk type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk: Option<DiskTarget>,

    /// CD-ROM type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cdrom: Option<CdromTarget>,

    /// Boot order
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_order: Option<u32>,
}

/// Disk target configuration
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiskTarget {
    /// Bus type (virtio, sata, scsi)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,
}

/// CD-ROM target configuration
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CdromTarget {
    /// Bus type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,

    /// Read only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly: Option<bool>,
}

/// Network interface
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Interface {
    /// Interface name (must match network name)
    pub name: String,

    /// Masquerade mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub masquerade: Option<MasqueradeMode>,

    /// Bridge mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge: Option<BridgeMode>,

    /// SR-IOV mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sriov: Option<SriovMode>,

    /// MAC address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,

    /// Model (e.g., virtio, e1000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Masquerade network mode
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct MasqueradeMode {}

/// Bridge network mode
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct BridgeMode {}

/// SR-IOV network mode
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
pub struct SriovMode {}

/// Machine type specification
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MachineSpec {
    /// Machine type (e.g., q35)
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub machine_type: Option<String>,
}

/// Network configuration
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    /// Network name
    pub name: String,

    /// Pod network
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod: Option<PodNetwork>,

    /// Multus network
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multus: Option<MultusNetwork>,
}

/// Pod network configuration
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PodNetwork {
    /// VM network CIDR
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vm_network_cidr: Option<String>,
}

/// Multus network configuration
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MultusNetwork {
    /// Network attachment definition name
    pub network_name: String,
}

/// Volume configuration
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    /// Volume name
    pub name: String,

    /// Container disk source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_disk: Option<ContainerDiskSource>,

    /// Cloud-init config drive
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_init_config_drive: Option<CloudInitConfigDrive>,

    /// Cloud-init no cloud
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_init_no_cloud: Option<CloudInitNoCloud>,

    /// PVC source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent_volume_claim: Option<PvcSource>,

    /// DataVolume source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_volume: Option<DataVolumeSource>,

    /// Empty disk
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_disk: Option<EmptyDiskSource>,
}

/// Container disk source
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerDiskSource {
    /// Container image
    pub image: String,

    /// Image pull policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_pull_policy: Option<String>,
}

/// Cloud-init config drive
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudInitConfigDrive {
    /// User data (base64 encoded or plain text)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_data: Option<String>,

    /// Network data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_data: Option<String>,

    /// Secret reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_ref: Option<SecretRef>,
}

/// Cloud-init no cloud
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CloudInitNoCloud {
    /// User data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_data: Option<String>,

    /// Network data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_data: Option<String>,

    /// Secret reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_ref: Option<SecretRef>,
}

/// Secret reference
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecretRef {
    /// Secret name
    pub name: String,
}

/// PVC source
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PvcSource {
    /// PVC name
    pub claim_name: String,

    /// Read only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
}

/// DataVolume source
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataVolumeSource {
    /// DataVolume name
    pub name: String,
}

/// Empty disk source
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EmptyDiskSource {
    /// Disk capacity
    pub capacity: String,
}

/// VirtualMachine status
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VirtualMachineStatus {
    /// Whether the VM is created
    #[serde(default)]
    pub created: bool,

    /// Whether the VM is ready
    #[serde(default)]
    pub ready: bool,

    /// Print column data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub printable_status: Option<String>,

    /// Volume snapshot statuses
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volume_snapshot_statuses: Vec<VolumeSnapshotStatus>,

    /// Conditions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<VmCondition>,
}

/// Volume snapshot status
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VolumeSnapshotStatus {
    /// Volume name
    pub name: String,

    /// Whether snapshot is enabled
    #[serde(default)]
    pub enabled: bool,
}

/// VM condition
#[derive(Clone, Debug, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VmCondition {
    /// Condition type
    #[serde(rename = "type")]
    pub condition_type: String,

    /// Condition status (True, False, Unknown)
    pub status: String,

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

/// VM configuration for easy creation
#[derive(Clone, Debug)]
pub struct VmConfig {
    pub name: String,
    pub namespace: String,
    pub cpu_cores: u32,
    pub memory: String,
    pub image: String,
    pub ssh_public_key: Option<String>,
    pub labels: BTreeMap<String, String>,
    pub network_type: NetworkType,
}

/// Network type for VM
#[derive(Clone, Debug, Default)]
pub enum NetworkType {
    #[default]
    Masquerade,
    Bridge,
    Multus(String),
}

impl VmConfig {
    /// Create a new VM configuration
    pub fn new(name: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: namespace.into(),
            cpu_cores: 1,
            memory: "1Gi".to_string(),
            image: "quay.io/containerdisks/fedora:latest".to_string(),
            ssh_public_key: None,
            labels: BTreeMap::new(),
            network_type: NetworkType::Masquerade,
        }
    }

    /// Set CPU cores
    pub fn cpu(mut self, cores: u32) -> Self {
        self.cpu_cores = cores;
        self
    }

    /// Set memory
    pub fn memory(mut self, memory: impl Into<String>) -> Self {
        self.memory = memory.into();
        self
    }

    /// Set container disk image
    pub fn image(mut self, image: impl Into<String>) -> Self {
        self.image = image.into();
        self
    }

    /// Set SSH public key for cloud-init
    pub fn ssh_key(mut self, key: impl Into<String>) -> Self {
        self.ssh_public_key = Some(key.into());
        self
    }

    /// Add label
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Set network type
    pub fn network(mut self, network_type: NetworkType) -> Self {
        self.network_type = network_type;
        self
    }

    /// Build the VirtualMachine resource
    pub fn build(self) -> VirtualMachine {
        let has_ssh_key = self.ssh_public_key.is_some();
        let cloud_init = self.ssh_public_key.map(|key| {
            let user_data = format!(
                r#"#cloud-config
user: fedora
password: fedora
chpasswd:
  expire: false
ssh_authorized_keys:
  - {key}
"#
            );
            CloudInitNoCloud {
                user_data: Some(user_data),
                network_data: None,
                secret_ref: None,
            }
        });

        let interface = match &self.network_type {
            NetworkType::Masquerade => Interface {
                name: "default".to_string(),
                masquerade: Some(MasqueradeMode {}),
                ..Default::default()
            },
            NetworkType::Bridge => Interface {
                name: "default".to_string(),
                bridge: Some(BridgeMode {}),
                ..Default::default()
            },
            NetworkType::Multus(_) => Interface {
                name: "default".to_string(),
                bridge: Some(BridgeMode {}),
                ..Default::default()
            },
        };

        let network = match &self.network_type {
            NetworkType::Masquerade | NetworkType::Bridge => Network {
                name: "default".to_string(),
                pod: Some(PodNetwork::default()),
                multus: None,
            },
            NetworkType::Multus(net_name) => Network {
                name: "default".to_string(),
                pod: None,
                multus: Some(MultusNetwork {
                    network_name: net_name.clone(),
                }),
            },
        };

        let mut volumes = vec![Volume {
            name: "rootdisk".to_string(),
            container_disk: Some(ContainerDiskSource {
                image: self.image,
                image_pull_policy: Some("IfNotPresent".to_string()),
            }),
            ..Default::default()
        }];

        if let Some(ci) = cloud_init {
            volumes.push(Volume {
                name: "cloudinit".to_string(),
                cloud_init_no_cloud: Some(ci),
                ..Default::default()
            });
        }

        let mut disks = vec![Disk {
            name: "rootdisk".to_string(),
            disk: Some(DiskTarget {
                bus: Some("virtio".to_string()),
            }),
            boot_order: Some(1),
            ..Default::default()
        }];

        if has_ssh_key {
            disks.push(Disk {
                name: "cloudinit".to_string(),
                disk: Some(DiskTarget {
                    bus: Some("virtio".to_string()),
                }),
                ..Default::default()
            });
        }

        VirtualMachine {
            metadata: kube::api::ObjectMeta {
                name: Some(self.name),
                namespace: Some(self.namespace),
                labels: if self.labels.is_empty() {
                    None
                } else {
                    Some(self.labels)
                },
                ..Default::default()
            },
            spec: VirtualMachineSpec {
                running: true,
                run_strategy: None,
                template: VmiTemplate {
                    metadata: Some(VmiTemplateMetadata::default()),
                    spec: VmiTemplateSpec {
                        domain: DomainSpec {
                            cpu: Some(CpuSpec {
                                cores: Some(self.cpu_cores),
                                ..Default::default()
                            }),
                            memory: Some(MemorySpec {
                                guest: Some(self.memory),
                                ..Default::default()
                            }),
                            devices: DevicesSpec {
                                disks,
                                interfaces: vec![interface],
                                rng: Some(RngDevice {}),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        networks: vec![network],
                        volumes,
                        termination_grace_period_seconds: Some(30),
                    },
                },
            },
            status: None,
        }
    }
}

/// VirtualMachine manager
pub struct VirtualMachineManager {
    client: K8sClient,
}

impl VirtualMachineManager {
    /// Create a new VM manager
    pub fn new(client: K8sClient) -> Self {
        Self { client }
    }

    fn api(&self, namespace: &str) -> Api<VirtualMachine> {
        Api::namespaced(self.client.client().clone(), namespace)
    }

    /// Create a VirtualMachine
    pub async fn create(&self, vm: &VirtualMachine, namespace: &str) -> Result<VirtualMachine> {
        let api = self.api(namespace);
        api.create(&PostParams::default(), vm)
            .await
            .context("Failed to create VirtualMachine")
    }

    /// Get a VirtualMachine
    pub async fn get(&self, name: &str, namespace: &str) -> Result<VirtualMachine> {
        let api = self.api(namespace);
        api.get(name).await.context("Failed to get VirtualMachine")
    }

    /// List VirtualMachines
    pub async fn list(&self, namespace: &str) -> Result<Vec<VirtualMachine>> {
        let api = self.api(namespace);
        let vms = api
            .list(&ListParams::default())
            .await
            .context("Failed to list VirtualMachines")?;
        Ok(vms.items)
    }

    /// Delete a VirtualMachine
    pub async fn delete(&self, name: &str, namespace: &str) -> Result<()> {
        let api = self.api(namespace);
        api.delete(name, &DeleteParams::default())
            .await
            .context("Failed to delete VirtualMachine")?;
        info!("Deleted VirtualMachine {}/{}", namespace, name);
        Ok(())
    }

    /// Start a VirtualMachine
    pub async fn start(&self, name: &str, namespace: &str) -> Result<()> {
        let api = self.api(namespace);
        let patch = serde_json::json!({
            "spec": {
                "running": true
            }
        });
        api.patch(name, &PatchParams::default(), &Patch::Merge(&patch))
            .await
            .context("Failed to start VirtualMachine")?;
        info!("Started VirtualMachine {}/{}", namespace, name);
        Ok(())
    }

    /// Stop a VirtualMachine
    pub async fn stop(&self, name: &str, namespace: &str) -> Result<()> {
        let api = self.api(namespace);
        let patch = serde_json::json!({
            "spec": {
                "running": false
            }
        });
        api.patch(name, &PatchParams::default(), &Patch::Merge(&patch))
            .await
            .context("Failed to stop VirtualMachine")?;
        info!("Stopped VirtualMachine {}/{}", namespace, name);
        Ok(())
    }

    /// Restart a VirtualMachine
    pub async fn restart(&self, name: &str, namespace: &str) -> Result<()> {
        self.stop(name, namespace).await?;
        sleep(Duration::from_secs(2)).await;
        self.start(name, namespace).await?;
        Ok(())
    }

    /// Wait for VM to be ready
    pub async fn wait_ready(&self, name: &str, namespace: &str, timeout_secs: u64) -> Result<bool> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                warn!(
                    "Timeout waiting for VirtualMachine {}/{} to be ready",
                    namespace, name
                );
                return Ok(false);
            }

            match self.get(name, namespace).await {
                Ok(vm) => {
                    if let Some(status) = &vm.status {
                        if status.ready {
                            info!("VirtualMachine {}/{} is ready", namespace, name);
                            return Ok(true);
                        }
                        debug!(
                            "VirtualMachine {}/{} status: {:?}",
                            namespace, name, status.printable_status
                        );
                    }
                }
                Err(e) => {
                    debug!("Error checking VM status: {}", e);
                }
            }

            sleep(Duration::from_secs(5)).await;
        }
    }

    /// Check if KubeVirt is installed
    pub async fn is_kubevirt_installed(&self) -> Result<bool> {
        self.client
            .crd_exists("kubevirt.io", "v1", "VirtualMachine")
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_config_builder() {
        let vm = VmConfig::new("test-vm", "default")
            .cpu(2)
            .memory("2Gi")
            .image("quay.io/containerdisks/ubuntu:latest")
            .label("app", "gateway-test")
            .build();

        assert_eq!(vm.metadata.name.as_deref(), Some("test-vm"));
        assert_eq!(vm.metadata.namespace.as_deref(), Some("default"));
        assert!(vm.spec.running);
    }

    #[test]
    fn test_vm_config_with_ssh() {
        let vm = VmConfig::new("ssh-vm", "default")
            .ssh_key("ssh-rsa AAAAB3...")
            .build();

        assert_eq!(vm.spec.template.spec.volumes.len(), 2);
        assert!(vm.spec.template.spec.volumes[1]
            .cloud_init_no_cloud
            .is_some());
    }

    #[test]
    fn test_network_types() {
        let masq_vm = VmConfig::new("masq-vm", "default")
            .network(NetworkType::Masquerade)
            .build();
        assert!(masq_vm.spec.template.spec.networks[0].pod.is_some());

        let multus_vm = VmConfig::new("multus-vm", "default")
            .network(NetworkType::Multus("my-network".to_string()))
            .build();
        assert!(multus_vm.spec.template.spec.networks[0].multus.is_some());
    }
}
