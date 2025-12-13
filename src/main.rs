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
mod models;
mod output;
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

async fn manage_vm(_args: cli::VmArgs) -> Result<()> {
    info!("KubeVirt VM management - Coming in Sprint 5");
    println!("KubeVirt integration will be implemented in Sprint 5.");
    println!("This will enable testing of AMD64-only gateways (kgateway) on ARM64 hosts.");
    Ok(())
}

fn show_results(_args: cli::ResultsArgs) -> Result<()> {
    info!("Results viewer - displaying stored results");
    println!("Results storage and retrieval will be fully implemented.");
    Ok(())
}
