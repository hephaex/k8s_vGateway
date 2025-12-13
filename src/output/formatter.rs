//! Output formatters for test results
//!
//! Provides JSON, Table, and summary output formats.

#![allow(dead_code)]

use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;

use crate::executor::AggregateResult;
use crate::models::{GatewayImpl, TestResult, TestRoundSummary, TestStatus};

/// Output format options
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
    JsonPretty,
    Csv,
    Summary,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "table" => Some(OutputFormat::Table),
            "json" => Some(OutputFormat::Json),
            "json-pretty" | "jsonpretty" => Some(OutputFormat::JsonPretty),
            "csv" => Some(OutputFormat::Csv),
            "summary" => Some(OutputFormat::Summary),
            _ => None,
        }
    }
}

/// Result formatter
pub struct ResultFormatter {
    format: OutputFormat,
    colorize: bool,
}

impl ResultFormatter {
    pub fn new(format: OutputFormat) -> Self {
        Self {
            format,
            colorize: true,
        }
    }

    pub fn no_color(mut self) -> Self {
        self.colorize = false;
        self
    }

    /// Format a single test result
    pub fn format_result(&self, result: &TestResult) -> String {
        match self.format {
            OutputFormat::Table => self.format_result_table(result),
            OutputFormat::Json => serde_json::to_string(result).unwrap_or_default(),
            OutputFormat::JsonPretty => serde_json::to_string_pretty(result).unwrap_or_default(),
            OutputFormat::Csv => self.format_result_csv(result),
            OutputFormat::Summary => self.format_result_summary(result),
        }
    }

    fn format_result_table(&self, result: &TestResult) -> String {
        let status_str = if self.colorize {
            match result.status {
                TestStatus::Pass => "\x1b[32m✓ PASS\x1b[0m",
                TestStatus::Fail => "\x1b[31m✗ FAIL\x1b[0m",
                TestStatus::Skip => "\x1b[33m○ SKIP\x1b[0m",
                TestStatus::Error => "\x1b[31m! ERROR\x1b[0m",
            }
        } else {
            match result.status {
                TestStatus::Pass => "✓ PASS",
                TestStatus::Fail => "✗ FAIL",
                TestStatus::Skip => "○ SKIP",
                TestStatus::Error => "! ERROR",
            }
        };

        format!(
            "{:2}. {:20} {} [{:>6}ms]",
            result.test_case.number(),
            result.test_case.name(),
            status_str,
            result.duration_ms
        )
    }

    fn format_result_csv(&self, result: &TestResult) -> String {
        format!(
            "{},{},{},{},\"{}\"",
            result.test_case.number(),
            result.test_case.name(),
            result.status,
            result.duration_ms,
            result.message.as_deref().unwrap_or("").replace('"', "\"\"")
        )
    }

    fn format_result_summary(&self, result: &TestResult) -> String {
        format!(
            "{} {} ({}ms)",
            result.status.symbol(),
            result.test_case.name(),
            result.duration_ms
        )
    }

    /// Format test round summary
    pub fn format_summary(&self, summary: &TestRoundSummary) -> String {
        match self.format {
            OutputFormat::Table => self.format_summary_table(summary),
            OutputFormat::Json => serde_json::to_string(summary).unwrap_or_default(),
            OutputFormat::JsonPretty => serde_json::to_string_pretty(summary).unwrap_or_default(),
            OutputFormat::Csv => self.format_summary_csv(summary),
            OutputFormat::Summary => self.format_summary_brief(summary),
        }
    }

