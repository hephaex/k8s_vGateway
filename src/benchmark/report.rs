//! Benchmark report generation
//!
//! Provides formatted reports in various output formats.

use serde::{Deserialize, Serialize};

use super::runner::{BenchmarkComparison, BenchmarkResult};

/// Report output format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportFormat {
    /// Plain text table
    Text,
    /// JSON format
    Json,
    /// Pretty-printed JSON
    JsonPretty,
    /// Markdown format
    Markdown,
    /// CSV format
    Csv,
    /// HTML format
    Html,
}

impl ReportFormat {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "text" | "table" => Some(Self::Text),
            "json" => Some(Self::Json),
            "json-pretty" => Some(Self::JsonPretty),
            "markdown" | "md" => Some(Self::Markdown),
            "csv" => Some(Self::Csv),
            "html" => Some(Self::Html),
            _ => None,
        }
    }
}

/// Benchmark report generator
pub struct BenchmarkReport;

impl BenchmarkReport {
    /// Generate a single benchmark report
    pub fn single(result: &BenchmarkResult, format: ReportFormat) -> String {
        match format {
            ReportFormat::Text => Self::single_text(result),
            ReportFormat::Json => serde_json::to_string(result).unwrap_or_default(),
            ReportFormat::JsonPretty => serde_json::to_string_pretty(result).unwrap_or_default(),
            ReportFormat::Markdown => Self::single_markdown(result),
            ReportFormat::Csv => Self::single_csv(result),
            ReportFormat::Html => Self::single_html(result),
        }
    }

    /// Generate comparison report
    pub fn comparison(results: &[BenchmarkResult], format: ReportFormat) -> String {
        let comparison = BenchmarkComparison::new(results.to_vec());
        match format {
            ReportFormat::Text => comparison.format_table(),
            ReportFormat::Json => serde_json::to_string(results).unwrap_or_default(),
            ReportFormat::JsonPretty => serde_json::to_string_pretty(results).unwrap_or_default(),
            ReportFormat::Markdown => Self::comparison_markdown(&comparison),
            ReportFormat::Csv => Self::comparison_csv(results),
            ReportFormat::Html => Self::comparison_html(&comparison),
        }
    }

    /// Single result as text
    fn single_text(result: &BenchmarkResult) -> String {
        let mut output = String::new();
        let m = &result.metrics;
        let c = &result.config;

        output.push_str(&format!(
            "\n{:=^70}\n",
            format!(" {} Benchmark Report ", c.gateway.name())
        ));
        output.push_str("\nConfiguration:\n");
        output.push_str(&format!("  Target URL:    {}\n", c.url()));
        output.push_str(&format!("  Duration:      {} seconds\n", c.duration_secs));
        output.push_str(&format!("  Concurrency:   {}\n", c.concurrency));
        output.push_str(&format!("  Load Pattern:  {:?}\n", c.pattern));

        output.push_str("\nThroughput:\n");
        output.push_str(&format!(
            "  Total Requests:    {:>10}\n",
            m.throughput.total_requests
        ));
        output.push_str(&format!(
            "  Successful:        {:>10}\n",
            m.throughput.successful_requests
        ));
        output.push_str(&format!(
            "  Failed:            {:>10}\n",
            m.throughput.failed_requests
        ));
        output.push_str(&format!(
            "  Requests/sec:      {:>10.2}\n",
            m.throughput.rps
        ));
        output.push_str(&format!(
            "  Success Rate:      {:>9.1}%\n",
            m.throughput.success_rate * 100.0
        ));

        output.push_str("\nLatency (ms):\n");
        output.push_str(&format!("  Min:      {:>10.2}\n", m.latency.min));
        output.push_str(&format!("  Max:      {:>10.2}\n", m.latency.max));
        output.push_str(&format!("  Mean:     {:>10.2}\n", m.latency.mean));
        output.push_str(&format!("  Std Dev:  {:>10.2}\n", m.latency.std_dev));
        output.push_str(&format!(
            "  P50:      {:>10.2}\n",
            m.latency.percentiles.p50
        ));
        output.push_str(&format!(
            "  P90:      {:>10.2}\n",
            m.latency.percentiles.p90
        ));
        output.push_str(&format!(
            "  P95:      {:>10.2}\n",
            m.latency.percentiles.p95
        ));
        output.push_str(&format!(
            "  P99:      {:>10.2}\n",
            m.latency.percentiles.p99
        ));
        output.push_str(&format!(
            "  P99.9:    {:>10.2}\n",
            m.latency.percentiles.p999
        ));

        if m.errors.total() > 0 {
            output.push_str("\nErrors:\n");
            output.push_str(&format!(
                "  Connection:   {:>10}\n",
                m.errors.connection_errors
            ));
            output.push_str(&format!(
                "  Timeout:      {:>10}\n",
                m.errors.timeout_errors
            ));
            output.push_str(&format!("  Client (4xx): {:>10}\n", m.errors.client_errors));
            output.push_str(&format!("  Server (5xx): {:>10}\n", m.errors.server_errors));
            output.push_str(&format!("  Other:        {:>10}\n", m.errors.other_errors));
        }

        output.push_str(&format!("\n{:=^70}\n", ""));
        output
    }

