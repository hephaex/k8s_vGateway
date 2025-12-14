//! Environment variable configuration
//!
//! Provides environment variable overrides for configuration.

use std::env;

/// Environment variable prefix
const ENV_PREFIX: &str = "GATEWAY_POC";

/// Environment configuration from environment variables
#[derive(Clone, Debug, Default)]
pub struct EnvConfig {
    /// Gateway IP from GATEWAY_POC_IP
    pub gateway_ip: Option<String>,
    /// Gateway from GATEWAY_POC_GATEWAY
    pub gateway: Option<String>,
    /// Hostname from GATEWAY_POC_HOSTNAME
    pub hostname: Option<String>,
    /// Port from GATEWAY_POC_PORT
    pub port: Option<u16>,
    /// Timeout from GATEWAY_POC_TIMEOUT
    pub timeout: Option<u64>,
    /// Rounds from GATEWAY_POC_ROUNDS
    pub rounds: Option<u32>,
    /// Parallel from GATEWAY_POC_PARALLEL
    pub parallel: Option<bool>,
    /// Config file from GATEWAY_POC_CONFIG
    pub config_file: Option<String>,
    /// Environment name from GATEWAY_POC_ENV
    pub environment: Option<String>,
    /// Verbose from GATEWAY_POC_VERBOSE
    pub verbose: Option<bool>,
    /// Output format from GATEWAY_POC_FORMAT
    pub format: Option<String>,
    /// Namespace from GATEWAY_POC_NAMESPACE
    pub namespace: Option<String>,
    /// Kubeconfig from KUBECONFIG
    pub kubeconfig: Option<String>,
}

impl EnvConfig {
    /// Load configuration from environment variables
    pub fn load() -> Self {
        Self {
            gateway_ip: get_env("IP"),
            gateway: get_env("GATEWAY"),
            hostname: get_env("HOSTNAME"),
            port: get_env_parse("PORT"),
            timeout: get_env_parse("TIMEOUT"),
            rounds: get_env_parse("ROUNDS"),
            parallel: get_env_bool("PARALLEL"),
            config_file: get_env("CONFIG"),
            environment: get_env("ENV"),
            verbose: get_env_bool("VERBOSE"),
            format: get_env("FORMAT"),
            namespace: get_env("NAMESPACE"),
            kubeconfig: env::var("KUBECONFIG").ok(),
        }
    }

    /// Check if any environment variables are set
    pub fn has_any(&self) -> bool {
        self.gateway_ip.is_some()
            || self.gateway.is_some()
            || self.hostname.is_some()
            || self.port.is_some()
            || self.timeout.is_some()
            || self.rounds.is_some()
            || self.parallel.is_some()
            || self.config_file.is_some()
            || self.environment.is_some()
            || self.verbose.is_some()
            || self.format.is_some()
            || self.namespace.is_some()
    }

    /// Get gateway IP with fallback
    pub fn gateway_ip_or(&self, default: &str) -> String {
        self.gateway_ip.clone().unwrap_or_else(|| default.to_string())
    }

    /// Get gateway with fallback
    pub fn gateway_or(&self, default: &str) -> String {
        self.gateway.clone().unwrap_or_else(|| default.to_string())
    }

    /// Get hostname with fallback
    pub fn hostname_or(&self, default: &str) -> String {
        self.hostname.clone().unwrap_or_else(|| default.to_string())
    }

    /// Get timeout with fallback
    pub fn timeout_or(&self, default: u64) -> u64 {
        self.timeout.unwrap_or(default)
    }

    /// Get rounds with fallback
    pub fn rounds_or(&self, default: u32) -> u32 {
        self.rounds.unwrap_or(default)
    }

    /// Print current environment configuration
    pub fn print_summary(&self) {
        println!("Environment Configuration:");
        println!("  {}_IP:        {:?}", ENV_PREFIX, self.gateway_ip);
        println!("  {}_GATEWAY:   {:?}", ENV_PREFIX, self.gateway);
        println!("  {}_HOSTNAME:  {:?}", ENV_PREFIX, self.hostname);
        println!("  {}_PORT:      {:?}", ENV_PREFIX, self.port);
        println!("  {}_TIMEOUT:   {:?}", ENV_PREFIX, self.timeout);
        println!("  {}_ROUNDS:    {:?}", ENV_PREFIX, self.rounds);
        println!("  {}_PARALLEL:  {:?}", ENV_PREFIX, self.parallel);
        println!("  {}_CONFIG:    {:?}", ENV_PREFIX, self.config_file);
        println!("  {}_ENV:       {:?}", ENV_PREFIX, self.environment);
        println!("  KUBECONFIG:          {:?}", self.kubeconfig);
    }
}

/// Get environment variable with prefix
fn get_env(name: &str) -> Option<String> {
    env::var(format!("{ENV_PREFIX}_{name}")).ok()
}

/// Get environment variable and parse to type
fn get_env_parse<T: std::str::FromStr>(name: &str) -> Option<T> {
    get_env(name).and_then(|v| v.parse().ok())
}

/// Get environment variable as boolean
fn get_env_bool(name: &str) -> Option<bool> {
    get_env(name).map(|v| {
        matches!(
            v.to_lowercase().as_str(),
            "1" | "true" | "yes" | "on" | "enabled"
        )
    })
}

/// Builder for setting environment variables (useful for testing)
pub struct EnvBuilder {
    vars: Vec<(String, String)>,
}

