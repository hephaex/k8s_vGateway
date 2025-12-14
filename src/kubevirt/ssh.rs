//! SSH connectivity for KubeVirt VMs
//!
//! Provides SSH client for connecting to and executing commands in VMs.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// SSH client configuration
#[derive(Clone, Debug)]
pub struct SshConfig {
    /// SSH username
    pub username: String,

    /// SSH private key path
    pub private_key_path: Option<PathBuf>,

    /// SSH password (if not using key)
    pub password: Option<String>,

    /// SSH port
    pub port: u16,

    /// Connection timeout in seconds
    pub timeout_secs: u64,

    /// Strict host key checking
    pub strict_host_key_checking: bool,

    /// Number of connection retries
    pub retries: u32,

    /// Delay between retries in seconds
    pub retry_delay_secs: u64,
}

impl Default for SshConfig {
    fn default() -> Self {
        Self {
            username: "fedora".to_string(),
            private_key_path: None,
            password: None,
            port: 22,
            timeout_secs: 30,
            strict_host_key_checking: false,
            retries: 3,
            retry_delay_secs: 5,
        }
    }
}

impl SshConfig {
    /// Create a new SSH config
    pub fn new(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            ..Default::default()
        }
    }

    /// Set private key path
    pub fn private_key(mut self, path: impl Into<PathBuf>) -> Self {
        self.private_key_path = Some(path.into());
        self
    }

    /// Set password
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set port
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set connection timeout
    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Set retries
    pub fn retries(mut self, count: u32) -> Self {
        self.retries = count;
        self
    }
}

/// SSH client for connecting to VMs
pub struct SshClient {
    config: SshConfig,
}

impl SshClient {
    /// Create a new SSH client
    pub fn new(config: SshConfig) -> Self {
        Self { config }
    }

    /// Create with default config for a given username
    pub fn with_user(username: impl Into<String>) -> Self {
        Self {
            config: SshConfig::new(username),
        }
    }

    /// Build SSH command arguments
    fn build_ssh_args(&self, host: &str) -> Vec<String> {
        let mut args = vec![
            "-o".to_string(),
            format!(
                "StrictHostKeyChecking={}",
                if self.config.strict_host_key_checking {
                    "yes"
                } else {
                    "no"
                }
            ),
            "-o".to_string(),
            "UserKnownHostsFile=/dev/null".to_string(),
            "-o".to_string(),
            format!("ConnectTimeout={}", self.config.timeout_secs),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "LogLevel=ERROR".to_string(),
            "-p".to_string(),
            self.config.port.to_string(),
        ];

        if let Some(ref key_path) = self.config.private_key_path {
            args.push("-i".to_string());
            args.push(key_path.to_string_lossy().to_string());
        }

        args.push(format!("{}@{}", self.config.username, host));
        args
    }

