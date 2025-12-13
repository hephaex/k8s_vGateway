//! Test execution engine
//!
//! Provides sequential and parallel test execution capabilities.

mod parallel;
mod runner;

pub use parallel::{AggregateResult, BatchRunner, ParallelExecutor};
pub use runner::TestRunner;
