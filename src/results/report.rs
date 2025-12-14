//! Report generation for test results
//!
//! Generate formatted reports in various output formats.

use std::fmt::Write;

use chrono::{DateTime, Utc};

use crate::results::compare::{GatewayComparator, GatewayComparison};
use crate::results::storage::{ResultsStorage, StoredTestRun};

/// Report generator
pub struct ReportGenerator {
    storage: ResultsStorage,
}

impl ReportGenerator {
    /// Create a new report generator
    pub fn new(storage: ResultsStorage) -> Self {
        Self { storage }
    }

    /// Generate a single gateway report
    pub fn gateway_report(&self, run: &StoredTestRun, format: ReportFormat) -> String {
        match format {
            ReportFormat::Text => self.format_text_report(run),
            ReportFormat::Markdown => self.format_markdown_report(run),
            ReportFormat::Html => self.format_html_report(run),
        }
    }

    /// Generate comparison report
    pub fn comparison_report(&self, runs: &[StoredTestRun], format: ReportFormat) -> String {
        let comparison = GatewayComparator::compare(runs);
        match format {
            ReportFormat::Text => self.format_text_comparison(&comparison),
            ReportFormat::Markdown => self.format_markdown_comparison(&comparison),
            ReportFormat::Html => self.format_html_comparison(&comparison),
        }
    }

    fn format_text_report(&self, run: &StoredTestRun) -> String {
        let mut output = String::new();

        // Header
        writeln!(output, "\n{:=^70}", " Gateway API Test Report ").unwrap();
        writeln!(output).unwrap();

        // Summary
        writeln!(output, "Gateway: {}", run.gateway).unwrap();
        writeln!(output, "IP: {}", run.gateway_ip).unwrap();
        writeln!(output, "Run ID: {}", run.id).unwrap();
        writeln!(output, "Started: {}", format_datetime(&run.started_at)).unwrap();
        writeln!(output, "Completed: {}", format_datetime(&run.completed_at)).unwrap();
        writeln!(output, "Rounds: {}", run.rounds).unwrap();
        writeln!(output).unwrap();

        // Aggregate stats
        if let Some(agg) = &run.aggregate {
            writeln!(output, "{:-^70}", " Aggregate Statistics ").unwrap();
            writeln!(
                output,
                "Average Pass Rate: {:.1}%",
                agg.avg_pass_rate * 100.0
            )
            .unwrap();
            writeln!(
                output,
                "Pass Rate Range: {:.1}% - {:.1}%",
                agg.min_pass_rate * 100.0,
                agg.max_pass_rate * 100.0
            )
            .unwrap();
            writeln!(output, "Average Duration: {}ms", agg.avg_duration_ms).unwrap();
            writeln!(output, "Total Duration: {}ms", agg.total_duration_ms).unwrap();
            writeln!(output).unwrap();

            // Per-test stats
            writeln!(output, "{:-^70}", " Per-Test Statistics ").unwrap();
            writeln!(
                output,
                "{:<25} {:>8} {:>8} {:>8} {:>8}",
                "Test", "Pass%", "Avg(ms)", "Min(ms)", "Max(ms)"
            )
            .unwrap();
            writeln!(output, "{:-<70}", "").unwrap();

            for (name, stats) in &agg.test_stats {
                writeln!(
                    output,
                    "{:<25} {:>7.1}% {:>8} {:>8} {:>8}",
                    truncate(name, 25),
                    stats.pass_rate * 100.0,
                    stats.avg_duration_ms,
                    stats.min_duration_ms,
                    stats.max_duration_ms
                )
                .unwrap();
            }
        }

        // Round details
        writeln!(output, "\n{:-^70}", " Round Details ").unwrap();
        for summary in &run.summaries {
            writeln!(
                output,
                "\nRound {}: {}/{} passed ({:.1}%) in {}ms",
                summary.round,
                summary.passed,
                summary.total,
                summary.pass_rate * 100.0,
                summary.duration_ms
            )
            .unwrap();
        }

        writeln!(output, "\n{:=^70}", "").unwrap();
        output
    }

