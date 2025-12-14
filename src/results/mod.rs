//! Results storage and reporting module
//!
//! Provides persistent storage, comparison, and report generation for test results.

#![allow(dead_code)]

mod compare;
mod report;
mod storage;

pub use compare::{ComparisonFormatter, GatewayComparator};
pub use report::{ReportFormat, ReportGenerator};
pub use storage::ResultsStorage;