    fn format_summary_table(&self, summary: &TestRoundSummary) -> String {
        let mut output = String::new();

        // Header
        output.push_str("\n╔══════════════════════════════════════════════════════════════╗\n");
        output.push_str(&format!(
            "║  Round {:3} - {:40} ║\n",
            summary.round, summary.gateway
        ));
        output.push_str("╠══════════════════════════════════════════════════════════════╣\n");

        // Results
        for result in &summary.results {
            output.push_str(&format!("║  {}  ║\n", self.format_result_table(result)));
        }

        // Footer
        output.push_str("╠══════════════════════════════════════════════════════════════╣\n");

        let pass_str = if self.colorize {
            format!("\x1b[32m{}\x1b[0m", summary.passed)
        } else {
            summary.passed.to_string()
        };
        let fail_str = if self.colorize && summary.failed > 0 {
            format!("\x1b[31m{}\x1b[0m", summary.failed)
        } else {
            summary.failed.to_string()
        };

        output.push_str(&format!(
            "║  Total: {:2} | Pass: {} | Fail: {} | Skip: {:2} | Error: {:2}     ║\n",
            summary.total, pass_str, fail_str, summary.skipped, summary.errors
        ));
        output.push_str(&format!(
            "║  Pass Rate: {:5.1}% | Duration: {:6}ms                      ║\n",
            summary.pass_rate(),
            summary.total_duration_ms
        ));
        output.push_str("╚══════════════════════════════════════════════════════════════╝\n");

        output
    }

    fn format_summary_csv(&self, summary: &TestRoundSummary) -> String {
        let mut output = String::new();
        output.push_str("test_num,test_name,status,duration_ms,message\n");
        for result in &summary.results {
            output.push_str(&self.format_result_csv(result));
            output.push('\n');
        }
        output
    }

    fn format_summary_brief(&self, summary: &TestRoundSummary) -> String {
        format!(
            "{} Gateway - Round {}: {}/{} passed ({:.1}%) in {}ms",
            summary.gateway,
            summary.round,
            summary.passed,
            summary.total,
            summary.pass_rate(),
            summary.total_duration_ms
        )
    }

    /// Format comparison across multiple gateways
    pub fn format_comparison(&self, results: &HashMap<GatewayImpl, TestRoundSummary>) -> String {
        match self.format {
            OutputFormat::Table => self.format_comparison_table(results),
            OutputFormat::Json | OutputFormat::JsonPretty => {
                let json_results: HashMap<String, &TestRoundSummary> = results
                    .iter()
                    .map(|(k, v)| (k.name().to_string(), v))
                    .collect();
                if self.format == OutputFormat::JsonPretty {
                    serde_json::to_string_pretty(&json_results).unwrap_or_default()
                } else {
                    serde_json::to_string(&json_results).unwrap_or_default()
                }
            }
            _ => self.format_comparison_table(results),
        }
    }

    fn format_comparison_table(&self, results: &HashMap<GatewayImpl, TestRoundSummary>) -> String {
        let mut output = String::new();

        // Header
        output.push_str(
            "\n┌─────────────────────────────────────────────────────────────────────────────┐\n",
        );
        output.push_str(
            "│                        Gateway Comparison Results                           │\n",
        );
        output.push_str(
            "├─────────────────────────┬───────┬───────┬───────┬──────────┬───────────────┤\n",
        );
        output.push_str(
            "│ Gateway                 │ Pass  │ Fail  │ Total │ Rate     │ Duration      │\n",
        );
        output.push_str(
            "├─────────────────────────┼───────┼───────┼───────┼──────────┼───────────────┤\n",
        );

        // Sort by pass rate
        let mut sorted: Vec<_> = results.iter().collect();
        sorted.sort_by(|a, b| b.1.pass_rate().partial_cmp(&a.1.pass_rate()).unwrap());

        for (impl_, summary) in sorted {
            let rate_str = format!("{:5.1}%", summary.pass_rate());
            let rate_colored = if self.colorize {
                if summary.pass_rate() >= 90.0 {
                    format!("\x1b[32m{rate_str}\x1b[0m")
                } else if summary.pass_rate() >= 50.0 {
                    format!("\x1b[33m{rate_str}\x1b[0m")
                } else {
                    format!("\x1b[31m{rate_str}\x1b[0m")
                }
            } else {
                rate_str
            };

            output.push_str(&format!(
                "│ {:23} │ {:5} │ {:5} │ {:5} │ {:>8} │ {:>10}ms │\n",
                impl_.name(),
                summary.passed,
                summary.failed,
                summary.total,
                rate_colored,
                summary.total_duration_ms
            ));
        }

        output.push_str(
            "└─────────────────────────┴───────┴───────┴───────┴──────────┴───────────────┘\n",
        );

        output
    }

