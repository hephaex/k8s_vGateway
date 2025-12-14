//! Configuration file management
//!
//! Handles finding, loading, and validating configuration files.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::profile::{GatewayProfile, TestProfile};
use super::{AppConfig, KubeVirtConfig};

/// Configuration file locations (in order of precedence)
const CONFIG_LOCATIONS: &[&str] = &[
    "./gateway-poc.yaml",
    "./gateway-poc.yml",
    "./.gateway-poc.yaml",
    "./.gateway-poc/config.yaml",
    "~/.config/gateway-poc/config.yaml",
    "~/.gateway-poc.yaml",
];

/// Full configuration file structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    /// Version of config file format
    #[serde(default = "default_version")]
    pub version: String,

    /// Application settings
    #[serde(default)]
    pub app: AppConfig,

    /// Gateway profiles
    #[serde(default)]
    pub gateway_profiles: Vec<GatewayProfile>,

    /// Test profiles
    #[serde(default)]
    pub test_profiles: Vec<TestProfile>,

    /// Environment-specific overrides
    #[serde(default)]
    pub environments: Vec<EnvironmentConfig>,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            version: default_version(),
            app: AppConfig::default(),
            gateway_profiles: Vec::new(),
            test_profiles: Vec::new(),
            environments: Vec::new(),
        }
    }
}

