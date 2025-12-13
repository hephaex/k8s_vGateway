//! Kubernetes API client module
//!
//! Provides Kubernetes resource management for Gateway API testing.

mod client;
mod gateway;
mod httproute;
mod pod;

pub use client::K8sClient;
