//! Configuration module
//!
//! Handles loading and managing configuration.

#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Application configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    /// Default gateway implementation
    pub default_gateway: String,

    /// Default number of test rounds
    pub default_rounds: u32,

    /// HTTP timeout in seconds
    pub timeout_secs: u64,

    /// Enable parallel execution by default
    pub parallel: bool,

    /// Maximum concurrent tests
    pub max_concurrent: usize,

    /// KubeVirt configuration
    pub kubevirt: KubeVirtConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_gateway: "nginx".to_string(),
            default_rounds: 1,
            timeout_secs: 30,
            parallel: false,
            max_concurrent: 4,
            kubevirt: KubeVirtConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load configuration from file
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content =
            std::fs::read_to_string(path.as_ref()).context("Failed to read config file")?;

        let config: Self = if path
            .as_ref()
            .extension()
            .map(|e| e == "yaml" || e == "yml")
            .unwrap_or(false)
        {
            serde_yaml::from_str(&content).context("Failed to parse YAML config")?
        } else {
            serde_json::from_str(&content).context("Failed to parse JSON config")?
        };

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = if path
            .as_ref()
            .extension()
            .map(|e| e == "yaml" || e == "yml")
            .unwrap_or(false)
        {
            serde_yaml::to_string(self).context("Failed to serialize config")?
        } else {
            serde_json::to_string_pretty(self).context("Failed to serialize config")?
        };

        std::fs::write(path, content).context("Failed to write config file")?;
        Ok(())
    }
}

/// KubeVirt VM configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KubeVirtConfig {
    /// Namespace for VMs
    pub namespace: String,

    /// Default VM CPU cores
    pub default_cpu: u32,

    /// Default VM memory in GB
    pub default_memory: u32,

    /// Default VM disk size in GB
    pub default_disk: u32,

    /// VM image URL
    pub image_url: String,

    /// SSH key path
    pub ssh_key_path: Option<String>,
}

impl Default for KubeVirtConfig {
    fn default() -> Self {
        Self {
            namespace: "kubevirt-vms".to_string(),
            default_cpu: 4,
            default_memory: 8,
            default_disk: 50,
            image_url: "docker.io/kubevirt/fedora-cloud-container-disk-demo:latest".to_string(),
            ssh_key_path: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.default_gateway, "nginx");
        assert_eq!(config.timeout_secs, 30);
    }

    #[test]
    fn test_kubevirt_config() {
        let config = KubeVirtConfig::default();
        assert_eq!(config.default_cpu, 4);
        assert_eq!(config.default_memory, 8);
    }
}
