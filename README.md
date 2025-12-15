# k8s_vGateway

[![CI](https://github.com/hephaex/k8s_vGateway/actions/workflows/ci.yml/badge.svg)](https://github.com/hephaex/k8s_vGateway/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/gateway-poc.svg)](https://crates.io/crates/gateway-poc)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL%203.0-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

Kubernetes Gateway API Implementation Comparison Tool

A CLI tool to test and compare 7 different Gateway API implementations on Kubernetes with KubeVirt support.

## Supported Gateway Implementations

- **NGINX Gateway Fabric**
- **Envoy Gateway**
- **Istio**
- **Cilium**
- **Contour**
- **Traefik**
- **HAProxy**

## Features

- 17 Gateway API conformance tests
- Performance benchmarking with multiple load patterns
- KubeVirt VM management for isolated testing
- Multiple output formats (table, JSON, CSV, Markdown)
- Configuration profiles for different test scenarios
- Automated gateway deployment and health checks

## Installation

### From crates.io

```bash
cargo install gateway-poc
```

### Pre-built Binaries

Download the latest release for your platform:

```bash
# Linux (x86_64)
curl -LO https://github.com/hephaex/k8s_vGateway/releases/latest/download/gateway-poc-linux-amd64
chmod +x gateway-poc-linux-amd64
sudo mv gateway-poc-linux-amd64 /usr/local/bin/gateway-poc

# macOS (ARM64)
curl -LO https://github.com/hephaex/k8s_vGateway/releases/latest/download/gateway-poc-darwin-arm64
chmod +x gateway-poc-darwin-arm64
sudo mv gateway-poc-darwin-arm64 /usr/local/bin/gateway-poc
```

### From Source

```bash
git clone https://github.com/hephaex/k8s_vGateway.git
cd k8s_vGateway
cargo build --release
```

### Requirements

- Kubernetes cluster with Gateway API CRDs installed
- kubectl configured with cluster access
- Rust 1.70+ (only for building from source)

## Usage

### Run Tests

```bash
# Test specific gateway
gateway-poc test --gateway nginx --ip 10.0.0.1

# Run single test
gateway-poc test --gateway envoy --test 1

# Run all tests with multiple rounds
gateway-poc test --gateway istio --all --rounds 10

# Parallel execution
gateway-poc test --gateway cilium --all --parallel --concurrent 4
```

### List Available Tests

```bash
gateway-poc list --detailed
gateway-poc list --gateways
```

### Deploy Gateway

```bash
# Install Gateway API CRDs
gateway-poc deploy crds

# Install gateway implementation
gateway-poc deploy install nginx --namespace gateway-system

# Check gateway health
gateway-poc deploy health nginx --ip 10.0.0.1
```

### Benchmarking

```bash
# Run benchmark
gateway-poc benchmark run --gateway nginx --ip 10.0.0.1 --duration 60 --rps 1000

# Compare multiple gateways
gateway-poc benchmark compare --gateways nginx,envoy,istio --ip 10.0.0.1
```

### KubeVirt VM Management

```bash
# Create test VMs
gateway-poc vm create --workers 2 --cpu 4 --memory 8

# Check VM status
gateway-poc vm status

# Delete VMs
gateway-poc vm delete --all
```

### Configuration

```bash
# Initialize config file
gateway-poc config init

# Show current configuration
gateway-poc config show

# List available profiles
gateway-poc config profiles --detailed
```

## Test Categories

| Category | Tests | Description |
|----------|-------|-------------|
| Routing | 1-5 | HTTP routing, path matching, header routing |
| TLS | 6-8 | TLS termination, mTLS, certificate management |
| Traffic | 9-12 | Load balancing, rate limiting, retries |
| Advanced | 13-17 | WebSocket, gRPC, cross-namespace routing |

## Output Formats

- `table` - Human-readable table format
- `json` - JSON output
- `json-pretty` - Pretty-printed JSON
- `csv` - CSV format for spreadsheets
- `summary` - Condensed summary view

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `GATEWAY_POC_GATEWAY` | Default gateway implementation | nginx |
| `GATEWAY_POC_IP` | Gateway IP address | - |
| `GATEWAY_POC_TIMEOUT` | Request timeout (seconds) | 30 |
| `GATEWAY_POC_LOG_LEVEL` | Log level (trace/debug/info/warn/error) | info |

## Documentation

- [CHANGELOG](CHANGELOG.md) - Release history
- [CONTRIBUTING](CONTRIBUTING.md) - Contribution guidelines
- [SECURITY](SECURITY.md) - Security policy

## License

GPL-3.0

## Author

hephaex@gmail.com
