//! Results storage and retrieval
//!
//! Provides persistent storage for test results in JSON format.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use crate::models::{GatewayImpl, TestResult, TestRoundSummary, TestStatus};

/// Stored test run containing all results
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredTestRun {
    /// Unique run ID
    pub id: String,

    /// Gateway implementation tested
    pub gateway: String,

    /// Gateway IP address
    pub gateway_ip: String,

    /// Timestamp when test started
    pub started_at: DateTime<Utc>,

    /// Timestamp when test completed
    pub completed_at: DateTime<Utc>,

    /// Number of rounds
    pub rounds: u32,

    /// Round summaries
    pub summaries: Vec<StoredRoundSummary>,

    /// Aggregate statistics
    pub aggregate: Option<AggregateStats>,

    /// Test configuration
    pub config: TestRunConfig,

    /// Environment info
    pub environment: EnvironmentInfo,
}

/// Stored round summary
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredRoundSummary {
    /// Round number
    pub round: u32,

    /// Total tests run
    pub total: usize,

    /// Tests passed
    pub passed: usize,

    /// Tests failed
    pub failed: usize,

    /// Tests skipped
    pub skipped: usize,

    /// Pass rate (0.0 - 1.0)
    pub pass_rate: f64,

    /// Total duration in milliseconds
    pub duration_ms: u64,

    /// Individual test results
    pub results: Vec<StoredTestResult>,
}

/// Stored test result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredTestResult {
    /// Test case number
    pub test_number: u8,

    /// Test name
    pub test_name: String,

    /// Test category
    pub category: String,

    /// Whether test passed
    pub passed: bool,

    /// Duration in milliseconds
    pub duration_ms: u64,

    /// HTTP status code (if applicable)
    pub status_code: Option<u16>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Additional details
    pub details: BTreeMap<String, String>,
}

/// Aggregate statistics across all rounds
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AggregateStats {
    /// Average pass rate
    pub avg_pass_rate: f64,

    /// Minimum pass rate
    pub min_pass_rate: f64,

    /// Maximum pass rate
    pub max_pass_rate: f64,

    /// Average duration per round
    pub avg_duration_ms: u64,

    /// Total duration
    pub total_duration_ms: u64,

    /// Per-test statistics
    pub test_stats: BTreeMap<String, TestStats>,
}

/// Statistics for a single test across rounds
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestStats {
    /// Number of times passed
    pub pass_count: u32,

    /// Number of times failed
    pub fail_count: u32,

    /// Pass rate
    pub pass_rate: f64,

    /// Average duration
    pub avg_duration_ms: u64,

    /// Min duration
    pub min_duration_ms: u64,

    /// Max duration
    pub max_duration_ms: u64,
}

/// Test run configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestRunConfig {
    /// Hostname used for tests
    pub hostname: String,

    /// HTTP port
    pub http_port: u16,

    /// HTTPS port
    pub https_port: u16,

    /// Timeout in seconds
    pub timeout_secs: u64,

    /// Whether tests ran in parallel
    pub parallel: bool,

    /// Concurrency level
    pub concurrency: usize,
}

/// Environment information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    /// Operating system
    pub os: String,

    /// Architecture
    pub arch: String,

    /// Kubernetes version (if available)
    pub k8s_version: Option<String>,

    /// Gateway version (if available)
    pub gateway_version: Option<String>,

    /// Tool version
    pub tool_version: String,
}

impl Default for TestRunConfig {
    fn default() -> Self {
        Self {
            hostname: "example.com".to_string(),
            http_port: 80,
            https_port: 443,
            timeout_secs: 30,
            parallel: false,
            concurrency: 4,
        }
    }
}

