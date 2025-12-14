//! Gateway installation and management
//!
//! Installs, configures, and manages Gateway API implementations.

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::k8s::K8sClient;
use crate::models::GatewayImpl;

/// Gateway installer configuration
#[derive(Clone, Debug)]
pub struct InstallerConfig {
    /// Namespace for gateway installation
    pub namespace: String,

    /// Wait timeout in seconds
    pub timeout_secs: u64,

    /// Helm release name prefix
    pub release_prefix: String,

    /// Additional Helm values
    pub helm_values: BTreeMap<String, String>,
}

impl Default for InstallerConfig {
    fn default() -> Self {
        Self {
            namespace: "gateway-system".to_string(),
            timeout_secs: 300,
            release_prefix: "gateway-poc".to_string(),
            helm_values: BTreeMap::new(),
        }
    }
}

impl InstallerConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = ns.into();
        self
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn helm_value(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.helm_values.insert(key.into(), value.into());
        self
    }
}

/// Gateway installer
pub struct GatewayInstaller {
    config: InstallerConfig,
    k8s_client: Option<K8sClient>,
}

impl GatewayInstaller {
    /// Create a new gateway installer
    pub fn new(config: InstallerConfig) -> Self {
        Self {
            config,
            k8s_client: None,
        }
    }

    /// Set Kubernetes client
    pub fn with_k8s_client(mut self, client: K8sClient) -> Self {
        self.k8s_client = Some(client);
        self
    }

    /// Install Gateway API CRDs
    pub async fn install_gateway_api_crds(&self) -> Result<()> {
        info!("Installing Gateway API CRDs...");

        let output = Command::new("kubectl")
            .args([
                "apply",
                "-f",
                "https://github.com/kubernetes-sigs/gateway-api/releases/download/v1.0.0/standard-install.yaml",
            ])
            .output()
            .await
            .context("Failed to install Gateway API CRDs")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to install Gateway API CRDs: {stderr}");
        }

