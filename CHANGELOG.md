# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2] - 2025-12-15

### Added

- CHANGELOG.md for release history
- Documentation links section in README
- GitHub repository topics (14 topics)

## [0.1.1] - 2025-12-15

### Changed

- Updated `dirs` from 5.0.1 to 6.0.0
- Updated `rand` from 0.8.5 to 0.9.2
- Updated `prost` from 0.12.6 to 0.14.1
- Updated `softprops/action-gh-release` from 1 to 2

### Added

- CONTRIBUTING.md with contribution guidelines
- SECURITY.md with security policy
- Issue templates (bug report, feature request, gateway support)
- Pull request template
- Dependabot configuration for automated dependency updates
- Cargo.lock for reproducible builds
- Comprehensive .gitignore for Rust projects

## [0.1.0] - 2025-12-14

### Added

- Initial release of k8s_vGateway
- Support for 7 Gateway API implementations:
  - NGINX Gateway Fabric
  - Envoy Gateway
  - Istio
  - Cilium
  - Contour
  - Traefik
  - HAProxy
- 17 Gateway API conformance tests:
  - Routing tests (1-5): HTTP routing, path matching, header routing
  - TLS tests (6-8): TLS termination, mTLS, certificate management
  - Traffic tests (9-12): Load balancing, rate limiting, retries
  - Advanced tests (13-17): WebSocket, gRPC, cross-namespace routing
- Performance benchmarking with multiple load patterns:
  - Constant load
  - Ramp up/down
  - Step function
  - Spike testing
- KubeVirt VM management for isolated testing
- Gateway deployment automation with health checks
- Configuration profiles for different test scenarios
- Multiple output formats: table, JSON, CSV, Markdown, HTML
- CLI commands:
  - `test` - Run Gateway API tests
  - `list` - List available tests and gateways
  - `vm` - Manage KubeVirt VMs
  - `results` - View test results
  - `deploy` - Deploy and manage gateway implementations
  - `benchmark` - Run performance benchmarks
  - `config` - Manage configuration and profiles
- GitHub Actions CI/CD pipeline
- README with documentation and badges
- GPL-3.0 license

[Unreleased]: https://github.com/hephaex/k8s_vGateway/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/hephaex/k8s_vGateway/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/hephaex/k8s_vGateway/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/hephaex/k8s_vGateway/releases/tag/v0.1.0