impl Default for EnvironmentInfo {
    fn default() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            k8s_version: None,
            gateway_version: None,
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl StoredTestRun {
    /// Create a new stored test run
    pub fn new(gateway: GatewayImpl, gateway_ip: &str) -> Self {
        Self {
            id: generate_run_id(),
            gateway: gateway.name().to_string(),
            gateway_ip: gateway_ip.to_string(),
            started_at: Utc::now(),
            completed_at: Utc::now(),
            rounds: 0,
            summaries: Vec::new(),
            aggregate: None,
            config: TestRunConfig::default(),
            environment: EnvironmentInfo::default(),
        }
    }

    /// Set configuration
    pub fn with_config(mut self, config: TestRunConfig) -> Self {
        self.config = config;
        self
    }

    /// Add a round summary
    pub fn add_round(&mut self, round: u32, summary: &TestRoundSummary) {
        let stored = StoredRoundSummary::from_round_summary(round, summary);
        self.summaries.push(stored);
        self.rounds = round;
        self.completed_at = Utc::now();
    }

    /// Calculate aggregate statistics
    pub fn calculate_aggregate(&mut self) {
        if self.summaries.is_empty() {
            return;
        }

        let mut pass_rates: Vec<f64> = Vec::new();
        let mut durations: Vec<u64> = Vec::new();
        let mut test_results: BTreeMap<String, Vec<(bool, u64)>> = BTreeMap::new();

        for summary in &self.summaries {
            pass_rates.push(summary.pass_rate);
            durations.push(summary.duration_ms);

            for result in &summary.results {
                test_results
                    .entry(result.test_name.clone())
                    .or_default()
                    .push((result.passed, result.duration_ms));
            }
        }

        let avg_pass_rate = pass_rates.iter().sum::<f64>() / pass_rates.len() as f64;
        let min_pass_rate = pass_rates.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_pass_rate = pass_rates.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let total_duration_ms: u64 = durations.iter().sum();
        let avg_duration_ms = total_duration_ms / durations.len() as u64;

        let mut test_stats: BTreeMap<String, TestStats> = BTreeMap::new();
        for (name, results) in test_results {
            let pass_count = results.iter().filter(|(p, _)| *p).count() as u32;
            let fail_count = results.len() as u32 - pass_count;
            let pass_rate = pass_count as f64 / results.len() as f64;

            let durs: Vec<u64> = results.iter().map(|(_, d)| *d).collect();
            let avg_dur = durs.iter().sum::<u64>() / durs.len() as u64;
            let min_dur = *durs.iter().min().unwrap_or(&0);
            let max_dur = *durs.iter().max().unwrap_or(&0);

            test_stats.insert(
                name,
                TestStats {
                    pass_count,
                    fail_count,
                    pass_rate,
                    avg_duration_ms: avg_dur,
                    min_duration_ms: min_dur,
                    max_duration_ms: max_dur,
                },
            );
        }

        self.aggregate = Some(AggregateStats {
            avg_pass_rate,
            min_pass_rate,
            max_pass_rate,
            avg_duration_ms,
            total_duration_ms,
            test_stats,
        });
    }
}

impl StoredRoundSummary {
    /// Convert from TestRoundSummary
    pub fn from_round_summary(round: u32, summary: &TestRoundSummary) -> Self {
        let results: Vec<StoredTestResult> = summary
            .results
            .iter()
            .map(StoredTestResult::from_test_result)
            .collect();

        let pass_rate = if summary.total > 0 {
            summary.passed as f64 / summary.total as f64
        } else {
            0.0
        };

        Self {
            round,
            total: summary.total,
            passed: summary.passed,
            failed: summary.failed,
            skipped: summary.skipped,
            pass_rate,
            duration_ms: summary.total_duration_ms,
            results,
        }
    }
}

impl StoredTestResult {
    /// Convert from TestResult
    pub fn from_test_result(result: &TestResult) -> Self {
        Self {
            test_number: result.test_case.number(),
            test_name: result.test_case.name().to_string(),
            category: result.test_case.category().to_string(),
            passed: result.status == TestStatus::Pass,
            duration_ms: result.duration_ms,
            status_code: None,
            error: result.message.clone(),
            details: BTreeMap::new(),
        }
    }
}

/// Generate unique run ID
fn generate_run_id() -> String {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let random: u32 = rand::random::<u32>() % 10000;
    format!("{timestamp}_{random:04}")
}

/// Results storage manager
pub struct ResultsStorage {
    /// Base directory for results
    base_dir: PathBuf,
}

impl ResultsStorage {
    /// Create a new results storage
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Create with default directory
    pub fn default_dir() -> Result<Self> {
        let base_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gateway-poc")
            .join("results");
        Ok(Self::new(base_dir))
    }