        info!("Gateway API CRDs installed successfully");
        Ok(())
    }

    /// Install experimental Gateway API CRDs (includes TCPRoute, etc.)
    pub async fn install_gateway_api_experimental(&self) -> Result<()> {
        info!("Installing experimental Gateway API CRDs...");

        let output = Command::new("kubectl")
            .args([
                "apply",
                "-f",
                "https://github.com/kubernetes-sigs/gateway-api/releases/download/v1.0.0/experimental-install.yaml",
            ])
            .output()
            .await
            .context("Failed to install experimental Gateway API CRDs")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to install experimental CRDs: {stderr}");
        }

        info!("Experimental Gateway API CRDs installed successfully");
        Ok(())
    }

    /// Install a gateway implementation
    pub async fn install(&self, gateway: GatewayImpl) -> Result<InstallResult> {
        info!("Installing {} gateway...", gateway.name());

        // Create namespace if needed
        self.ensure_namespace().await?;

        match gateway {
            GatewayImpl::Nginx => self.install_nginx().await,
            GatewayImpl::Envoy => self.install_envoy_gateway().await,
            GatewayImpl::Istio => self.install_istio().await,
            GatewayImpl::Cilium => self.install_cilium().await,
            GatewayImpl::Kong => self.install_kong().await,
            GatewayImpl::Traefik => self.install_traefik().await,
            GatewayImpl::Kgateway => self.install_kgateway().await,
        }
    }

    /// Uninstall a gateway implementation
    pub async fn uninstall(&self, gateway: GatewayImpl) -> Result<()> {
        info!("Uninstalling {} gateway...", gateway.name());

        let release_name = format!("{}-{}", self.config.release_prefix, gateway.short_name());

        match gateway {
            GatewayImpl::Istio => self.uninstall_istio().await,
            GatewayImpl::Cilium => self.uninstall_cilium().await,
            _ => self.helm_uninstall(&release_name).await,
        }
    }

    async fn ensure_namespace(&self) -> Result<()> {
        let output = Command::new("kubectl")
            .args(["create", "namespace", &self.config.namespace, "--dry-run=client", "-o", "yaml"])
            .output()
            .await?;

        if output.status.success() {
            let _ = Command::new("kubectl")
                .args(["apply", "-f", "-"])
                .stdin(std::process::Stdio::piped())
                .spawn();
        }

        // Apply namespace
        let output = Command::new("kubectl")
            .args(["create", "namespace", &self.config.namespace])
            .output()
            .await?;

        // Ignore "already exists" error
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("already exists") {
                debug!("Namespace creation note: {}", stderr);
            }
        }

        Ok(())
    }

    async fn install_nginx(&self) -> Result<InstallResult> {
        let release_name = format!("{}-nginx", self.config.release_prefix);

        // Add nginx repo
        self.helm_repo_add("nginx", "https://kubernetes.github.io/ingress-nginx").await?;

        // Install NGINX Gateway Fabric
        let mut args = vec![
            "upgrade".to_string(),
            "--install".to_string(),
            release_name.clone(),
            "oci://ghcr.io/nginxinc/charts/nginx-gateway-fabric".to_string(),
            "--namespace".to_string(),
            self.config.namespace.clone(),
            "--wait".to_string(),
            "--timeout".to_string(),
            format!("{}s", self.config.timeout_secs),
        ];

        // Add custom values
        for (key, value) in &self.config.helm_values {
            args.push("--set".to_string());
            args.push(format!("{key}={value}"));
        }

        self.helm_install(&args).await?;

        Ok(InstallResult {
            gateway: GatewayImpl::Nginx,
            release_name,
            namespace: self.config.namespace.clone(),
            gateway_class: "nginx".to_string(),
            status: InstallStatus::Installed,
        })
    }

    async fn install_envoy_gateway(&self) -> Result<InstallResult> {
        let release_name = format!("{}-envoy", self.config.release_prefix);

        // Install Envoy Gateway
        let args = vec![
            "upgrade".to_string(),
            "--install".to_string(),
            release_name.clone(),
            "oci://docker.io/envoyproxy/gateway-helm".to_string(),
            "--namespace".to_string(),
            self.config.namespace.clone(),
            "--create-namespace".to_string(),
            "--wait".to_string(),
            "--timeout".to_string(),
            format!("{}s", self.config.timeout_secs),
        ];

        self.helm_install(&args).await?;

        Ok(InstallResult {
            gateway: GatewayImpl::Envoy,
            release_name,
            namespace: self.config.namespace.clone(),
            gateway_class: "eg".to_string(),
            status: InstallStatus::Installed,
        })
    }

    async fn install_istio(&self) -> Result<InstallResult> {
        info!("Installing Istio with istioctl...");

        // Check if istioctl exists
        let check = Command::new("istioctl").arg("version").output().await;
        if check.is_err() {
            return Ok(InstallResult {
                gateway: GatewayImpl::Istio,
                release_name: "istio".to_string(),
                namespace: "istio-system".to_string(),
                gateway_class: "istio".to_string(),
                status: InstallStatus::Failed("istioctl not found".to_string()),
            });
        }

        // Install Istio with minimal profile
        let output = Command::new("istioctl")
            .args(["install", "--set", "profile=minimal", "-y"])
            .output()
            .await
            .context("Failed to run istioctl")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(InstallResult {
                gateway: GatewayImpl::Istio,
                release_name: "istio".to_string(),
                namespace: "istio-system".to_string(),
                gateway_class: "istio".to_string(),
                status: InstallStatus::Failed(stderr.to_string()),
            });
        }

        Ok(InstallResult {
            gateway: GatewayImpl::Istio,
            release_name: "istio".to_string(),
            namespace: "istio-system".to_string(),
            gateway_class: "istio".to_string(),
            status: InstallStatus::Installed,
        })
    }

    async fn install_cilium(&self) -> Result<InstallResult> {
        info!("Installing Cilium...");

        // Check if cilium CLI exists
        let check = Command::new("cilium").arg("version").output().await;
        if check.is_err() {
            // Fall back to Helm
            return self.install_cilium_helm().await;
        }

        let output = Command::new("cilium")
            .args([
                "install",
                "--set", "kubeProxyReplacement=true",
                "--set", "gatewayAPI.enabled=true",
            ])
            .output()
            .await
            .context("Failed to run cilium install")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Cilium CLI install failed: {}", stderr);
            return self.install_cilium_helm().await;
        }

        Ok(InstallResult {
            gateway: GatewayImpl::Cilium,
            release_name: "cilium".to_string(),
            namespace: "kube-system".to_string(),
            gateway_class: "cilium".to_string(),
            status: InstallStatus::Installed,
        })
    }

    async fn install_cilium_helm(&self) -> Result<InstallResult> {
        let release_name = format!("{}-cilium", self.config.release_prefix);

        self.helm_repo_add("cilium", "https://helm.cilium.io/").await?;

        let args = vec![
            "upgrade".to_string(),
            "--install".to_string(),
            release_name.clone(),
            "cilium/cilium".to_string(),
            "--namespace".to_string(),
            "kube-system".to_string(),
            "--set".to_string(),
            "kubeProxyReplacement=true".to_string(),
            "--set".to_string(),
            "gatewayAPI.enabled=true".to_string(),
            "--wait".to_string(),
            "--timeout".to_string(),
            format!("{}s", self.config.timeout_secs),
        ];

        self.helm_install(&args).await?;

        Ok(InstallResult {
            gateway: GatewayImpl::Cilium,
            release_name,
            namespace: "kube-system".to_string(),
            gateway_class: "cilium".to_string(),
            status: InstallStatus::Installed,
        })
    }

    async fn install_kong(&self) -> Result<InstallResult> {
        let release_name = format!("{}-kong", self.config.release_prefix);

        self.helm_repo_add("kong", "https://charts.konghq.com").await?;

        let args = vec![
            "upgrade".to_string(),
            "--install".to_string(),
            release_name.clone(),
            "kong/ingress".to_string(),
            "--namespace".to_string(),
            self.config.namespace.clone(),
            "--create-namespace".to_string(),
            "--set".to_string(),
            "gateway.enabled=true".to_string(),
            "--wait".to_string(),
            "--timeout".to_string(),
            format!("{}s", self.config.timeout_secs),
        ];

        self.helm_install(&args).await?;

        Ok(InstallResult {
            gateway: GatewayImpl::Kong,
            release_name,
            namespace: self.config.namespace.clone(),
            gateway_class: "kong".to_string(),
            status: InstallStatus::Installed,
        })
    }

    async fn install_traefik(&self) -> Result<InstallResult> {
        let release_name = format!("{}-traefik", self.config.release_prefix);

        self.helm_repo_add("traefik", "https://traefik.github.io/charts").await?;

        let args = vec![
            "upgrade".to_string(),
            "--install".to_string(),
            release_name.clone(),
            "traefik/traefik".to_string(),
            "--namespace".to_string(),
            self.config.namespace.clone(),
            "--create-namespace".to_string(),
            "--set".to_string(),
            "experimental.kubernetesGateway.enabled=true".to_string(),
            "--wait".to_string(),
            "--timeout".to_string(),
            format!("{}s", self.config.timeout_secs),
        ];

        self.helm_install(&args).await?;

        Ok(InstallResult {
            gateway: GatewayImpl::Traefik,
            release_name,
            namespace: self.config.namespace.clone(),
            gateway_class: "traefik".to_string(),
            status: InstallStatus::Installed,
        })
    }

    async fn install_kgateway(&self) -> Result<InstallResult> {
        let release_name = format!("{}-kgateway", self.config.release_prefix);

        self.helm_repo_add("kgateway", "https://kgateway-dev.github.io/kgateway/").await?;

        let args = vec![
            "upgrade".to_string(),
            "--install".to_string(),
            release_name.clone(),
            "kgateway/kgateway".to_string(),
            "--namespace".to_string(),
            self.config.namespace.clone(),
            "--create-namespace".to_string(),
            "--wait".to_string(),
            "--timeout".to_string(),
            format!("{}s", self.config.timeout_secs),
        ];

        self.helm_install(&args).await?;

        Ok(InstallResult {
            gateway: GatewayImpl::Kgateway,
            release_name,
            namespace: self.config.namespace.clone(),
            gateway_class: "kgateway".to_string(),
            status: InstallStatus::Installed,
        })
    }

    async fn helm_repo_add(&self, name: &str, url: &str) -> Result<()> {
        debug!("Adding Helm repo: {} -> {}", name, url);

        let output = Command::new("helm")
            .args(["repo", "add", name, url])
            .output()
            .await
            .context("Failed to add Helm repo")?;

        // Ignore "already exists" error
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("already exists") {
                warn!("Helm repo add warning: {}", stderr);
            }
        }

        // Update repo
        let _ = Command::new("helm")
            .args(["repo", "update", name])
            .output()
            .await;

        Ok(())
    }

    async fn helm_install(&self, args: &[String]) -> Result<()> {
        debug!("Running helm with args: {:?}", args);

        let output = Command::new("helm")
            .args(args)
            .output()
            .await
            .context("Failed to run helm")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Helm install failed: {stderr}");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("Helm output: {}", stdout);

        Ok(())
    }

    async fn helm_uninstall(&self, release_name: &str) -> Result<()> {
        info!("Uninstalling Helm release: {}", release_name);

        let output = Command::new("helm")
            .args([
                "uninstall",
                release_name,
                "--namespace",
                &self.config.namespace,
            ])
            .output()
            .await
            .context("Failed to run helm uninstall")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("not found") {
                anyhow::bail!("Helm uninstall failed: {stderr}");
            }
        }

        Ok(())
    }

    async fn uninstall_istio(&self) -> Result<()> {
        info!("Uninstalling Istio...");

        let output = Command::new("istioctl")
            .args(["uninstall", "--purge", "-y"])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                info!("Istio uninstalled successfully");
            }
            _ => {
                warn!("Istio uninstall may have failed or istioctl not available");
            }
        }

        Ok(())
    }

    async fn uninstall_cilium(&self) -> Result<()> {
        info!("Uninstalling Cilium...");

        // Try cilium CLI first
        let output = Command::new("cilium")
            .args(["uninstall"])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                info!("Cilium uninstalled successfully");
            }
            _ => {
                // Fall back to Helm
                let release_name = format!("{}-cilium", self.config.release_prefix);
                let _ = self.helm_uninstall(&release_name).await;
            }
        }

        Ok(())
    }

    /// List installed gateways
    pub async fn list_installed(&self) -> Result<Vec<InstallResult>> {
        let mut results = Vec::new();

        // Check each gateway implementation
        for gateway in GatewayImpl::all() {
            if let Ok(status) = self.check_installed(gateway).await {
                if status.is_installed() {
                    results.push(InstallResult {
                        gateway,
                        release_name: format!("{}-{}", self.config.release_prefix, gateway.short_name()),
                        namespace: self.config.namespace.clone(),
                        gateway_class: gateway.gateway_class().to_string(),
                        status,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Check if a gateway is installed
    pub async fn check_installed(&self, gateway: GatewayImpl) -> Result<InstallStatus> {
        let gateway_class = gateway.gateway_class();

        // Check if GatewayClass exists
        let output = Command::new("kubectl")
            .args(["get", "gatewayclass", gateway_class, "-o", "name"])
            .output()
            .await?;

        if output.status.success() {
            return Ok(InstallStatus::Installed);
        }

        // Check Helm release
        let release_name = format!("{}-{}", self.config.release_prefix, gateway.short_name());
        let output = Command::new("helm")
            .args(["status", &release_name, "-n", &self.config.namespace])
            .output()
            .await?;

        if output.status.success() {
            Ok(InstallStatus::Installed)
        } else {
            Ok(InstallStatus::NotInstalled)
        }
    }

    /// Wait for gateway to be ready
    pub async fn wait_ready(&self, gateway: GatewayImpl) -> Result<bool> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(self.config.timeout_secs);
        let gateway_class = gateway.gateway_class();

        info!("Waiting for {} to be ready...", gateway.name());

        loop {
            if start.elapsed() > timeout {
                warn!("Timeout waiting for {} to be ready", gateway.name());
                return Ok(false);
            }

            // Check GatewayClass status
            let output = Command::new("kubectl")
                .args([
                    "get", "gatewayclass", gateway_class,
                    "-o", "jsonpath={.status.conditions[?(@.type=='Accepted')].status}",
                ])
                .output()
                .await?;

            if output.status.success() {
                let status = String::from_utf8_lossy(&output.stdout);
                if status.trim() == "True" {
                    info!("{} is ready", gateway.name());
                    return Ok(true);
                }
            }

            sleep(Duration::from_secs(5)).await;
        }
    }
}

/// Installation result
#[derive(Clone, Debug)]
pub struct InstallResult {
    /// Gateway implementation
    pub gateway: GatewayImpl,

    /// Helm release name
    pub release_name: String,

    /// Namespace
    pub namespace: String,

    /// GatewayClass name
    pub gateway_class: String,

    /// Installation status
    pub status: InstallStatus,
}

/// Installation status
#[derive(Clone, Debug)]
pub enum InstallStatus {
    Installed,
    NotInstalled,
    Installing,
    Failed(String),
}

impl InstallStatus {
    pub fn is_installed(&self) -> bool {
        matches!(self, InstallStatus::Installed)
    }

    pub fn as_str(&self) -> &str {
        match self {
            InstallStatus::Installed => "Installed",
            InstallStatus::NotInstalled => "Not Installed",
            InstallStatus::Installing => "Installing",
            InstallStatus::Failed(_) => "Failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_installer_config() {
        let config = InstallerConfig::new()
            .namespace("test-ns")
            .timeout(600)
            .helm_value("key", "value");

        assert_eq!(config.namespace, "test-ns");
        assert_eq!(config.timeout_secs, 600);
        assert_eq!(config.helm_values.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_install_status() {
        assert!(InstallStatus::Installed.is_installed());
        assert!(!InstallStatus::NotInstalled.is_installed());
        assert_eq!(InstallStatus::Installing.as_str(), "Installing");
    }
}