impl EnvBuilder {
    /// Create a new environment builder
    pub fn new() -> Self {
        Self { vars: Vec::new() }
    }

    /// Set gateway IP
    pub fn gateway_ip(mut self, ip: impl Into<String>) -> Self {
        self.vars.push((format!("{ENV_PREFIX}_IP"), ip.into()));
        self
    }

    /// Set gateway
    pub fn gateway(mut self, gateway: impl Into<String>) -> Self {
        self.vars.push((format!("{ENV_PREFIX}_GATEWAY"), gateway.into()));
        self
    }

    /// Set hostname
    pub fn hostname(mut self, hostname: impl Into<String>) -> Self {
        self.vars.push((format!("{ENV_PREFIX}_HOSTNAME"), hostname.into()));
        self
    }

    /// Set port
    pub fn port(mut self, port: u16) -> Self {
        self.vars.push((format!("{ENV_PREFIX}_PORT"), port.to_string()));
        self
    }

    /// Set timeout
    pub fn timeout(mut self, timeout: u64) -> Self {
        self.vars.push((format!("{ENV_PREFIX}_TIMEOUT"), timeout.to_string()));
        self
    }

    /// Set rounds
    pub fn rounds(mut self, rounds: u32) -> Self {
        self.vars.push((format!("{ENV_PREFIX}_ROUNDS"), rounds.to_string()));
        self
    }

    /// Set parallel
    pub fn parallel(mut self, parallel: bool) -> Self {
        self.vars.push((format!("{ENV_PREFIX}_PARALLEL"), parallel.to_string()));
        self
    }

    /// Set environment name
    pub fn environment(mut self, env: impl Into<String>) -> Self {
        self.vars.push((format!("{ENV_PREFIX}_ENV"), env.into()));
        self
    }

    /// Apply environment variables
    pub fn apply(self) {
        for (key, value) in self.vars {
            env::set_var(key, value);
        }
    }

    /// Apply and return guard that restores on drop
    pub fn apply_scoped(self) -> EnvGuard {
        let previous: Vec<_> = self
            .vars
            .iter()
            .map(|(k, _)| (k.clone(), env::var(k).ok()))
            .collect();

        self.apply();

        EnvGuard { previous }
    }
}

impl Default for EnvBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Guard that restores environment variables on drop
pub struct EnvGuard {
    previous: Vec<(String, Option<String>)>,
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            match value {
                Some(v) => env::set_var(key, v),
                None => env::remove_var(key),
            }
        }
    }
}

/// Print all GATEWAY_POC environment variables
pub fn print_env_help() {
    println!("Environment Variables:");
    println!();
    println!("  {ENV_PREFIX}_IP          Gateway IP address");
    println!("  {ENV_PREFIX}_GATEWAY     Gateway implementation (nginx, envoy, istio, etc.)");
    println!("  {ENV_PREFIX}_HOSTNAME    Hostname for Host header");
    println!("  {ENV_PREFIX}_PORT        Gateway port");
    println!("  {ENV_PREFIX}_TIMEOUT     Request timeout in seconds");
    println!("  {ENV_PREFIX}_ROUNDS      Number of test rounds");
    println!("  {ENV_PREFIX}_PARALLEL    Enable parallel execution (true/false)");
    println!("  {ENV_PREFIX}_CONFIG      Path to configuration file");
    println!("  {ENV_PREFIX}_ENV         Environment name (dev, staging, prod)");
    println!("  {ENV_PREFIX}_VERBOSE     Enable verbose output (true/false)");
    println!("  {ENV_PREFIX}_FORMAT      Output format (table, json, csv)");
    println!("  {ENV_PREFIX}_NAMESPACE   Kubernetes namespace");
    println!("  KUBECONFIG            Path to kubeconfig file");
    println!();
    println!("Example:");
    println!("  export {ENV_PREFIX}_IP=10.0.0.100");
    println!("  export {ENV_PREFIX}_GATEWAY=nginx");
    println!("  gateway-poc test --all");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_config_default() {
        let config = EnvConfig::default();
        assert!(config.gateway_ip.is_none());
        assert!(config.gateway.is_none());
    }

    #[test]
    fn test_env_config_fallback() {
        let config = EnvConfig::default();
        assert_eq!(config.gateway_ip_or("127.0.0.1"), "127.0.0.1");
        assert_eq!(config.gateway_or("nginx"), "nginx");
        assert_eq!(config.timeout_or(30), 30);
    }

    #[test]
    fn test_env_builder() {
        let _guard = EnvBuilder::new()
            .gateway_ip("10.0.0.1")
            .gateway("envoy")
            .timeout(60)
            .apply_scoped();

        let config = EnvConfig::load();
        assert_eq!(config.gateway_ip, Some("10.0.0.1".to_string()));
        assert_eq!(config.gateway, Some("envoy".to_string()));
        assert_eq!(config.timeout, Some(60));
    }

    #[test]
    fn test_env_bool_parsing() {
        let _guard = EnvBuilder::new()
            .parallel(true)
            .apply_scoped();

        let config = EnvConfig::load();
        assert_eq!(config.parallel, Some(true));
    }

    #[test]
    fn test_has_any() {
        let empty = EnvConfig::default();
        assert!(!empty.has_any());

        let with_ip = EnvConfig {
            gateway_ip: Some("10.0.0.1".to_string()),
            ..Default::default()
        };
        assert!(with_ip.has_any());
    }
}