    /// Single result as markdown
    fn single_markdown(result: &BenchmarkResult) -> String {
        let mut output = String::new();
        let m = &result.metrics;
        let c = &result.config;

        output.push_str(&format!("# {} Benchmark Report\n\n", c.gateway.name()));
        output.push_str("## Configuration\n\n");
        output.push_str("| Setting | Value |\n");
        output.push_str("|---------|-------|\n");
        output.push_str(&format!("| Target URL | `{}` |\n", c.url()));
        output.push_str(&format!("| Duration | {} seconds |\n", c.duration_secs));
        output.push_str(&format!("| Concurrency | {} |\n", c.concurrency));

        output.push_str("\n## Throughput\n\n");
        output.push_str("| Metric | Value |\n");
        output.push_str("|--------|-------|\n");
        output.push_str(&format!(
            "| Total Requests | {} |\n",
            m.throughput.total_requests
        ));
        output.push_str(&format!(
            "| Successful | {} |\n",
            m.throughput.successful_requests
        ));
        output.push_str(&format!("| Failed | {} |\n", m.throughput.failed_requests));
        output.push_str(&format!("| Requests/sec | {:.2} |\n", m.throughput.rps));
        output.push_str(&format!(
            "| Success Rate | {:.1}% |\n",
            m.throughput.success_rate * 100.0
        ));

        output.push_str("\n## Latency (milliseconds)\n\n");
        output.push_str("| Percentile | Value |\n");
        output.push_str("|------------|-------|\n");
        output.push_str(&format!("| Min | {:.2} |\n", m.latency.min));
        output.push_str(&format!(
            "| P50 (median) | {:.2} |\n",
            m.latency.percentiles.p50
        ));
        output.push_str(&format!("| P90 | {:.2} |\n", m.latency.percentiles.p90));
        output.push_str(&format!("| P95 | {:.2} |\n", m.latency.percentiles.p95));
        output.push_str(&format!("| P99 | {:.2} |\n", m.latency.percentiles.p99));
        output.push_str(&format!("| P99.9 | {:.2} |\n", m.latency.percentiles.p999));
        output.push_str(&format!("| Max | {:.2} |\n", m.latency.max));
        output.push_str(&format!("| Mean | {:.2} |\n", m.latency.mean));
        output.push_str(&format!("| Std Dev | {:.2} |\n", m.latency.std_dev));

        output
    }

    /// Single result as CSV
    fn single_csv(result: &BenchmarkResult) -> String {
        let m = &result.metrics;
        let c = &result.config;

        let header = "gateway,url,duration_secs,concurrency,total_requests,successful,failed,rps,success_rate,latency_min,latency_max,latency_mean,latency_p50,latency_p90,latency_p95,latency_p99,latency_p999";
        let row = format!(
            "{},{},{},{},{},{},{},{:.2},{:.4},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}",
            c.gateway.short_name(),
            c.url(),
            c.duration_secs,
            c.concurrency,
            m.throughput.total_requests,
            m.throughput.successful_requests,
            m.throughput.failed_requests,
            m.throughput.rps,
            m.throughput.success_rate,
            m.latency.min,
            m.latency.max,
            m.latency.mean,
            m.latency.percentiles.p50,
            m.latency.percentiles.p90,
            m.latency.percentiles.p95,
            m.latency.percentiles.p99,
            m.latency.percentiles.p999
        );

        format!("{header}\n{row}")
    }