impl ConfigFile {
    /// Create a new config file with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Find configuration file in standard locations
    pub fn find() -> Option<PathBuf> {
        for location in CONFIG_LOCATIONS {
            let path = expand_path(location);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    /// Load configuration from default location
    pub fn load_default() -> Result<Self> {
        if let Some(path) = Self::find() {
            Self::load(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load configuration from file
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Self = if is_yaml_file(path) {
            serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse YAML config: {}", path.display()))?
        } else {
            serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse JSON config: {}", path.display()))?
        };

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let content = if is_yaml_file(path) {
            serde_yaml::to_string(self).context("Failed to serialize config")?
        } else {
            serde_json::to_string_pretty(self).context("Failed to serialize config")?
        };

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate version
        if !["1.0", "1.1"].contains(&self.version.as_str()) {
            anyhow::bail!("Unsupported config version: {}", self.version);
        }

        // Validate test profiles
        for profile in &self.test_profiles {
            for test_num in &profile.tests {
                if *test_num < 1 || *test_num > 17 {
                    anyhow::bail!(
                        "Invalid test number {} in profile '{}'. Valid range: 1-17",
                        test_num,
                        profile.name
                    );
                }
            }
        }

        Ok(())
    }

    /// Generate example configuration
    pub fn example() -> Self {
        use crate::models::GatewayImpl;

        Self {
            version: "1.0".to_string(),
            app: AppConfig {
                default_gateway: "nginx".to_string(),
                default_rounds: 3,
                timeout_secs: 30,
                parallel: true,
                max_concurrent: 4,
                kubevirt: KubeVirtConfig::default(),
            },
            gateway_profiles: vec![
                GatewayProfile::default_for(GatewayImpl::Nginx),
                GatewayProfile::default_for(GatewayImpl::Envoy),
            ],
            test_profiles: vec![
                TestProfile::smoke(),
                TestProfile::routing(),
                TestProfile::all(),
            ],
            environments: vec![
                EnvironmentConfig {
                    name: "development".to_string(),
                    gateway_ip: "127.0.0.1".to_string(),
                    hostname: "dev.example.com".to_string(),
                    tls_enabled: false,
                    extra: std::collections::HashMap::new(),
                },
                EnvironmentConfig {
                    name: "staging".to_string(),
                    gateway_ip: "10.0.0.100".to_string(),
                    hostname: "staging.example.com".to_string(),
                    tls_enabled: true,
                    extra: std::collections::HashMap::new(),
                },
            ],
        }
    }

    /// Get environment by name
    pub fn environment(&self, name: &str) -> Option<&EnvironmentConfig> {
        self.environments.iter().find(|e| e.name == name)
    }

    /// Get gateway profile by name
    pub fn gateway_profile(&self, name: &str) -> Option<&GatewayProfile> {
        self.gateway_profiles.iter().find(|p| p.name == name)
    }

    /// Get test profile by name
    pub fn test_profile(&self, name: &str) -> Option<&TestProfile> {
        self.test_profiles.iter().find(|p| p.name == name)
    }

    /// Merge with another config (other takes precedence)
    pub fn merge(&mut self, other: ConfigFile) {
        // Merge app config
        if other.app.default_gateway != "nginx" {
            self.app.default_gateway = other.app.default_gateway;
        }
        if other.app.default_rounds != 1 {
            self.app.default_rounds = other.app.default_rounds;
        }
        if other.app.timeout_secs != 30 {
            self.app.timeout_secs = other.app.timeout_secs;
        }
        if other.app.parallel {
            self.app.parallel = true;
        }
        if other.app.max_concurrent != 4 {
            self.app.max_concurrent = other.app.max_concurrent;
        }

        // Add profiles from other
        for profile in other.gateway_profiles {
            if !self.gateway_profiles.iter().any(|p| p.name == profile.name) {
                self.gateway_profiles.push(profile);
            }
        }
        for profile in other.test_profiles {
            if !self.test_profiles.iter().any(|p| p.name == profile.name) {
                self.test_profiles.push(profile);
            }
        }
        for env in other.environments {
            if !self.environments.iter().any(|e| e.name == env.name) {
                self.environments.push(env);
            }
        }
    }
}

/// Environment-specific configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Environment name (e.g., "dev", "staging", "prod")
    pub name: String,
    /// Gateway IP address
    pub gateway_ip: String,
    /// Hostname for testing
    pub hostname: String,
    /// TLS enabled
    #[serde(default)]
    pub tls_enabled: bool,
    /// Extra environment-specific settings
    #[serde(default)]
    pub extra: std::collections::HashMap<String, String>,
}

impl EnvironmentConfig {
    /// Create new environment config
    pub fn new(name: impl Into<String>, gateway_ip: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            gateway_ip: gateway_ip.into(),
            hostname: "example.com".to_string(),
            tls_enabled: false,
            extra: std::collections::HashMap::new(),
        }
    }

    /// Set hostname
    pub fn with_hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = hostname.into();
        self
    }

    /// Enable TLS
    pub fn with_tls(mut self) -> Self {
        self.tls_enabled = true;
        self
    }
}

/// Expand ~ to home directory
fn expand_path(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(path)
}

/// Check if file is YAML based on extension
fn is_yaml_file(path: &Path) -> bool {
    path.extension()
        .map(|e| e == "yaml" || e == "yml")
        .unwrap_or(false)
}

/// Config file watcher for hot-reloading (optional)
pub struct ConfigWatcher {
    path: PathBuf,
    last_modified: Option<std::time::SystemTime>,
}

impl ConfigWatcher {
    /// Create a new config watcher
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            last_modified: None,
        }
    }

    /// Check if config file has changed
    pub fn has_changed(&mut self) -> bool {
        if let Ok(metadata) = std::fs::metadata(&self.path) {
            if let Ok(modified) = metadata.modified() {
                let changed = self.last_modified.map(|lm| modified > lm).unwrap_or(true);
                self.last_modified = Some(modified);
                return changed;
            }
        }
        false
    }

    /// Reload config if changed
    pub fn reload_if_changed(&mut self) -> Result<Option<ConfigFile>> {
        if self.has_changed() {
            Ok(Some(ConfigFile::load(&self.path)?))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_file_default() {
        let config = ConfigFile::default();
        assert_eq!(config.version, "1.0");
    }

    #[test]
    fn test_config_file_example() {
        let config = ConfigFile::example();
        assert!(!config.gateway_profiles.is_empty());
        assert!(!config.test_profiles.is_empty());
        assert!(!config.environments.is_empty());
    }

    #[test]
    fn test_config_file_save_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.yaml");

        let config = ConfigFile::example();
        config.save(&path).unwrap();

        let loaded = ConfigFile::load(&path).unwrap();
        assert_eq!(loaded.version, config.version);
        assert_eq!(loaded.app.default_gateway, config.app.default_gateway);
    }

    #[test]
    fn test_environment_config() {
        let env = EnvironmentConfig::new("test", "10.0.0.1")
            .with_hostname("test.example.com")
            .with_tls();

        assert_eq!(env.name, "test");
        assert_eq!(env.gateway_ip, "10.0.0.1");
        assert!(env.tls_enabled);
    }

    #[test]
    fn test_validate_config() {
        let mut config = ConfigFile::default();
        config.test_profiles.push(TestProfile {
            name: "invalid".to_string(),
            description: String::new(),
            tests: vec![99], // Invalid test number
            rounds: 1,
            parallel: false,
            timeout_secs: 30,
            tags: Vec::new(),
        });

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_expand_path() {
        let path = expand_path("./test.yaml");
        assert_eq!(path, PathBuf::from("./test.yaml"));
    }
}
