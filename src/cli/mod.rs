//! CLI argument parsing
//!
//! Defines command-line interface using clap.

use clap::{Parser, Subcommand};

/// Kubernetes Gateway API Implementation Comparison Tool
#[derive(Parser, Debug)]
#[command(name = "gateway-poc")]
#[command(author = "hephaex@gmail.com")]
#[command(version = "0.1.0")]
#[command(about = "Test and compare 7 Gateway API implementations")]
#[command(long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run Gateway API tests
    Test(TestArgs),

    /// List available tests and gateways
    List(ListArgs),

    /// Manage KubeVirt VMs
    Vm(VmArgs),

    /// View test results
    Results(ResultsArgs),

    /// Deploy and manage gateway implementations
    Deploy(DeployArgs),
}

/// Arguments for test command
#[derive(Parser, Debug)]
pub struct TestArgs {
    /// Gateway implementation to test
    #[arg(short, long, default_value = "nginx")]
    pub gateway: String,

    /// Gateway IP address
    #[arg(short, long)]
    pub ip: Option<String>,

    /// Specific test number to run (1-17)
    #[arg(short, long)]
    pub test: Option<u8>,

    /// Run all tests
    #[arg(short, long)]
    pub all: bool,

    /// Number of test rounds
    #[arg(short, long, default_value = "1")]
    pub rounds: u32,

    /// Run tests in parallel
    #[arg(short, long)]
    pub parallel: bool,

    /// Number of concurrent tests (when parallel)
    #[arg(short, long, default_value = "4")]
    pub concurrent: usize,

    /// Output format (table, json, json-pretty, csv, summary)
    #[arg(short, long, default_value = "table")]
    pub format: String,

    /// Hostname for Host header
    #[arg(long, default_value = "example.com")]
    pub hostname: String,

    /// HTTP port
    #[arg(long, default_value = "80")]
    pub http_port: u16,

    /// HTTPS port
    #[arg(long, default_value = "443")]
    pub https_port: u16,

    /// gRPC port
    #[arg(long, default_value = "9090")]
    pub grpc_port: u16,

    /// Timeout in seconds
    #[arg(long, default_value = "30")]
    pub timeout: u64,

    /// Skip specific tests (comma-separated test numbers)
    #[arg(long)]
    pub skip: Option<String>,

    /// Save results to file
    #[arg(short, long)]
    pub output: Option<String>,
}

/// Arguments for list command
#[derive(Parser, Debug)]
pub struct ListArgs {
    /// Show detailed test information
    #[arg(short, long)]
    pub detailed: bool,

    /// Show gateway implementations
    #[arg(short, long)]
    pub gateways: bool,
}

/// Arguments for VM management
#[derive(Parser, Debug)]
pub struct VmArgs {
    #[command(subcommand)]
    pub action: VmAction,
}

#[derive(Subcommand, Debug)]
pub enum VmAction {
    /// Create KubeVirt VMs
    Create {
        /// Number of worker VMs
        #[arg(short, long, default_value = "1")]
        workers: u32,

        /// VM CPU cores
        #[arg(long, default_value = "4")]
        cpu: u32,

        /// VM memory in GB
        #[arg(long, default_value = "8")]
        memory: u32,

        /// VM disk size in GB
        #[arg(long, default_value = "50")]
        disk: u32,
    },

    /// Delete KubeVirt VMs
    Delete {
        /// Delete all VMs
        #[arg(short, long)]
        all: bool,

        /// Specific VM name to delete
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Show VM status
    Status,

    /// SSH into VM
    Ssh {
        /// VM name
        name: String,
    },
}

/// Arguments for results command
#[derive(Parser, Debug)]
pub struct ResultsArgs {
    /// Show summary only
    #[arg(short, long)]
    pub summary: bool,

    /// Filter by gateway
    #[arg(short, long)]
    pub gateway: Option<String>,

    /// Output format
    #[arg(short, long, default_value = "table")]
    pub format: String,

    /// Export to file
    #[arg(short, long)]
    pub export: Option<String>,
}

/// Arguments for deploy command
#[derive(Parser, Debug)]
pub struct DeployArgs {
    #[command(subcommand)]
    pub action: DeployAction,
}

#[derive(Subcommand, Debug)]
pub enum DeployAction {
    /// Install a gateway implementation
    Install {
        /// Gateway implementation to install
        gateway: String,

        /// Namespace for installation
        #[arg(short, long, default_value = "gateway-system")]
        namespace: String,

        /// Wait timeout in seconds
        #[arg(long, default_value = "300")]
        timeout: u64,
    },

    /// Uninstall a gateway implementation
    Uninstall {
        /// Gateway implementation to uninstall
        gateway: String,

        /// Namespace
        #[arg(short, long, default_value = "gateway-system")]
        namespace: String,
    },

    /// List installed gateways
    List,

    /// Check gateway health
    Health {
        /// Gateway implementation to check
        gateway: String,

        /// Gateway IP address
        #[arg(short, long)]
        ip: String,

        /// Gateway port
        #[arg(short, long, default_value = "80")]
        port: u16,
    },

    /// Run pre-flight checks
    Preflight {
        /// Gateway implementation
        gateway: String,

        /// Gateway IP address
        #[arg(short, long)]
        ip: String,

        /// Gateway port
        #[arg(short, long, default_value = "80")]
        port: u16,
    },

    /// Install Gateway API CRDs
    Crds {
        /// Install experimental CRDs
        #[arg(long)]
        experimental: bool,
    },

    /// Generate Kubernetes manifests
    Manifest {
        /// Gateway implementation
        #[arg(short, long, default_value = "nginx")]
        gateway: String,

        /// Resource type (gateway, httproute)
        #[arg(short, long, default_value = "gateway")]
        resource: String,

        /// Resource name
        #[arg(short, long, default_value = "test-gateway")]
        name: String,

        /// Output format (yaml, json)
        #[arg(short, long, default_value = "yaml")]
        format: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        let args = Args::parse_from(["gateway-poc", "list", "--detailed"]);
        match args.command {
            Command::List(list_args) => {
                assert!(list_args.detailed);
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_test_args() {
        let args = Args::parse_from([
            "gateway-poc",
            "test",
            "--gateway",
            "envoy",
            "--rounds",
            "10",
            "--parallel",
        ]);
        match args.command {
            Command::Test(test_args) => {
                assert_eq!(test_args.gateway, "envoy");
                assert_eq!(test_args.rounds, 10);
                assert!(test_args.parallel);
            }
            _ => panic!("Expected Test command"),
        }
    }
}
