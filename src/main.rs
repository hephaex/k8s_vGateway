//! Gateway API PoC - Kubernetes Gateway Implementation Comparison Tool
//!
//! A CLI tool for testing and comparing 7 Gateway API implementations
//! with KubeVirt virtualization support for AMD64 components on ARM64 hosts.
//!
//! ## Features
//!
//! - 17 comprehensive test cases covering routing, TLS, traffic management
//! - Support for 7 Gateway implementations (NGINX, Envoy, Istio, Cilium, Kong, Traefik, kgateway)
//! - Parallel test execution
//! - Multiple output formats (Table, JSON, CSV)
//! - KubeVirt VM management for AMD64 testing
//!
//! ## Usage
//!
//! ```bash
//! # Run all tests for a gateway
//! gateway-poc test --gateway nginx --ip 10.0.0.1
//!
//! # Run specific test
//! gateway-poc test --gateway envoy --test 1
//!
//! # Run multiple rounds
//! gateway-poc test --gateway istio --rounds 100
//!
//! # List available tests
//! gateway-poc list --detailed
//!
//! # Manage KubeVirt VMs
//! gateway-poc vm create --workers 2
//! gateway-poc vm status
//! ```

use anyhow::Result;
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod cli;
mod config;
mod executor;
mod http;
mod k8s;
mod kubevirt;
mod models;
mod output;
mod results;
mod tests;
mod utils;

use cli::Args;
use executor::{BatchRunner, ParallelExecutor, TestRunner};
use models::{GatewayConfig, GatewayImpl, TestCase, TestConfig};
use output::{OutputFormat, ResultFormatter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .compact()
        .init();

    let args = Args::parse();

    match args.command {
        cli::Command::Test(test_args) => {
            run_tests(test_args).await?;
        }
        cli::Command::List(list_args) => {
            list_tests(list_args);
        }
        cli::Command::Vm(vm_args) => {
            manage_vm(vm_args).await?;
        }
        cli::Command::Results(results_args) => {
            show_results(results_args)?;
        }
    }

    Ok(())
}

async fn run_tests(args: cli::TestArgs) -> Result<()> {
    let implementation = GatewayImpl::from_str(&args.gateway)
        .ok_or_else(|| anyhow::anyhow!("Unknown gateway: {}", args.gateway))?;

    let gateway_config = GatewayConfig::new(implementation).with_hostname(&args.hostname);

    let config = TestConfig::new(gateway_config).with_rounds(args.rounds);

    let gateway_ip = args.ip.as_deref().unwrap_or("127.0.0.1");

    info!(
        "Testing {} Gateway at {} ({} rounds)",
        implementation, gateway_ip, args.rounds
    );

    let formatter =
        ResultFormatter::new(OutputFormat::from_str(&args.format).unwrap_or(OutputFormat::Table));

    if args.parallel {
        let executor = ParallelExecutor::new(args.concurrent);

        if args.rounds > 1 {
            let batch_runner = BatchRunner::new(args.concurrent, args.rounds);
            let summaries = batch_runner.run_rounds(gateway_ip, &config.gateway).await?;

            for summary in &summaries {
                println!("{}", formatter.format_summary(summary));
            }

            let aggregate = BatchRunner::aggregate_results(&summaries);
            println!(
                "{}",
                formatter.format_aggregate(&aggregate, implementation.name())
            );
        } else {
            let summary = executor
                .run_all_parallel(gateway_ip, &config.gateway)
                .await?;
            println!("{}", formatter.format_summary(&summary));
        }
    } else {
        let runner = TestRunner::new(config)?.with_gateway_ip(gateway_ip);

        if let Some(test_num) = args.test {
            let test_case = TestCase::from_number(test_num)
                .ok_or_else(|| anyhow::anyhow!("Invalid test number: {test_num}"))?;
            let result = runner.run_test(test_case).await;
            println!("{}", formatter.format_result(&result));
        } else if args.rounds > 1 {
            let summaries = runner.run_rounds(args.rounds).await?;
            for summary in summaries {
                println!("{}", formatter.format_summary(&summary));
            }
        } else {
            let summary = runner.run_all().await?;
            println!("{}", formatter.format_summary(&summary));
        }
    }

    Ok(())
}