    /// Single result as HTML
    fn single_html(result: &BenchmarkResult) -> String {
        let m = &result.metrics;
        let c = &result.config;

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>{} Benchmark Report</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 40px; background: #f5f5f5; }}
        .container {{ max-width: 800px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        h1 {{ color: #333; border-bottom: 2px solid #007bff; padding-bottom: 10px; }}
        h2 {{ color: #555; margin-top: 30px; }}
        table {{ width: 100%; border-collapse: collapse; margin: 15px 0; }}
        th, td {{ padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background: #f8f9fa; font-weight: 600; }}
        .metric {{ font-size: 24px; font-weight: bold; color: #007bff; }}
        .metric-label {{ font-size: 12px; color: #666; text-transform: uppercase; }}
        .metrics-grid {{ display: grid; grid-template-columns: repeat(4, 1fr); gap: 20px; margin: 20px 0; }}
        .metric-card {{ background: #f8f9fa; padding: 20px; border-radius: 8px; text-align: center; }}
        .success {{ color: #28a745; }}
        .warning {{ color: #ffc107; }}
        .danger {{ color: #dc3545; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>{} Benchmark Report</h1>

        <div class="metrics-grid">
            <div class="metric-card">
                <div class="metric">{:.1}</div>
                <div class="metric-label">Requests/sec</div>
            </div>
            <div class="metric-card">
                <div class="metric">{:.2}ms</div>
                <div class="metric-label">P99 Latency</div>
            </div>
            <div class="metric-card">
                <div class="metric {}">{:.1}%</div>
                <div class="metric-label">Success Rate</div>
            </div>
            <div class="metric-card">
                <div class="metric">{}</div>
                <div class="metric-label">Total Requests</div>
            </div>
        </div>

        <h2>Configuration</h2>
        <table>
            <tr><th>Setting</th><th>Value</th></tr>
            <tr><td>Target URL</td><td><code>{}</code></td></tr>
            <tr><td>Duration</td><td>{} seconds</td></tr>
            <tr><td>Concurrency</td><td>{}</td></tr>
        </table>

        <h2>Latency Distribution</h2>
        <table>
            <tr><th>Percentile</th><th>Latency (ms)</th></tr>
            <tr><td>Minimum</td><td>{:.2}</td></tr>
            <tr><td>P50 (Median)</td><td>{:.2}</td></tr>
            <tr><td>P90</td><td>{:.2}</td></tr>
            <tr><td>P95</td><td>{:.2}</td></tr>
            <tr><td>P99</td><td>{:.2}</td></tr>
            <tr><td>P99.9</td><td>{:.2}</td></tr>
            <tr><td>Maximum</td><td>{:.2}</td></tr>
        </table>

        <h2>Throughput</h2>
        <table>
            <tr><th>Metric</th><th>Value</th></tr>
            <tr><td>Total Requests</td><td>{}</td></tr>
            <tr><td>Successful</td><td class="success">{}</td></tr>
            <tr><td>Failed</td><td class="{}">{}</td></tr>
            <tr><td>Success Rate</td><td>{:.2}%</td></tr>
        </table>
    </div>
</body>
</html>"#,
            c.gateway.name(),
            c.gateway.name(),
            m.throughput.rps,
            m.latency.percentiles.p99,
            if m.throughput.success_rate > 0.99 {
                "success"
            } else if m.throughput.success_rate > 0.95 {
                "warning"
            } else {
                "danger"
            },
            m.throughput.success_rate * 100.0,
            m.throughput.total_requests,
            c.url(),
            c.duration_secs,
            c.concurrency,
            m.latency.min,
            m.latency.percentiles.p50,
            m.latency.percentiles.p90,
            m.latency.percentiles.p95,
            m.latency.percentiles.p99,
            m.latency.percentiles.p999,
            m.latency.max,
            m.throughput.total_requests,
            m.throughput.successful_requests,
            if m.throughput.failed_requests > 0 {
                "danger"
            } else {
                ""
            },
            m.throughput.failed_requests,
            m.throughput.success_rate * 100.0
        )
    }

    /// Comparison as markdown
    fn comparison_markdown(comparison: &BenchmarkComparison) -> String {
        let mut output = String::new();

        output.push_str("# Gateway API Benchmark Comparison\n\n");
        output.push_str("## Performance Rankings\n\n");
        output.push_str("| Rank | Gateway | RPS | P50 (ms) | P95 (ms) | P99 (ms) | Success % |\n");
        output.push_str("|------|---------|-----|----------|----------|----------|----------|\n");

        for (i, result) in comparison.by_rps().iter().enumerate() {
            output.push_str(&format!(
                "| {} | {} | {:.1} | {:.2} | {:.2} | {:.2} | {:.1}% |\n",
                i + 1,
                result.config.gateway.name(),
                result.metrics.throughput.rps,
                result.metrics.latency.percentiles.p50,
                result.metrics.latency.percentiles.p95,
                result.metrics.latency.percentiles.p99,
                result.metrics.throughput.success_rate * 100.0
            ));
        }

        output.push_str("\n## Lowest Latency (by P99)\n\n");
        output.push_str("| Rank | Gateway | P99 (ms) | RPS |\n");
        output.push_str("|------|---------|----------|-----|\n");

        for (i, result) in comparison.by_latency().iter().enumerate() {
            output.push_str(&format!(
                "| {} | {} | {:.2} | {:.1} |\n",
                i + 1,
                result.config.gateway.name(),
                result.metrics.latency.percentiles.p99,
                result.metrics.throughput.rps
            ));
        }

        output
    }

    /// Comparison as CSV
    fn comparison_csv(results: &[BenchmarkResult]) -> String {
        let mut output = String::new();
        output.push_str("gateway,rps,success_rate,latency_p50,latency_p95,latency_p99,latency_p999,total_requests,failed_requests\n");

        for result in results {
            let m = &result.metrics;
            output.push_str(&format!(
                "{},{:.2},{:.4},{:.2},{:.2},{:.2},{:.2},{},{}\n",
                result.config.gateway.short_name(),
                m.throughput.rps,
                m.throughput.success_rate,
                m.latency.percentiles.p50,
                m.latency.percentiles.p95,
                m.latency.percentiles.p99,
                m.latency.percentiles.p999,
                m.throughput.total_requests,
                m.throughput.failed_requests
            ));
        }

        output
    }

    /// Comparison as HTML
    fn comparison_html(comparison: &BenchmarkComparison) -> String {
        let results = comparison.by_rps();

        let mut rows = String::new();
        for (i, result) in results.iter().enumerate() {
            let m = &result.metrics;
            let success_class = if m.throughput.success_rate > 0.99 {
                "success"
            } else if m.throughput.success_rate > 0.95 {
                "warning"
            } else {
                "danger"
            };

            rows.push_str(&format!(
                r#"<tr>
                    <td>{}</td>
                    <td><strong>{}</strong></td>
                    <td>{:.1}</td>
                    <td>{:.2}</td>
                    <td>{:.2}</td>
                    <td>{:.2}</td>
                    <td class="{}">{:.1}%</td>
                </tr>"#,
                i + 1,
                result.config.gateway.name(),
                m.throughput.rps,
                m.latency.percentiles.p50,
                m.latency.percentiles.p95,
                m.latency.percentiles.p99,
                success_class,
                m.throughput.success_rate * 100.0
            ));
        }

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Gateway API Benchmark Comparison</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 40px; background: #f5f5f5; }}
        .container {{ max-width: 1000px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        h1 {{ color: #333; border-bottom: 2px solid #007bff; padding-bottom: 10px; }}
        table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
        th, td {{ padding: 12px; text-align: right; border-bottom: 1px solid #ddd; }}
        th {{ background: #007bff; color: white; font-weight: 600; }}
        td:first-child, th:first-child {{ text-align: center; width: 50px; }}
        td:nth-child(2), th:nth-child(2) {{ text-align: left; }}
        tr:nth-child(1) td {{ background: #fff3cd; }}
        tr:nth-child(2) td {{ background: #e8f4ea; }}
        tr:nth-child(3) td {{ background: #fce4d6; }}
        .success {{ color: #28a745; font-weight: bold; }}
        .warning {{ color: #ffc107; font-weight: bold; }}
        .danger {{ color: #dc3545; font-weight: bold; }}
        .legend {{ margin-top: 20px; font-size: 14px; color: #666; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Gateway API Benchmark Comparison</h1>
        <table>
            <tr>
                <th>Rank</th>
                <th>Gateway</th>
                <th>RPS</th>
                <th>P50 (ms)</th>
                <th>P95 (ms)</th>
                <th>P99 (ms)</th>
                <th>Success</th>
            </tr>
            {rows}
        </table>
        <div class="legend">
            <strong>Legend:</strong> 🥇 1st place | 🥈 2nd place | 🥉 3rd place
        </div>
    </div>
</body>
</html>"#
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_format_from_str() {
        assert_eq!(ReportFormat::from_str("text"), Some(ReportFormat::Text));
        assert_eq!(ReportFormat::from_str("json"), Some(ReportFormat::Json));
        assert_eq!(
            ReportFormat::from_str("markdown"),
            Some(ReportFormat::Markdown)
        );
        assert_eq!(ReportFormat::from_str("md"), Some(ReportFormat::Markdown));
        assert_eq!(ReportFormat::from_str("csv"), Some(ReportFormat::Csv));
        assert_eq!(ReportFormat::from_str("html"), Some(ReportFormat::Html));
        assert_eq!(ReportFormat::from_str("invalid"), None);
    }
}