    /// Test SSH connectivity
    pub async fn test_connection(&self, host: &str) -> Result<bool> {
        debug!(
            "Testing SSH connection to {}@{}",
            self.config.username, host
        );

        let mut args = self.build_ssh_args(host);
        args.push("echo".to_string());
        args.push("connected".to_string());

        let output = Command::new("ssh")
            .args(&args)
            .output()
            .await
            .context("Failed to execute SSH command")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.trim() == "connected")
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!("SSH connection test failed: {}", stderr);
            Ok(false)
        }
    }

    /// Wait for SSH to become available
    pub async fn wait_for_ssh(&self, host: &str, timeout_secs: u64) -> Result<bool> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        info!(
            "Waiting for SSH on {}:{} (timeout: {}s)",
            host, self.config.port, timeout_secs
        );

        loop {
            if start.elapsed() > timeout {
                warn!("Timeout waiting for SSH on {}", host);
                return Ok(false);
            }

            if self.test_connection(host).await.unwrap_or(false) {
                info!("SSH is available on {}", host);
                return Ok(true);
            }

            debug!(
                "SSH not yet available, retrying in {}s...",
                self.config.retry_delay_secs
            );
            sleep(Duration::from_secs(self.config.retry_delay_secs)).await;
        }
    }

    /// Execute a command over SSH
    pub async fn exec(&self, host: &str, command: &str) -> Result<SshOutput> {
        debug!("Executing SSH command on {}: {}", host, command);

        let mut args = self.build_ssh_args(host);
        args.push(command.to_string());

        let output = Command::new("ssh")
            .args(&args)
            .output()
            .await
            .context("Failed to execute SSH command")?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(SshOutput {
            stdout,
            stderr,
            exit_code,
        })
    }

    /// Execute a command with retries
    pub async fn exec_with_retry(&self, host: &str, command: &str) -> Result<SshOutput> {
        let mut last_error = None;

        for attempt in 1..=self.config.retries {
            debug!(
                "SSH exec attempt {}/{}: {}",
                attempt, self.config.retries, command
            );

            match self.exec(host, command).await {
                Ok(output) if output.exit_code == 0 => return Ok(output),
                Ok(output) => {
                    debug!("Command failed with exit code {}", output.exit_code);
                    last_error = Some(output);
                }
                Err(e) => {
                    debug!("SSH exec error: {}", e);
                }
            }

            if attempt < self.config.retries {
                sleep(Duration::from_secs(self.config.retry_delay_secs)).await;
            }
        }

        if let Some(output) = last_error {
            Ok(output)
        } else {
            anyhow::bail!("SSH command failed after {} retries", self.config.retries)
        }
    }

    /// Copy file to remote host via SCP
    pub async fn scp_to(&self, host: &str, local_path: &str, remote_path: &str) -> Result<()> {
        debug!(
            "SCP {} -> {}@{}:{}",
            local_path, self.config.username, host, remote_path
        );

        let mut args = vec![
            "-o".to_string(),
            format!(
                "StrictHostKeyChecking={}",
                if self.config.strict_host_key_checking {
                    "yes"
                } else {
                    "no"
                }
            ),
            "-o".to_string(),
            "UserKnownHostsFile=/dev/null".to_string(),
            "-P".to_string(),
            self.config.port.to_string(),
        ];

        if let Some(ref key_path) = self.config.private_key_path {
            args.push("-i".to_string());
            args.push(key_path.to_string_lossy().to_string());
        }

        args.push(local_path.to_string());
        args.push(format!("{}@{}:{}", self.config.username, host, remote_path));

        let output = Command::new("scp")
            .args(&args)
            .output()
            .await
            .context("Failed to execute SCP command")?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("SCP failed: {stderr}")
        }
    }

    /// Copy file from remote host via SCP
    pub async fn scp_from(&self, host: &str, remote_path: &str, local_path: &str) -> Result<()> {
        debug!(
            "SCP {}@{}:{} -> {}",
            self.config.username, host, remote_path, local_path
        );

        let mut args = vec![
            "-o".to_string(),
            format!(
                "StrictHostKeyChecking={}",
                if self.config.strict_host_key_checking {
                    "yes"
                } else {
                    "no"
                }
            ),
            "-o".to_string(),
            "UserKnownHostsFile=/dev/null".to_string(),
            "-P".to_string(),
            self.config.port.to_string(),
        ];

        if let Some(ref key_path) = self.config.private_key_path {
            args.push("-i".to_string());
            args.push(key_path.to_string_lossy().to_string());
        }

        args.push(format!("{}@{}:{}", self.config.username, host, remote_path));
        args.push(local_path.to_string());

        let output = Command::new("scp")
            .args(&args)
            .output()
            .await
            .context("Failed to execute SCP command")?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("SCP failed: {stderr}")
        }
    }

    /// Create an interactive SSH session
    pub fn connect(&self, host: &str) -> SshSession {
        SshSession {
            client: self.clone_config(),
            host: host.to_string(),
        }
    }

    fn clone_config(&self) -> SshClient {
        SshClient {
            config: self.config.clone(),
        }
    }
}

/// SSH command output
#[derive(Clone, Debug)]
pub struct SshOutput {
    /// Standard output
    pub stdout: String,

    /// Standard error
    pub stderr: String,

    /// Exit code
    pub exit_code: i32,
}

impl SshOutput {
    /// Check if command was successful
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get stdout lines
    pub fn lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }
}

/// SSH session for multiple commands
pub struct SshSession {
    client: SshClient,
    host: String,
}

impl SshSession {
    /// Execute a command in this session
    pub async fn exec(&self, command: &str) -> Result<SshOutput> {
        self.client.exec(&self.host, command).await
    }

    /// Execute with retries
    pub async fn exec_with_retry(&self, command: &str) -> Result<SshOutput> {
        self.client.exec_with_retry(&self.host, command).await
    }

