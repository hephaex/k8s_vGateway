//! Data models for Gateway API testing
//!
//! This module contains all data structures used throughout the application.

mod gateway;
mod test_result;

pub use gateway::{GatewayConfig, GatewayImpl, TestConfig};
pub use test_result::{TestCase, TestResult, TestRoundSummary, TestStatus};
