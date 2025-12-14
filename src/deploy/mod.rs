//! Gateway deployment and management module
//!
//! Provides installation, health checking, and manifest generation
//! for Gateway API implementations.

#![allow(dead_code)]
#![allow(unused_imports)]

mod health;
mod installer;
mod manifest;

pub use health::{
    HealthCheck, HealthCheckConfig, HealthChecker, HealthStatus, PreFlightChecker, PreFlightResult,
};
pub use installer::{GatewayInstaller, InstallResult, InstallStatus, InstallerConfig};
pub use manifest::{
    BackendRef, GatewayManifest, HttpRouteManifest, HttpRouteRule, Listener, ManifestGenerator,
    Metadata, ParentRef,
};
