//! KubeVirt integration module
//!
//! Provides VM lifecycle management, SSH connectivity, and VMI monitoring
//! for testing Gateway API implementations in virtualized environments.

#![allow(dead_code)]

mod ssh;
mod vm;
mod vmi;

pub use ssh::{SshClient, SshConfig};
pub use vm::{VirtualMachineManager, VmConfig};
pub use vmi::VmiManager;