    fn format_markdown_report(&self, run: &StoredTestRun) -> String {
        let mut output = String::new();

        // Header
        writeln!(output, "# Gateway API Test Report\n").unwrap();
        writeln!(output, "## Summary\n").unwrap();
        writeln!(output, "| Property | Value |").unwrap();
        writeln!(output, "|----------|-------|").unwrap();
        writeln!(output, "| Gateway | {} |", run.gateway).unwrap();
        writeln!(output, "| IP Address | {} |", run.gateway_ip).unwrap();
        writeln!(output, "| Run ID | `{}` |", run.id).unwrap();
        writeln!(output, "| Started | {} |", format_datetime(&run.started_at)).unwrap();
        writeln!(
            output,
            "| Completed | {} |",
            format_datetime(&run.completed_at)
        )
        .unwrap();
        writeln!(output, "| Rounds | {} |", run.rounds).unwrap();

        // Aggregate stats
        if let Some(agg) = &run.aggregate {
            writeln!(output, "\n## Aggregate Statistics\n").unwrap();
            writeln!(output, "| Metric | Value |").unwrap();
            writeln!(output, "|--------|-------|").unwrap();
            writeln!(
                output,
                "| Average Pass Rate | {:.1}% |",
                agg.avg_pass_rate * 100.0
            )
            .unwrap();
            writeln!(
                output,
                "| Min Pass Rate | {:.1}% |",
                agg.min_pass_rate * 100.0
            )
            .unwrap();
            writeln!(
                output,
                "| Max Pass Rate | {:.1}% |",
                agg.max_pass_rate * 100.0
            )
            .unwrap();
            writeln!(output, "| Average Duration | {}ms |", agg.avg_duration_ms).unwrap();
            writeln!(output, "| Total Duration | {}ms |", agg.total_duration_ms).unwrap();

            writeln!(output, "\n## Per-Test Results\n").unwrap();
            writeln!(
                output,
                "| Test | Pass Rate | Avg (ms) | Min (ms) | Max (ms) |"
            )
            .unwrap();
            writeln!(
                output,
                "|------|-----------|----------|----------|----------|"
            )
            .unwrap();

            for (name, stats) in &agg.test_stats {
                writeln!(
                    output,
                    "| {} | {:.1}% | {} | {} | {} |",
                    name,
                    stats.pass_rate * 100.0,
                    stats.avg_duration_ms,
                    stats.min_duration_ms,
                    stats.max_duration_ms
                )
                .unwrap();
            }
        }

        // Round details
        writeln!(output, "\n## Round Details\n").unwrap();
        for summary in &run.summaries {
            writeln!(output, "### Round {}\n", summary.round).unwrap();
            writeln!(output, "- **Passed:** {}/{}", summary.passed, summary.total).unwrap();
            writeln!(output, "- **Pass Rate:** {:.1}%", summary.pass_rate * 100.0).unwrap();
            writeln!(output, "- **Duration:** {}ms\n", summary.duration_ms).unwrap();
        }

        output
    }