fn list_tests(args: cli::ListArgs) {
    println!("\nGateway API Test Cases (17 total)\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut current_category = "";

    for test_case in TestCase::all() {
        let category = test_case.category();
        if category != current_category {
            if !current_category.is_empty() {
                println!();
            }
            println!("\n{category} Tests:");
            println!("──────────────────────────────────────────────────────────────────────");
            current_category = category;
        }

        if args.detailed {
            println!(
                "  {:2}. {:20} [{}]",
                test_case.number(),
                test_case.name(),
                test_case.category()
            );
        } else {
            println!("  {:2}. {}", test_case.number(), test_case.name());
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    if args.gateways {
        println!("Supported Gateway Implementations:\n");
        for gateway in GatewayImpl::all() {
            let arm64_status = if gateway.supports_arm64() {
                "ARM64 ✓"
            } else {
                "AMD64 only (requires KubeVirt)"
            };
            println!("  - {:25} [{}]", gateway.name(), arm64_status);
        }
        println!();
    }
}

async fn manage_vm(args: cli::VmArgs) -> Result<()> {
    use kubevirt::{SshClient, SshConfig, VirtualMachineManager, VmConfig, VmiManager};

    let k8s_client = k8s::K8sClient::new("default").await?;
    let vm_manager = VirtualMachineManager::new(k8s_client.clone());
    let vmi_manager = VmiManager::new(k8s_client.clone());

    // Check if KubeVirt is installed
    if !vm_manager.is_kubevirt_installed().await? {
        println!("❌ KubeVirt is not installed in the cluster.");
        println!(
            "   Install KubeVirt first: https://kubevirt.io/user-guide/operations/installation/"
        );
        return Ok(());
    }

    match args.action {
        cli::VmAction::Create {
            workers,
            cpu,
            memory,
            disk: _,
        } => {
            info!("Creating {} KubeVirt VM(s)...", workers);

            for i in 0..workers {
                let vm_name = format!("gateway-test-vm-{i}");
                println!("Creating VM: {vm_name}");

                let vm = VmConfig::new(&vm_name, "default")
                    .cpu(cpu)
                    .memory(format!("{memory}Gi"))
                    .label("app", "gateway-test")
                    .label("instance", i.to_string())
                    .build();

                match vm_manager.create(&vm, "default").await {
                    Ok(_) => {
                        println!("  ✓ VM {vm_name} created successfully");

                        // Wait for VM to be ready
                        println!("  ⏳ Waiting for VM to be ready...");
                        if vm_manager.wait_ready(&vm_name, "default", 300).await? {
                            println!("  ✓ VM {vm_name} is ready");

                            // Wait for IP
                            if let Some(ip) =
                                vmi_manager.wait_for_ip(&vm_name, "default", 120).await?
                            {
                                println!("  ✓ VM {vm_name} has IP: {ip}");
                            }
                        } else {
                            println!("  ⚠ VM {vm_name} did not become ready in time");
                        }
                    }
                    Err(e) => {
                        println!("  ✗ Failed to create VM {vm_name}: {e}");
                    }
                }
            }
        }

        cli::VmAction::Delete { all, name } => {
            if all {
                info!("Deleting all gateway-test VMs...");
                let vms = vm_manager.list("default").await?;

                for vm in vms {
                    if let Some(labels) = &vm.metadata.labels {
                        if labels.get("app").map(|s| s.as_str()) == Some("gateway-test") {
                            if let Some(vm_name) = &vm.metadata.name {
                                match vm_manager.delete(vm_name, "default").await {
                                    Ok(_) => println!("  ✓ Deleted VM: {vm_name}"),
                                    Err(e) => println!("  ✗ Failed to delete {vm_name}: {e}"),
                                }
                            }
                        }
                    }
                }
            } else if let Some(vm_name) = name {
                info!("Deleting VM: {}", vm_name);
                match vm_manager.delete(&vm_name, "default").await {
                    Ok(_) => println!("✓ Deleted VM: {vm_name}"),
                    Err(e) => println!("✗ Failed to delete {vm_name}: {e}"),
                }
            } else {
                println!("Please specify --all or --name <vm_name>");
            }
        }

        cli::VmAction::Status => {
            info!("Fetching VM status...");
            let vms = vm_manager.list("default").await?;

            println!("\n┌─────────────────────────────────────────────────────────────┐");
            println!("│ KubeVirt VMs in 'default' namespace                          │");
            println!("├─────────────────────────┬──────────┬─────────────────────────┤");
            println!("│ Name                    │ Status   │ IP Address              │");
            println!("├─────────────────────────┼──────────┼─────────────────────────┤");

            for vm in vms {
                let name = vm.metadata.name.as_deref().unwrap_or("unknown");
                let status = vm
                    .status
                    .as_ref()
                    .and_then(|s| s.printable_status.clone())
                    .unwrap_or_else(|| "Unknown".to_string());

                // Try to get IP from VMI
                let ip = match vmi_manager.get_ip(name, "default").await {
                    Ok(Some(ip)) => ip,
                    _ => "N/A".to_string(),
                };

                println!("│ {name:23} │ {status:8} │ {ip:23} │");
            }

            println!("└─────────────────────────┴──────────┴─────────────────────────┘\n");
        }

        cli::VmAction::Ssh { name } => {
            info!("Connecting to VM via SSH: {}", name);

            // Get VM IP
            let ip = match vmi_manager.get_ip(&name, "default").await? {
                Some(ip) => ip,
                None => {
                    println!("❌ Could not find IP address for VM: {name}");
                    return Ok(());
                }
            };

            println!("Connecting to {name} ({ip})...");

            let ssh = SshClient::new(SshConfig::new("fedora").port(22));

            // Test connection
            if ssh.wait_for_ssh(&ip, 60).await? {
                println!("SSH is available. Use the following command to connect:");
                println!("\n  ssh fedora@{ip}\n");

                // Or use virtctl:
                println!("Alternatively, use virtctl:");
                println!("\n  virtctl ssh --namespace default {name}\n");
            } else {
                println!("❌ Could not establish SSH connection to VM");
            }
        }
    }

    Ok(())
}

fn show_results(args: cli::ResultsArgs) -> Result<()> {
    use results::{
        ComparisonFormatter, GatewayComparator, ReportFormat, ReportGenerator, ResultsStorage,
    };
    use std::path::PathBuf;

    info!("Results viewer - displaying stored results");

    let storage = ResultsStorage::default_dir()?;

    // List gateways if no specific gateway requested
    if args.gateway.is_none() && !args.summary {
        let gateways = storage.list_gateways()?;

        if gateways.is_empty() {
            println!("\n📭 No stored results found.");
            println!("   Run tests with: gateway-poc test --gateway <name> --ip <address>");
            return Ok(());
        }

        println!("\n┌─────────────────────────────────────────────────────────────┐");
        println!("│ Stored Test Results                                          │");
        println!("├─────────────────────────────────────────────────────────────┤");

        for gateway in &gateways {
            let runs = storage.list_runs(gateway)?;
            if !runs.is_empty() {
                let latest = &runs[0];
                println!(
                    "│ {:25} │ {:3} runs │ Latest: {:.1}% │",
                    gateway,
                    runs.len(),
                    latest.pass_rate * 100.0
                );
            }
        }

        println!("└─────────────────────────────────────────────────────────────┘");
        println!("\nUse --gateway <name> to view details for a specific gateway.");
        println!("Use --summary to compare all gateways.\n");

        return Ok(());
    }

    // Show comparison summary
    if args.summary {
        let gateways = storage.list_gateways()?;
        let mut runs = Vec::new();

        for gateway in gateways {
            if let Some(run) = storage.latest(&gateway)? {
                runs.push(run);
            }
        }

        if runs.is_empty() {
            println!("No results to compare.");
            return Ok(());
        }

        let comparison = GatewayComparator::compare(&runs);

        match args.format.as_str() {
            "json" => {
                println!("{}", ComparisonFormatter::format_json(&comparison));
            }
            _ => {
                println!("{}", ComparisonFormatter::format_table(&comparison));
            }
        }

        // Export if requested
        if let Some(export_path) = &args.export {
            let path = PathBuf::from(export_path);
            let format =
                ReportFormat::from_str(path.extension().and_then(|e| e.to_str()).unwrap_or("md"))
                    .unwrap_or(ReportFormat::Markdown);

            let generator = ReportGenerator::new(storage);
            let report = generator.comparison_report(&runs, format);
            std::fs::write(&path, report)?;
            println!("\n✓ Report exported to: {}", path.display());
        }

        return Ok(());
    }

    // Show specific gateway results
    if let Some(gateway) = &args.gateway {
        let runs = storage.load_gateway(gateway)?;

        if runs.is_empty() {
            println!("No results found for gateway: {gateway}");
            return Ok(());
        }

        let latest = &runs[0];

        match args.format.as_str() {
            "json" => {
                println!("{}", serde_json::to_string_pretty(latest)?);
            }
            _ => {
                println!("\n┌─────────────────────────────────────────────────────────────┐");
                println!("│ Gateway: {:50} │", latest.gateway);
                println!("├─────────────────────────────────────────────────────────────┤");
                println!("│ Run ID: {:50} │", latest.id);
                println!("│ IP: {:54} │", latest.gateway_ip);
                println!("│ Rounds: {:50} │", latest.rounds);

                if let Some(agg) = &latest.aggregate {
                    println!("├─────────────────────────────────────────────────────────────┤");
                    println!("│ Pass Rate: {:47.1}% │", agg.avg_pass_rate * 100.0);
                    println!("│ Avg Duration: {:44}ms │", agg.avg_duration_ms);
                    println!("├─────────────────────────────────────────────────────────────┤");
                    println!("│ {:30} {:>8} {:>10} │", "Test", "Pass%", "Avg(ms)");
                    println!("├─────────────────────────────────────────────────────────────┤");

                    for (name, stats) in &agg.test_stats {
                        let short_name = if name.len() > 30 {
                            format!("{}...", &name[..27])
                        } else {
                            name.clone()
                        };
                        println!(
                            "│ {:30} {:>7.1}% {:>10} │",
                            short_name,
                            stats.pass_rate * 100.0,
                            stats.avg_duration_ms
                        );
                    }
                }

                println!("└─────────────────────────────────────────────────────────────┘");

                // Show other runs
                if runs.len() > 1 {
                    println!("\nOther runs ({}):", runs.len() - 1);
                    for run in runs.iter().skip(1).take(5) {
                        let pass_rate = run
                            .aggregate
                            .as_ref()
                            .map(|a| format!("{:.1}%", a.avg_pass_rate * 100.0))
                            .unwrap_or_else(|| "N/A".to_string());
                        println!("  - {} | {} | {}", run.id, run.rounds, pass_rate);
                    }
                }
            }
        }

        // Export if requested
        if let Some(export_path) = &args.export {
            let path = PathBuf::from(export_path);
            let format =
                ReportFormat::from_str(path.extension().and_then(|e| e.to_str()).unwrap_or("md"))
                    .unwrap_or(ReportFormat::Markdown);

            let generator = ReportGenerator::new(storage);
            let report = generator.gateway_report(latest, format);
            std::fs::write(&path, report)?;
            println!("\n✓ Report exported to: {}", path.display());
        }
    }

    Ok(())
}
