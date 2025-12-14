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

mod benchmark;
mod cli;
mod config;
mod deploy;
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
        cli::Command::Deploy(deploy_args) => {
            manage_deploy(deploy_args).await?;
        }
        cli::Command::Benchmark(benchmark_args) => {
            run_benchmark(benchmark_args).await?;
        }
        cli::Command::Config(config_args) => {
            manage_config(config_args)?;
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

async fn manage_deploy(args: cli::DeployArgs) -> Result<()> {
    use deploy::{
        GatewayInstaller, HealthCheckConfig, HealthChecker, InstallerConfig, ManifestGenerator,
        PreFlightChecker,
    };

    match args.action {
        cli::DeployAction::Install {
            gateway,
            namespace,
            timeout,
        } => {
            let implementation = GatewayImpl::from_str(&gateway)
                .ok_or_else(|| anyhow::anyhow!("Unknown gateway: {gateway}"))?;

            let config = InstallerConfig::new()
                .namespace(&namespace)
                .timeout(timeout);

            let installer = GatewayInstaller::new(config);

            println!("Installing {} gateway...", implementation.name());

            match installer.install(implementation).await {
                Ok(result) => {
                    println!("\n✓ Installation complete!");
                    println!("  Gateway: {}", result.gateway.name());
                    println!("  Release: {}", result.release_name);
                    println!("  Namespace: {}", result.namespace);
                    println!("  GatewayClass: {}", result.gateway_class);
                    println!("  Status: {}", result.status.as_str());
                }
                Err(e) => {
                    println!("✗ Installation failed: {e}");
                }
            }
        }

        cli::DeployAction::Uninstall { gateway, namespace } => {
            let implementation = GatewayImpl::from_str(&gateway)
                .ok_or_else(|| anyhow::anyhow!("Unknown gateway: {gateway}"))?;

            let config = InstallerConfig::new().namespace(&namespace);
            let installer = GatewayInstaller::new(config);

            println!("Uninstalling {} gateway...", implementation.name());

            match installer.uninstall(implementation).await {
                Ok(()) => {
                    println!("✓ Uninstall complete!");
                }
                Err(e) => {
                    println!("✗ Uninstall failed: {e}");
                }
            }
        }

        cli::DeployAction::List => {
            let config = InstallerConfig::new();
            let installer = GatewayInstaller::new(config);

            println!("\n┌─────────────────────────────────────────────────────────────┐");
            println!("│ Gateway Implementations                                     │");
            println!("├─────────────────────────────────────────────────────────────┤");

            for gateway in GatewayImpl::all() {
                let status = installer
                    .check_installed(gateway)
                    .await
                    .unwrap_or(deploy::InstallStatus::NotInstalled);
                let status_icon = if status.is_installed() { "✓" } else { "○" };
                let arm64 = if gateway.supports_arm64() {
                    "ARM64"
                } else {
                    "AMD64"
                };

                println!(
                    "│ {} {:30} {:10} {:8} │",
                    status_icon,
                    gateway.name(),
                    status.as_str(),
                    arm64
                );
            }

            println!("└─────────────────────────────────────────────────────────────┘\n");
        }

        cli::DeployAction::Health { gateway, ip, port } => {
            let implementation = GatewayImpl::from_str(&gateway)
                .ok_or_else(|| anyhow::anyhow!("Unknown gateway: {gateway}"))?;

            let config = HealthCheckConfig::default();
            let checker = HealthChecker::new(config)?;

            let status = checker.check_gateway(implementation, &ip, port).await;
            println!("{}", status.format_table());
        }

        cli::DeployAction::Preflight { gateway, ip, port } => {
            let implementation = GatewayImpl::from_str(&gateway)
                .ok_or_else(|| anyhow::anyhow!("Unknown gateway: {gateway}"))?;

            let config = HealthCheckConfig::default();
            let checker = PreFlightChecker::new(config)?;

            let result = checker.run(implementation, &ip, port).await;
            println!("{}", result.format_table());

            if !result.passed {
                std::process::exit(1);
            }
        }

        cli::DeployAction::Crds { experimental } => {
            let config = InstallerConfig::new();
            let installer = GatewayInstaller::new(config);

            if experimental {
                println!("Installing experimental Gateway API CRDs...");
                installer.install_gateway_api_experimental().await?;
            } else {
                println!("Installing standard Gateway API CRDs...");
                installer.install_gateway_api_crds().await?;
            }

            println!("✓ Gateway API CRDs installed successfully");
        }

        cli::DeployAction::Manifest {
            gateway,
            resource,
            name,
            format,
        } => {
            let implementation = GatewayImpl::from_str(&gateway)
                .ok_or_else(|| anyhow::anyhow!("Unknown gateway: {gateway}"))?;

            let generator = ManifestGenerator::new(implementation);

            let output = match resource.to_lowercase().as_str() {
                "gateway" => {
                    let manifest = generator.gateway(&name);
                    if format == "json" {
                        ManifestGenerator::to_json(&manifest)
                    } else {
                        ManifestGenerator::to_yaml(&manifest)
                    }
                }
                "httproute" => {
                    let manifest = generator.http_route(&name, "test-gateway");
                    if format == "json" {
                        ManifestGenerator::to_json(&manifest)
                    } else {
                        ManifestGenerator::to_yaml(&manifest)
                    }
                }
                _ => {
                    anyhow::bail!(
                        "Unknown resource type: {resource}. Use 'gateway' or 'httproute'"
                    );
                }
            };

            println!("{output}");
        }
    }

    Ok(())
}

async fn run_benchmark(args: cli::BenchmarkArgs) -> Result<()> {
    use benchmark::{
        BenchmarkConfig, BenchmarkReport, BenchmarkReportFormat, BenchmarkRunner, LoadPattern,
    };
    use std::fs;

    match args.action {
        cli::BenchmarkAction::Run {
            gateway,
            ip,
            port,
            path,
            hostname,
            duration,
            concurrency,
            rps,
            pattern,
            warmup,
            format,
            output,
        } => {
            let implementation = GatewayImpl::from_str(&gateway)
                .ok_or_else(|| anyhow::anyhow!("Unknown gateway: {gateway}"))?;

            // Parse load pattern
            let load_pattern = match pattern.to_lowercase().as_str() {
                "constant" => LoadPattern::Constant { rps },
                "ramp" => LoadPattern::Ramp {
                    start_rps: rps / 2,
                    end_rps: rps,
                    duration_secs: duration,
                },
                "step" => LoadPattern::Step {
                    start_rps: rps / 4,
                    step_rps: rps / 4,
                    step_interval_secs: duration / 4,
                    max_rps: rps,
                },
                "spike" => LoadPattern::Spike {
                    base_rps: rps / 2,
                    spike_rps: rps * 2,
                    spike_duration_secs: duration / 6,
                },
                "max" => LoadPattern::Max { concurrency },
                _ => LoadPattern::Constant { rps },
            };

            let config = BenchmarkConfig::new(implementation, &ip)
                .with_pattern(load_pattern)
                .with_duration(duration)
                .with_concurrency(concurrency)
                .with_path(&path)
                .with_hostname(&hostname);

            // Update config with warmup and port
            let mut config = config;
            config.warmup_secs = warmup;
            config.port = port;

            println!(
                "Starting benchmark for {} at http://{}:{}{}",
                implementation.name(),
                ip,
                port,
                path
            );
            println!("Duration: {duration}s, Concurrency: {concurrency}, Pattern: {pattern:?}");

            let runner = BenchmarkRunner::new(config);
            let result = runner.run().await?;

            // Generate report
            let report_format =
                BenchmarkReportFormat::from_str(&format).unwrap_or(BenchmarkReportFormat::Text);
            let report = BenchmarkReport::single(&result, report_format);

            println!("{report}");

            // Save to file if specified
            if let Some(output_path) = output {
                fs::write(&output_path, &report)?;
                println!("Report saved to: {output_path}");
            }
        }

        cli::BenchmarkAction::Compare {
            gateways,
            ip,
            port,
            duration,
            concurrency,
            rps,
            format,
            output,
        } => {
            let gateway_list: Vec<&str> = gateways.split(',').map(|s| s.trim()).collect();
            let mut results = Vec::new();

            println!("Comparing {} gateways...\n", gateway_list.len());

            for gateway_name in gateway_list {
                if let Some(implementation) = GatewayImpl::from_str(gateway_name) {
                    println!("Benchmarking {}...", implementation.name());

                    let config = BenchmarkConfig::new(implementation, &ip)
                        .with_pattern(LoadPattern::Constant { rps })
                        .with_duration(duration)
                        .with_concurrency(concurrency);

                    let mut config = config;
                    config.port = port;

                    let runner = BenchmarkRunner::new(config);
                    match runner.run().await {
                        Ok(result) => {
                            println!(
                                "  ✓ {}: {:.1} RPS, p99={:.2}ms",
                                implementation.name(),
                                result.metrics.throughput.rps,
                                result.metrics.latency.percentiles.p99
                            );
                            results.push(result);
                        }
                        Err(e) => {
                            println!("  ✗ {}: Failed - {}", implementation.name(), e);
                        }
                    }
                } else {
                    println!("  ⚠ Unknown gateway: {gateway_name}");
                }
            }

            if !results.is_empty() {
                // Generate comparison report
                let report_format =
                    BenchmarkReportFormat::from_str(&format).unwrap_or(BenchmarkReportFormat::Text);
                let report = BenchmarkReport::comparison(&results, report_format);

                println!("\n{report}");

                // Save to file if specified
                if let Some(output_path) = output {
                    fs::write(&output_path, &report)?;
                    println!("Report saved to: {output_path}");
                }
            }
        }

        cli::BenchmarkAction::Histogram { file, buckets } => {
            let content = fs::read_to_string(&file)?;
            let result: benchmark::BenchmarkResult = serde_json::from_str(&content)?;

            println!(
                "\nLatency Histogram for {} Benchmark",
                result.config.gateway.name()
            );
            println!("{:=<60}", "");

            // Create histogram buckets
            let min = result.metrics.latency.min;
            let max = result.metrics.latency.max;
            let range = max - min;
            let _bucket_size = range / buckets as f64;

            println!("\nLatency Distribution (ms):");
            println!(
                "  Min: {:.2}ms, Max: {:.2}ms, Mean: {:.2}ms",
                min, max, result.metrics.latency.mean
            );
            println!(
                "\n  {:>12} {:>12} {:>12}",
                "Range (ms)", "Count", "Histogram"
            );
            println!("  {:->12} {:->12} {:->40}", "", "", "");

            // Note: We don't have individual samples stored, so show percentile-based distribution
            let percentiles = [
                ("0-50%", result.metrics.latency.percentiles.p50),
                ("50-90%", result.metrics.latency.percentiles.p90),
                ("90-95%", result.metrics.latency.percentiles.p95),
                ("95-99%", result.metrics.latency.percentiles.p99),
                ("99-99.9%", result.metrics.latency.percentiles.p999),
            ];

            for (label, value) in percentiles {
                let bar_len = ((value / max) * 40.0) as usize;
                let bar = "█".repeat(bar_len.min(40));
                println!("  {label:>12} {value:>12.2} {bar}");
            }

            println!("\nSummary:");
            println!(
                "  Total Requests: {}",
                result.metrics.throughput.total_requests
            );
            println!(
                "  Success Rate: {:.2}%",
                result.metrics.throughput.success_rate * 100.0
            );
            println!("  RPS: {:.1}", result.metrics.throughput.rps);
        }
    }

    Ok(())
}

fn manage_config(args: cli::ConfigArgs) -> Result<()> {
    use config::{ConfigFile, EnvConfig, ProfileManager, TestProfile};
    use std::path::Path;

    match args.action {
        cli::ConfigAction::Init { output, force } => {
            let path = Path::new(&output);
            if path.exists() && !force {
                anyhow::bail!(
                    "Configuration file already exists: {output}. Use --force to overwrite."
                );
            }

            let config = ConfigFile::example();
            config.save(path)?;
            println!("✓ Configuration file created: {output}");
            println!("\nEdit the file to customize your settings.");
        }

        cli::ConfigAction::Show { env, format } => {
            if env {
                let env_config = EnvConfig::load();
                env_config.print_summary();
            } else {
                let config = ConfigFile::load_default()?;
                let output = if format == "json" {
                    serde_json::to_string_pretty(&config)?
                } else {
                    serde_yaml::to_string(&config)?
                };
                println!("{output}");
            }
        }

        cli::ConfigAction::Validate { file } => {
            let path = file.unwrap_or_else(|| {
                ConfigFile::find()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "./gateway-poc.yaml".to_string())
            });

            match ConfigFile::load(&path) {
                Ok(_) => {
                    println!("✓ Configuration file is valid: {path}");
                }
                Err(e) => {
                    println!("✗ Configuration file is invalid: {path}");
                    println!("  Error: {e}");
                    return Err(e);
                }
            }
        }

        cli::ConfigAction::Profiles {
            gateways,
            tests,
            detailed,
        } => {
            let manager = ProfileManager::new();

            let show_gateways = gateways || !tests;
            let show_tests = tests || !gateways;

            if show_gateways {
                println!("Gateway Profiles:");
                println!("{:-<60}", "");
                for profile in manager.list_gateway_profiles() {
                    if detailed {
                        println!("  {} ({})", profile.name, profile.gateway.name());
                        println!("    Namespace: {}", profile.namespace);
                        println!(
                            "    Ports: HTTP={}, HTTPS={}",
                            profile.http_port, profile.https_port
                        );
                        println!("    Install: {:?}", profile.install_method);
                        println!();
                    } else {
                        println!("  {:20} - {}", profile.name, profile.gateway.name());
                    }
                }
                println!();
            }

            if show_tests {
                println!("Test Profiles:");
                println!("{:-<60}", "");
                for profile in manager.list_test_profiles() {
                    if detailed {
                        println!("  {}", profile.name);
                        println!("    Description: {}", profile.description);
                        println!("    Tests: {:?}", profile.tests);
                        println!(
                            "    Rounds: {}, Parallel: {}",
                            profile.rounds, profile.parallel
                        );
                        println!("    Tags: {:?}", profile.tags);
                        println!();
                    } else {
                        println!(
                            "  {:20} - {} ({} tests)",
                            profile.name,
                            profile.description,
                            profile.tests.len()
                        );
                    }
                }
            }
        }

        cli::ConfigAction::Profile { name, profile_type } => {
            let manager = ProfileManager::new();

            match profile_type.as_str() {
                "gateway" => {
                    if let Some(profile) = manager.gateway_profile(&name) {
                        println!("{}", serde_yaml::to_string(profile)?);
                    } else {
                        println!("Gateway profile not found: {name}");
                        println!("\nAvailable profiles:");
                        for p in manager.list_gateway_profiles() {
                            println!("  - {}", p.name);
                        }
                    }
                }
                "test" => {
                    if let Some(profile) = TestProfile::find(&name) {
                        println!("{}", serde_yaml::to_string(&profile)?);
                    } else {
                        println!("Test profile not found: {name}");
                        println!("\nAvailable profiles:");
                        for p in TestProfile::predefined() {
                            println!("  - {}", p.name);
                        }
                    }
                }
                _ => {
                    println!("Unknown profile type: {profile_type}. Use 'gateway' or 'test'.");
                }
            }
        }

        cli::ConfigAction::Set { key, value, file } => {
            let path = file.unwrap_or_else(|| "./gateway-poc.yaml".to_string());
            let mut config = if Path::new(&path).exists() {
                ConfigFile::load(&path)?
            } else {
                ConfigFile::default()
            };

            let value_display = value.clone();

            // Set value based on key
            match key.as_str() {
                "app.default_gateway" => config.app.default_gateway = value,
                "app.default_rounds" => config.app.default_rounds = value.parse()?,
                "app.timeout_secs" => config.app.timeout_secs = value.parse()?,
                "app.parallel" => config.app.parallel = value.parse()?,
                "app.max_concurrent" => config.app.max_concurrent = value.parse()?,
                _ => {
                    anyhow::bail!("Unknown configuration key: {key}");
                }
            }

            config.save(&path)?;
            println!("✓ Set {key} = {value_display} in {path}");
        }

        cli::ConfigAction::Get { key, file } => {
            let config = if let Some(path) = file {
                ConfigFile::load(&path)?
            } else {
                ConfigFile::load_default()?
            };

            let value = match key.as_str() {
                "app.default_gateway" => config.app.default_gateway.clone(),
                "app.default_rounds" => config.app.default_rounds.to_string(),
                "app.timeout_secs" => config.app.timeout_secs.to_string(),
                "app.parallel" => config.app.parallel.to_string(),
                "app.max_concurrent" => config.app.max_concurrent.to_string(),
                _ => {
                    anyhow::bail!("Unknown configuration key: {key}");
                }
            };

            println!("{value}");
        }

        cli::ConfigAction::Env => {
            config::env::print_env_help();
        }
    }

    Ok(())
}