    /// Ensure storage directory exists
    pub fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.base_dir)?;
        Ok(())
    }

    /// Get path for a gateway's results
    fn gateway_dir(&self, gateway: &str) -> PathBuf {
        self.base_dir.join(gateway.to_lowercase())
    }

    /// Get path for a specific run
    fn run_path(&self, gateway: &str, run_id: &str) -> PathBuf {
        self.gateway_dir(gateway).join(format!("{run_id}.json"))
    }

    /// Save a test run
    pub fn save(&self, run: &StoredTestRun) -> Result<PathBuf> {
        let gateway_dir = self.gateway_dir(&run.gateway);
        fs::create_dir_all(&gateway_dir)?;

        let path = self.run_path(&run.gateway, &run.id);
        let file = File::create(&path).context("Failed to create results file")?;
        let writer = BufWriter::new(file);

        serde_json::to_writer_pretty(writer, run).context("Failed to write results")?;

        info!("Saved test results to {}", path.display());
        Ok(path)
    }

    /// Load a test run
    pub fn load(&self, gateway: &str, run_id: &str) -> Result<StoredTestRun> {
        let path = self.run_path(gateway, run_id);
        let file = File::open(&path).context("Failed to open results file")?;
        let reader = BufReader::new(file);

        let run: StoredTestRun =
            serde_json::from_reader(reader).context("Failed to parse results")?;

        debug!("Loaded test results from {}", path.display());
        Ok(run)
    }

    /// Load all runs for a gateway
    pub fn load_gateway(&self, gateway: &str) -> Result<Vec<StoredTestRun>> {
        let gateway_dir = self.gateway_dir(gateway);
        if !gateway_dir.exists() {
            return Ok(Vec::new());
        }

        let mut runs = Vec::new();
        for entry in fs::read_dir(&gateway_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                match self.load_from_path(&path) {
                    Ok(run) => runs.push(run),
                    Err(e) => {
                        debug!("Failed to load {}: {}", path.display(), e);
                    }
                }
            }
        }

        // Sort by timestamp
        runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(runs)
    }

    /// Load from a specific path
    pub fn load_from_path(&self, path: &Path) -> Result<StoredTestRun> {
        let file = File::open(path).context("Failed to open results file")?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).context("Failed to parse results")
    }

    /// List all gateways with results
    pub fn list_gateways(&self) -> Result<Vec<String>> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }

        let mut gateways = Vec::new();
        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    gateways.push(name.to_string());
                }
            }
        }

        gateways.sort();
        Ok(gateways)
    }

    /// List all runs for a gateway
    pub fn list_runs(&self, gateway: &str) -> Result<Vec<RunInfo>> {
        let gateway_dir = self.gateway_dir(gateway);
        if !gateway_dir.exists() {
            return Ok(Vec::new());
        }

        let mut runs = Vec::new();
        for entry in fs::read_dir(&gateway_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(run) = self.load_from_path(&path) {
                    runs.push(RunInfo {
                        id: run.id,
                        gateway: run.gateway,
                        started_at: run.started_at,
                        rounds: run.rounds,
                        pass_rate: run
                            .aggregate
                            .as_ref()
                            .map(|a| a.avg_pass_rate)
                            .unwrap_or(0.0),
                    });
                }
            }
        }

        runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(runs)
    }

    /// Get latest run for a gateway
    pub fn latest(&self, gateway: &str) -> Result<Option<StoredTestRun>> {
        let runs = self.load_gateway(gateway)?;
        Ok(runs.into_iter().next())
    }

    /// Delete a run
    pub fn delete(&self, gateway: &str, run_id: &str) -> Result<()> {
        let path = self.run_path(gateway, run_id);
        if path.exists() {
            fs::remove_file(&path)?;
            info!("Deleted results: {}", path.display());
        }
        Ok(())
    }

    /// Delete all runs for a gateway
    pub fn delete_gateway(&self, gateway: &str) -> Result<()> {
        let gateway_dir = self.gateway_dir(gateway);
        if gateway_dir.exists() {
            fs::remove_dir_all(&gateway_dir)?;
            info!("Deleted all results for gateway: {gateway}");
        }
        Ok(())
    }

    /// Export run to a file
    pub fn export(&self, run: &StoredTestRun, path: &Path, format: ExportFormat) -> Result<()> {
        match format {
            ExportFormat::Json => {
                let file = File::create(path)?;
                let writer = BufWriter::new(file);
                serde_json::to_writer_pretty(writer, run)?;
            }
            ExportFormat::Csv => {
                let mut writer = csv::Writer::from_path(path)?;

                // Write header
                writer.write_record([
                    "round",
                    "test_number",
                    "test_name",
                    "category",
                    "passed",
                    "duration_ms",
                    "status_code",
                    "error",
                ])?;

                // Write results
                for summary in &run.summaries {
                    for result in &summary.results {
                        writer.write_record([
                            summary.round.to_string(),
                            result.test_number.to_string(),
                            result.test_name.clone(),
                            result.category.clone(),
                            result.passed.to_string(),
                            result.duration_ms.to_string(),
                            result
                                .status_code
                                .map(|s| s.to_string())
                                .unwrap_or_default(),
                            result.error.clone().unwrap_or_default(),
                        ])?;
                    }
                }
                writer.flush()?;
            }
        }

        info!("Exported results to {}", path.display());
        Ok(())
    }
}

/// Brief run information
#[derive(Clone, Debug)]
pub struct RunInfo {
    pub id: String,
    pub gateway: String,
    pub started_at: DateTime<Utc>,
    pub rounds: u32,
    pub pass_rate: f64,
}

/// Export format
#[derive(Clone, Copy, Debug)]
pub enum ExportFormat {
    Json,
    Csv,
}

impl ExportFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(ExportFormat::Json),
            "csv" => Some(ExportFormat::Csv),
            _ => None,
        }
    }

    pub fn from_extension(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|e| e.to_str())
            .and_then(Self::from_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_run_id() {
        let id1 = generate_run_id();
        let id2 = generate_run_id();
        assert!(!id1.is_empty());
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_stored_test_run() {
        let run = StoredTestRun::new(GatewayImpl::Nginx, "10.0.0.1");
        assert_eq!(run.gateway, "NGINX Gateway Fabric");
        assert_eq!(run.gateway_ip, "10.0.0.1");
        assert_eq!(run.rounds, 0);
    }

    #[test]
    fn test_export_format() {
        assert!(matches!(
            ExportFormat::from_str("json"),
            Some(ExportFormat::Json)
        ));
        assert!(matches!(
            ExportFormat::from_str("csv"),
            Some(ExportFormat::Csv)
        ));
        assert!(ExportFormat::from_str("unknown").is_none());
    }

    #[test]
    fn test_environment_info() {
        let env = EnvironmentInfo::default();
        assert!(!env.os.is_empty());
        assert!(!env.arch.is_empty());
        assert_eq!(env.tool_version, env!("CARGO_PKG_VERSION"));
    }
}