    fn format_html_report(&self, run: &StoredTestRun) -> String {
        let mut output = String::new();

        writeln!(output, r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Gateway API Test Report - {}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 40px; background: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background: white; padding: 40px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        h1 {{ color: #333; border-bottom: 2px solid #007bff; padding-bottom: 10px; }}
        h2 {{ color: #555; margin-top: 30px; }}
        table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
        th, td {{ padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background: #007bff; color: white; }}
        tr:hover {{ background: #f8f9fa; }}
        .pass {{ color: #28a745; font-weight: bold; }}
        .fail {{ color: #dc3545; font-weight: bold; }}
        .stat-card {{ display: inline-block; background: #f8f9fa; padding: 20px; margin: 10px; border-radius: 8px; min-width: 150px; text-align: center; }}
        .stat-value {{ font-size: 24px; font-weight: bold; color: #007bff; }}
        .stat-label {{ color: #666; font-size: 14px; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Gateway API Test Report</h1>

        <h2>Summary</h2>
        <div class="stat-card">
            <div class="stat-value">{}</div>
            <div class="stat-label">Gateway</div>
        </div>"#, run.gateway, run.gateway).unwrap();

        if let Some(agg) = &run.aggregate {
            writeln!(
                output,
                r#"
        <div class="stat-card">
            <div class="stat-value">{:.1}%</div>
            <div class="stat-label">Pass Rate</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">{}</div>
            <div class="stat-label">Rounds</div>
        </div>
        <div class="stat-card">
            <div class="stat-value">{}ms</div>
            <div class="stat-label">Avg Duration</div>
        </div>

        <h2>Test Results</h2>
        <table>
            <tr>
                <th>Test</th>
                <th>Pass Rate</th>
                <th>Pass/Fail</th>
                <th>Avg Duration</th>
                <th>Min/Max Duration</th>
            </tr>"#,
                agg.avg_pass_rate * 100.0,
                run.rounds,
                agg.avg_duration_ms
            )
            .unwrap();

            for (name, stats) in &agg.test_stats {
                let pass_class = if stats.pass_rate >= 0.99 {
                    "pass"
                } else {
                    "fail"
                };
                writeln!(
                    output,
                    r#"
            <tr>
                <td>{}</td>
                <td class="{}">{:.1}%</td>
                <td>{} / {}</td>
                <td>{}ms</td>
                <td>{}ms / {}ms</td>
            </tr>"#,
                    name,
                    pass_class,
                    stats.pass_rate * 100.0,
                    stats.pass_count,
                    stats.fail_count,
                    stats.avg_duration_ms,
                    stats.min_duration_ms,
                    stats.max_duration_ms
                )
                .unwrap();
            }

            writeln!(output, "        </table>").unwrap();
        }

        writeln!(
            output,
            r#"
        <h2>Environment</h2>
        <table>
            <tr><th>Property</th><th>Value</th></tr>
            <tr><td>Run ID</td><td><code>{}</code></td></tr>
            <tr><td>Gateway IP</td><td>{}</td></tr>
            <tr><td>Started</td><td>{}</td></tr>
            <tr><td>Completed</td><td>{}</td></tr>
            <tr><td>OS</td><td>{}</td></tr>
            <tr><td>Architecture</td><td>{}</td></tr>
            <tr><td>Tool Version</td><td>{}</td></tr>
        </table>
    </div>
</body>
</html>"#,
            run.id,
            run.gateway_ip,
            format_datetime(&run.started_at),
            format_datetime(&run.completed_at),
            run.environment.os,
            run.environment.arch,
            run.environment.tool_version
        )
        .unwrap();

        output
    }

    fn format_text_comparison(&self, comparison: &GatewayComparison) -> String {
        crate::results::compare::ComparisonFormatter::format_table(comparison)
    }

    fn format_markdown_comparison(&self, comparison: &GatewayComparison) -> String {
        let mut output = String::new();

        writeln!(output, "# Gateway API Comparison Report\n").unwrap();

        writeln!(output, "## Summary\n").unwrap();
        writeln!(output, "| Metric | Value |").unwrap();
        writeln!(output, "|--------|-------|").unwrap();
        writeln!(
            output,
            "| Gateways Compared | {} |",
            comparison.summary.gateway_count
        )
        .unwrap();
        writeln!(
            output,
            "| Tests Compared | {} |",
            comparison.summary.test_count
        )
        .unwrap();
        writeln!(
            output,
            "| Best Overall | {} |",
            comparison.summary.best_overall.as_deref().unwrap_or("N/A")
        )
        .unwrap();
        writeln!(
            output,
            "| Most Reliable | {} |",
            comparison.summary.most_reliable.as_deref().unwrap_or("N/A")
        )
        .unwrap();
        writeln!(
            output,
            "| Fastest | {} |",
            comparison.summary.fastest.as_deref().unwrap_or("N/A")
        )
        .unwrap();

        writeln!(output, "\n## Rankings by Pass Rate\n").unwrap();
        writeln!(output, "| Rank | Gateway | Pass Rate |").unwrap();
        writeln!(output, "|------|---------|-----------|").unwrap();
        for rank in &comparison.rankings.by_pass_rate {
            writeln!(
                output,
                "| {} | {} | {:.1}% |",
                rank.rank,
                rank.gateway,
                rank.value * 100.0
            )
            .unwrap();
        }

        writeln!(output, "\n## Rankings by Speed\n").unwrap();
        writeln!(output, "| Rank | Gateway | Avg Duration |").unwrap();
        writeln!(output, "|------|---------|--------------|").unwrap();
        for rank in &comparison.rankings.by_duration {
            writeln!(
                output,
                "| {} | {} | {:.0}ms |",
                rank.rank, rank.gateway, rank.value
            )
            .unwrap();
        }

        writeln!(output, "\n## Test Wins\n").unwrap();
        writeln!(output, "| Gateway | Wins |").unwrap();
        writeln!(output, "|---------|------|").unwrap();
        for (gateway, wins) in &comparison.rankings.wins {
            writeln!(output, "| {gateway} | {wins} |").unwrap();
        }

        writeln!(output, "\n## Test Result Distribution\n").unwrap();
        writeln!(
            output,
            "- **Universal Pass:** {} tests",
            comparison.summary.universal_pass
        )
        .unwrap();
        writeln!(
            output,
            "- **Universal Fail:** {} tests",
            comparison.summary.universal_fail
        )
        .unwrap();
        writeln!(
            output,
            "- **Mixed Results:** {} tests",
            comparison.summary.mixed_results
        )
        .unwrap();

        output
    }

    fn format_html_comparison(&self, comparison: &GatewayComparison) -> String {
        let mut output = String::new();

        writeln!(output, r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Gateway API Comparison Report</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 40px; background: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background: white; padding: 40px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        h1 {{ color: #333; border-bottom: 2px solid #007bff; padding-bottom: 10px; }}
        h2 {{ color: #555; margin-top: 30px; }}
        table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
        th, td {{ padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background: #007bff; color: white; }}
        tr:hover {{ background: #f8f9fa; }}
        .winner {{ background: #d4edda !important; }}
        .rank-1 {{ font-weight: bold; color: #28a745; }}
        .charts {{ display: flex; flex-wrap: wrap; gap: 20px; }}
        .chart {{ flex: 1; min-width: 300px; background: #f8f9fa; padding: 20px; border-radius: 8px; }}
        .bar {{ height: 20px; background: #007bff; border-radius: 4px; margin: 5px 0; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Gateway API Comparison Report</h1>

        <h2>Best Performers</h2>
        <table>
            <tr>
                <th>Category</th>
                <th>Gateway</th>
            </tr>
            <tr>
                <td>Best Overall</td>
                <td class="winner">{}</td>
            </tr>
            <tr>
                <td>Most Reliable</td>
                <td class="winner">{}</td>
            </tr>
            <tr>
                <td>Fastest</td>
                <td class="winner">{}</td>
            </tr>
        </table>

        <h2>Rankings by Pass Rate</h2>
        <table>
            <tr><th>Rank</th><th>Gateway</th><th>Pass Rate</th></tr>"#,
            comparison.summary.best_overall.as_deref().unwrap_or("N/A"),
            comparison.summary.most_reliable.as_deref().unwrap_or("N/A"),
            comparison.summary.fastest.as_deref().unwrap_or("N/A")
        ).unwrap();

        for rank in &comparison.rankings.by_pass_rate {
            let class = if rank.rank == 1 {
                " class=\"rank-1\""
            } else {
                ""
            };
            writeln!(
                output,
                "            <tr{}><td>{}</td><td>{}</td><td>{:.1}%</td></tr>",
                class,
                rank.rank,
                rank.gateway,
                rank.value * 100.0
            )
            .unwrap();
        }

        writeln!(
            output,
            r#"        </table>

        <h2>Rankings by Speed</h2>
        <table>
            <tr><th>Rank</th><th>Gateway</th><th>Avg Duration</th></tr>"#
        )
        .unwrap();

        for rank in &comparison.rankings.by_duration {
            let class = if rank.rank == 1 {
                " class=\"rank-1\""
            } else {
                ""
            };
            writeln!(
                output,
                "            <tr{}><td>{}</td><td>{}</td><td>{:.0}ms</td></tr>",
                class, rank.rank, rank.gateway, rank.value
            )
            .unwrap();
        }

        writeln!(
            output,
            r#"        </table>

        <h2>Test Statistics</h2>
        <table>
            <tr>
                <th>Category</th>
                <th>Count</th>
            </tr>
            <tr><td>Universal Pass</td><td>{}</td></tr>
            <tr><td>Universal Fail</td><td>{}</td></tr>
            <tr><td>Mixed Results</td><td>{}</td></tr>
        </table>
    </div>
</body>
</html>"#,
            comparison.summary.universal_pass,
            comparison.summary.universal_fail,
            comparison.summary.mixed_results
        )
        .unwrap();

        output
    }
}

/// Report output format
#[derive(Clone, Copy, Debug)]
pub enum ReportFormat {
    Text,
    Markdown,
    Html,
}

impl ReportFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "text" | "txt" => Some(ReportFormat::Text),
            "markdown" | "md" => Some(ReportFormat::Markdown),
            "html" | "htm" => Some(ReportFormat::Html),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ReportFormat::Text => "txt",
            ReportFormat::Markdown => "md",
            ReportFormat::Html => "html",
        }
    }
}

fn format_datetime(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_format() {
        assert!(matches!(
            ReportFormat::from_str("text"),
            Some(ReportFormat::Text)
        ));
        assert!(matches!(
            ReportFormat::from_str("md"),
            Some(ReportFormat::Markdown)
        ));
        assert!(matches!(
            ReportFormat::from_str("html"),
            Some(ReportFormat::Html)
        ));
        assert!(ReportFormat::from_str("unknown").is_none());
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a long string", 10), "this is...");
    }
}