    /// Copy file to remote
    pub async fn upload(&self, local: &str, remote: &str) -> Result<()> {
        self.client.scp_to(&self.host, local, remote).await
    }

    /// Copy file from remote
    pub async fn download(&self, remote: &str, local: &str) -> Result<()> {
        self.client.scp_from(&self.host, remote, local).await
    }

    /// Run curl command on remote host
    pub async fn curl(&self, url: &str) -> Result<SshOutput> {
        self.exec(&format!("curl -s -o /dev/null -w '%{{http_code}}' {url}"))
            .await
    }

    /// Run curl command with full output
    pub async fn curl_full(&self, url: &str) -> Result<SshOutput> {
        self.exec(&format!("curl -s -i {url}")).await
    }

    /// Check if a port is open
    pub async fn check_port(&self, host: &str, port: u16) -> Result<bool> {
        let output = self
            .exec(&format!(
                "timeout 5 bash -c 'cat < /dev/null > /dev/tcp/{host}/{port}'"
            ))
            .await?;
        Ok(output.is_success())
    }

    /// Get IP address of an interface
    pub async fn get_ip(&self, interface: &str) -> Result<Option<String>> {
        let output = self
            .exec(&format!(
                "ip -4 addr show {interface} | grep -oP '(?<=inet\\s)\\d+(\\.\\d+){{3}}'"
            ))
            .await?;

        if output.is_success() && !output.stdout.trim().is_empty() {
            Ok(Some(output.stdout.trim().to_string()))
        } else {
            Ok(None)
        }
    }

    /// Install a package (supports apt and dnf)
    pub async fn install_package(&self, package: &str) -> Result<SshOutput> {
        // Try dnf first (Fedora), then apt (Ubuntu/Debian)
        let output = self
            .exec(&format!(
                "which dnf && sudo dnf install -y {package} || sudo apt-get install -y {package}"
            ))
            .await?;
        Ok(output)
    }

    /// Run a test against gateway
    pub async fn test_gateway(
        &self,
        gateway_ip: &str,
        port: u16,
        path: &str,
        hostname: Option<&str>,
    ) -> Result<GatewayTestResult> {
        let url = format!("http://{gateway_ip}:{port}{path}");

        let curl_cmd = if let Some(host) = hostname {
            format!(
                "curl -s -o /dev/null -w '%{{http_code}}\\n%{{time_total}}' -H 'Host: {host}' {url}"
            )
        } else {
            format!("curl -s -o /dev/null -w '%{{http_code}}\\n%{{time_total}}' {url}")
        };

        let output = self.exec(&curl_cmd).await?;
        let lines: Vec<&str> = output.stdout.lines().collect();

        if lines.len() >= 2 {
            let status_code = lines[0].parse().unwrap_or(0);
            let duration_secs: f64 = lines[1].parse().unwrap_or(0.0);

            Ok(GatewayTestResult {
                success: (200..300).contains(&status_code),
                status_code,
                duration_ms: (duration_secs * 1000.0) as u64,
                error: None,
            })
        } else {
            Ok(GatewayTestResult {
                success: false,
                status_code: 0,
                duration_ms: 0,
                error: Some(output.stderr),
            })
        }
    }
}

/// Result of a gateway test from VM
#[derive(Clone, Debug)]
pub struct GatewayTestResult {
    /// Whether the test passed
    pub success: bool,

    /// HTTP status code
    pub status_code: u16,

    /// Duration in milliseconds
    pub duration_ms: u64,

    /// Error message if any
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_config_builder() {
        let config = SshConfig::new("testuser").port(2222).timeout(60).retries(5);

        assert_eq!(config.username, "testuser");
        assert_eq!(config.port, 2222);
        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.retries, 5);
    }

    #[test]
    fn test_ssh_client_args() {
        let client = SshClient::new(SshConfig::new("fedora").port(22));
        let args = client.build_ssh_args("192.168.1.100");

        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"22".to_string()));
        assert!(args.contains(&"fedora@192.168.1.100".to_string()));
    }

    #[test]
    fn test_ssh_output() {
        let output = SshOutput {
            stdout: "line1\nline2\nline3".to_string(),
            stderr: String::new(),
            exit_code: 0,
        };

        assert!(output.is_success());
        assert_eq!(output.lines().len(), 3);
    }
}