    /// Format aggregate results
    pub fn format_aggregate(&self, aggregate: &AggregateResult, gateway: &str) -> String {
        match self.format {
            OutputFormat::Table => self.format_aggregate_table(aggregate, gateway),
            OutputFormat::Json | OutputFormat::JsonPretty => {
                #[derive(Serialize)]
                struct AggregateJson<'a> {
                    gateway: &'a str,
                    total_rounds: u32,
                    overall_pass_rate: f64,
                    test_pass_rates: HashMap<String, f64>,
                }

                let json = AggregateJson {
                    gateway,
                    total_rounds: aggregate.total_rounds,
                    overall_pass_rate: aggregate.overall_pass_rate,
                    test_pass_rates: aggregate
                        .test_pass_rates
                        .iter()
                        .map(|(k, v)| (k.name().to_string(), *v))
                        .collect(),
                };

                if self.format == OutputFormat::JsonPretty {
                    serde_json::to_string_pretty(&json).unwrap_or_default()
                } else {
                    serde_json::to_string(&json).unwrap_or_default()
                }
            }
            _ => self.format_aggregate_table(aggregate, gateway),
        }
    }

    fn format_aggregate_table(&self, aggregate: &AggregateResult, gateway: &str) -> String {
        let mut output = String::new();

        output.push_str("\n═══════════════════════════════════════════════════════════════\n");
        output.push_str(&format!(
            " Aggregate Results: {} ({} rounds)\n",
            gateway, aggregate.total_rounds
        ));
        output.push_str("═══════════════════════════════════════════════════════════════\n");

        output.push_str(&format!(
            " Overall Pass Rate: {:.1}%\n\n",
            aggregate.overall_pass_rate
        ));

        output.push_str(" Test Pass Rates:\n");
        output.push_str(" ───────────────────────────────────────────────────────────\n");

        let mut tests: Vec<_> = aggregate.test_pass_rates.iter().collect();
        tests.sort_by_key(|(tc, _)| tc.number());

        for (test_case, rate) in tests {
            let bar_len = (*rate / 5.0) as usize;
            let bar = "█".repeat(bar_len);
            let empty = "░".repeat(20 - bar_len);

            let rate_str = if self.colorize {
                if *rate >= 90.0 {
                    format!("\x1b[32m{rate:5.1}%\x1b[0m")
                } else if *rate >= 50.0 {
                    format!("\x1b[33m{rate:5.1}%\x1b[0m")
                } else {
                    format!("\x1b[31m{rate:5.1}%\x1b[0m")
                }
            } else {
                format!("{rate:5.1}%")
            };

            output.push_str(&format!(
                " {:2}. {:20} {} {} {}\n",
                test_case.number(),
                test_case.name(),
                bar,
                empty,
                rate_str
            ));
        }

        output.push_str(" ───────────────────────────────────────────────────────────\n");

        // Flaky tests
        let flaky: Vec<_> = aggregate
            .flaky_tests()
            .into_iter()
            .filter(|(_, r)| *r < 100.0)
            .collect();
        if !flaky.is_empty() {
            output.push_str("\n Flaky Tests (< 100% pass rate):\n");
            for (tc, rate) in flaky.iter().take(5) {
                output.push_str(&format!("   - {} ({:.1}%)\n", tc.name(), rate));
            }
        }

        output
    }
}

impl Default for ResultFormatter {
    fn default() -> Self {
        Self::new(OutputFormat::Table)
    }
}

/// Write results to a file
pub fn write_results_to_file(
    path: &str,
    summary: &TestRoundSummary,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let formatter = ResultFormatter::new(format).no_color();
    let content = formatter.format_summary(summary);

    let mut file = std::fs::File::create(path)?;
    file.write_all(content.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TestCase;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::from_str("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("TABLE"), Some(OutputFormat::Table));
        assert_eq!(OutputFormat::from_str("unknown"), None);
    }

    #[test]
    fn test_formatter_creation() {
        let formatter = ResultFormatter::new(OutputFormat::Json).no_color();
        assert_eq!(formatter.format, OutputFormat::Json);
        assert!(!formatter.colorize);
    }

    #[test]
    fn test_format_result() {
        let result = TestResult::pass(TestCase::HostRouting, 100);
        let formatter = ResultFormatter::new(OutputFormat::Summary);
        let output = formatter.format_result(&result);
        assert!(output.contains("Host Routing"));
    }
}
