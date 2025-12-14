//! Gateway comparison functionality
//!
//! Compare test results across different gateway implementations.

use std::collections::BTreeMap;

use crate::results::storage::{StoredTestRun, TestStats};

/// Comparison result between gateways
#[derive(Clone, Debug)]
pub struct GatewayComparison {
    /// Gateway names being compared
    pub gateways: Vec<String>,

    /// Per-test comparison
    pub test_comparisons: Vec<TestComparison>,

    /// Overall rankings
    pub rankings: GatewayRankings,

    /// Summary statistics
    pub summary: ComparisonSummary,
}

/// Comparison for a single test across gateways
#[derive(Clone, Debug)]
pub struct TestComparison {
    /// Test name
    pub test_name: String,

    /// Test category
    pub category: String,

    /// Results per gateway (gateway name -> stats)
    pub gateway_results: BTreeMap<String, TestComparisonResult>,

    /// Best performing gateway
    pub best_gateway: Option<String>,

    /// Winner criteria
    pub winner_criteria: WinnerCriteria,
}

/// Result for a single gateway in a test comparison
#[derive(Clone, Debug)]
pub struct TestComparisonResult {
    /// Pass rate (0.0 - 1.0)
    pub pass_rate: f64,

    /// Average duration in ms
    pub avg_duration_ms: u64,

    /// Pass count
    pub pass_count: u32,

    /// Fail count
    pub fail_count: u32,

    /// Relative performance score (higher is better)
    pub score: f64,
}

/// Criteria for determining the winner
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WinnerCriteria {
    /// Winner determined by pass rate
    PassRate,
    /// Winner determined by duration
    Duration,
    /// All gateways had same result
    Tie,
    /// No data available
    NoData,
}

/// Overall gateway rankings
#[derive(Clone, Debug)]
pub struct GatewayRankings {
    /// Ranking by overall pass rate
    pub by_pass_rate: Vec<RankedGateway>,

    /// Ranking by average duration
    pub by_duration: Vec<RankedGateway>,

    /// Ranking by combined score
    pub by_score: Vec<RankedGateway>,

    /// Number of tests won per gateway
    pub wins: BTreeMap<String, u32>,
}

/// A gateway with its rank
#[derive(Clone, Debug)]
pub struct RankedGateway {
    /// Rank (1 = best)
    pub rank: u32,

    /// Gateway name
    pub gateway: String,

    /// Value for this ranking
    pub value: f64,
}

/// Summary of comparison
#[derive(Clone, Debug)]
pub struct ComparisonSummary {
    /// Number of gateways compared
    pub gateway_count: usize,

    /// Number of tests compared
    pub test_count: usize,

    /// Best overall gateway (by combined score)
    pub best_overall: Option<String>,

    /// Most reliable gateway (highest pass rate)
    pub most_reliable: Option<String>,

    /// Fastest gateway (lowest avg duration)
    pub fastest: Option<String>,

    /// Tests where all gateways passed
    pub universal_pass: usize,

    /// Tests where all gateways failed
    pub universal_fail: usize,

    /// Tests with mixed results
    pub mixed_results: usize,
}

/// Gateway comparator
pub struct GatewayComparator;

impl GatewayComparator {
    /// Compare multiple gateway test runs
    pub fn compare(runs: &[StoredTestRun]) -> GatewayComparison {
        if runs.is_empty() {
            return GatewayComparison::empty();
        }

        let gateways: Vec<String> = runs.iter().map(|r| r.gateway.clone()).collect();

        // Build per-test comparisons
        let test_comparisons = Self::build_test_comparisons(runs);

        // Calculate rankings
        let rankings = Self::calculate_rankings(runs, &test_comparisons);

        // Build summary
        let summary = Self::build_summary(&gateways, &test_comparisons, &rankings);

        GatewayComparison {
            gateways,
            test_comparisons,
            rankings,
            summary,
        }
    }

    fn build_test_comparisons(runs: &[StoredTestRun]) -> Vec<TestComparison> {
        // Collect all test names
        let mut all_tests: BTreeMap<String, String> = BTreeMap::new(); // name -> category
        for run in runs {
            if let Some(agg) = &run.aggregate {
                for name in agg.test_stats.keys() {
                    if !all_tests.contains_key(name) {
                        // Try to find category from results
                        let category = run
                            .summaries
                            .first()
                            .and_then(|s| s.results.iter().find(|r| &r.test_name == name))
                            .map(|r| r.category.clone())
                            .unwrap_or_else(|| "Unknown".to_string());
                        all_tests.insert(name.clone(), category);
                    }
                }
            }
        }

        // Build comparisons for each test
        all_tests
            .into_iter()
            .map(|(test_name, category)| {
                let mut gateway_results: BTreeMap<String, TestComparisonResult> = BTreeMap::new();

                for run in runs {
                    if let Some(agg) = &run.aggregate {
                        if let Some(stats) = agg.test_stats.get(&test_name) {
                            let result = TestComparisonResult::from_stats(stats);
                            gateway_results.insert(run.gateway.clone(), result);
                        }
                    }
                }

                // Determine winner
                let (best_gateway, winner_criteria) = Self::determine_winner(&gateway_results);

                TestComparison {
                    test_name,
                    category,
                    gateway_results,
                    best_gateway,
                    winner_criteria,
                }
            })
            .collect()
    }

    fn determine_winner(
        results: &BTreeMap<String, TestComparisonResult>,
    ) -> (Option<String>, WinnerCriteria) {
        if results.is_empty() {
            return (None, WinnerCriteria::NoData);
        }

        // First, compare by pass rate
        let max_pass_rate = results.values().map(|r| r.pass_rate).fold(0.0, f64::max);
        let min_pass_rate = results.values().map(|r| r.pass_rate).fold(1.0, f64::min);

        if (max_pass_rate - min_pass_rate).abs() > 0.01 {
            // Significant difference in pass rate
            let winner = results
                .iter()
                .max_by(|a, b| a.1.pass_rate.partial_cmp(&b.1.pass_rate).unwrap())
                .map(|(k, _)| k.clone());
            return (winner, WinnerCriteria::PassRate);
        }

        // All have same pass rate, compare by duration
        let min_duration = results
            .values()
            .map(|r| r.avg_duration_ms)
            .min()
            .unwrap_or(0);
        let max_duration = results
            .values()
            .map(|r| r.avg_duration_ms)
            .max()
            .unwrap_or(0);

        if max_duration > 0 && min_duration < max_duration {
            let winner = results
                .iter()
                .min_by_key(|(_, v)| v.avg_duration_ms)
                .map(|(k, _)| k.clone());
            return (winner, WinnerCriteria::Duration);
        }

        // It's a tie
        (None, WinnerCriteria::Tie)
    }

    fn calculate_rankings(
        runs: &[StoredTestRun],
        comparisons: &[TestComparison],
    ) -> GatewayRankings {
        // Calculate wins per gateway
        let mut wins: BTreeMap<String, u32> = BTreeMap::new();
        for comp in comparisons {
            if let Some(winner) = &comp.best_gateway {
                *wins.entry(winner.clone()).or_insert(0) += 1;
            }
        }

        // Ranking by pass rate
        let mut by_pass_rate: Vec<RankedGateway> = runs
            .iter()
            .filter_map(|r| {
                r.aggregate.as_ref().map(|a| RankedGateway {
                    rank: 0,
                    gateway: r.gateway.clone(),
                    value: a.avg_pass_rate,
                })
            })
            .collect();
        by_pass_rate.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap());
        for (i, r) in by_pass_rate.iter_mut().enumerate() {
            r.rank = i as u32 + 1;
        }

        // Ranking by duration (lower is better)
        let mut by_duration: Vec<RankedGateway> = runs
            .iter()
            .filter_map(|r| {
                r.aggregate.as_ref().map(|a| RankedGateway {
                    rank: 0,
                    gateway: r.gateway.clone(),
                    value: a.avg_duration_ms as f64,
                })
            })
            .collect();
        by_duration.sort_by(|a, b| a.value.partial_cmp(&b.value).unwrap());
        for (i, r) in by_duration.iter_mut().enumerate() {
            r.rank = i as u32 + 1;
        }

        // Combined score ranking
        let mut by_score: Vec<RankedGateway> = runs
            .iter()
            .filter_map(|r| {
                r.aggregate.as_ref().map(|a| {
                    // Score = pass_rate * 100 - log(duration)
                    let duration_factor = (a.avg_duration_ms as f64).ln();
                    let score = a.avg_pass_rate * 100.0 - duration_factor;
                    RankedGateway {
                        rank: 0,
                        gateway: r.gateway.clone(),
                        value: score,
                    }
                })
            })
            .collect();
        by_score.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap());
        for (i, r) in by_score.iter_mut().enumerate() {
            r.rank = i as u32 + 1;
        }

        GatewayRankings {
            by_pass_rate,
            by_duration,
            by_score,
            wins,
        }
    }

    fn build_summary(
        gateways: &[String],
        comparisons: &[TestComparison],
        rankings: &GatewayRankings,
    ) -> ComparisonSummary {
        let mut universal_pass = 0;
        let mut universal_fail = 0;
        let mut mixed_results = 0;

        for comp in comparisons {
            if comp.gateway_results.is_empty() {
                continue;
            }

            let all_pass = comp.gateway_results.values().all(|r| r.pass_rate >= 0.99);
            let all_fail = comp.gateway_results.values().all(|r| r.pass_rate <= 0.01);

            if all_pass {
                universal_pass += 1;
            } else if all_fail {
                universal_fail += 1;
            } else {
                mixed_results += 1;
            }
        }

        ComparisonSummary {
            gateway_count: gateways.len(),
            test_count: comparisons.len(),
            best_overall: rankings.by_score.first().map(|r| r.gateway.clone()),
            most_reliable: rankings.by_pass_rate.first().map(|r| r.gateway.clone()),
            fastest: rankings.by_duration.first().map(|r| r.gateway.clone()),
            universal_pass,
            universal_fail,
            mixed_results,
        }
    }
}

impl TestComparisonResult {
    fn from_stats(stats: &TestStats) -> Self {
        // Calculate score: pass_rate * 100 - normalized_duration
        let duration_score = if stats.avg_duration_ms > 0 {
            (stats.avg_duration_ms as f64).ln() * 5.0
        } else {
            0.0
        };
        let score = stats.pass_rate * 100.0 - duration_score;

        Self {
            pass_rate: stats.pass_rate,
            avg_duration_ms: stats.avg_duration_ms,
            pass_count: stats.pass_count,
            fail_count: stats.fail_count,
            score,
        }
    }
}

impl GatewayComparison {
    fn empty() -> Self {
        Self {
            gateways: Vec::new(),
            test_comparisons: Vec::new(),
            rankings: GatewayRankings {
                by_pass_rate: Vec::new(),
                by_duration: Vec::new(),
                by_score: Vec::new(),
                wins: BTreeMap::new(),
            },
            summary: ComparisonSummary {
                gateway_count: 0,
                test_count: 0,
                best_overall: None,
                most_reliable: None,
                fastest: None,
                universal_pass: 0,
                universal_fail: 0,
                mixed_results: 0,
            },
        }
    }
}

/// Comparison report formatter
pub struct ComparisonFormatter;

impl ComparisonFormatter {
    /// Format comparison as table
    pub fn format_table(comparison: &GatewayComparison) -> String {
        let mut output = String::new();

        // Header
        output
            .push_str("\n╔════════════════════════════════════════════════════════════════════╗\n");
        output
            .push_str("║                    Gateway API Comparison Report                    ║\n");
        output.push_str("╠════════════════════════════════════════════════════════════════════╣\n");

        // Summary
        output.push_str(&format!(
            "║ Gateways: {:2}  │  Tests: {:2}  │  Best Overall: {:20} ║\n",
            comparison.summary.gateway_count,
            comparison.summary.test_count,
            comparison.summary.best_overall.as_deref().unwrap_or("N/A")
        ));

        output.push_str("╠════════════════════════════════════════════════════════════════════╣\n");

        // Rankings
        output.push_str("║ Rankings:                                                          ║\n");
        output.push_str("╟────────────────────────────────────────────────────────────────────╢\n");

        output.push_str("║  By Pass Rate:                                                     ║\n");
        for rank in &comparison.rankings.by_pass_rate {
            output.push_str(&format!(
                "║    #{} {:30} {:.1}%                   ║\n",
                rank.rank,
                rank.gateway,
                rank.value * 100.0
            ));
        }

        output.push_str("╟────────────────────────────────────────────────────────────────────╢\n");
        output.push_str("║  By Duration (fastest):                                            ║\n");
        for rank in &comparison.rankings.by_duration {
            output.push_str(&format!(
                "║    #{} {:30} {:>6.0}ms                  ║\n",
                rank.rank, rank.gateway, rank.value
            ));
        }

        output.push_str("╟────────────────────────────────────────────────────────────────────╢\n");
        output.push_str("║  Test Wins:                                                        ║\n");
        for (gateway, wins) in &comparison.rankings.wins {
            output.push_str(&format!(
                "║    {gateway:30} {wins:>3} wins                      ║\n"
            ));
        }

        output.push_str("╠════════════════════════════════════════════════════════════════════╣\n");

        // Test details (abbreviated)
        output.push_str("║ Test Results:                                                      ║\n");
        output.push_str(&format!(
            "║   Universal Pass: {:2}  │  Universal Fail: {:2}  │  Mixed: {:2}         ║\n",
            comparison.summary.universal_pass,
            comparison.summary.universal_fail,
            comparison.summary.mixed_results
        ));

        output.push_str("╚════════════════════════════════════════════════════════════════════╝\n");

        output
    }

    /// Format comparison as JSON
    pub fn format_json(comparison: &GatewayComparison) -> String {
        serde_json::to_string_pretty(&ComparisonJson::from(comparison)).unwrap_or_default()
    }
}

/// JSON-serializable comparison
#[derive(serde::Serialize)]
struct ComparisonJson {
    gateways: Vec<String>,
    summary: ComparisonSummaryJson,
    rankings: RankingsJson,
}

#[derive(serde::Serialize)]
struct ComparisonSummaryJson {
    gateway_count: usize,
    test_count: usize,
    best_overall: Option<String>,
    most_reliable: Option<String>,
    fastest: Option<String>,
}

#[derive(serde::Serialize)]
struct RankingsJson {
    by_pass_rate: Vec<RankEntryJson>,
    by_duration: Vec<RankEntryJson>,
    wins: BTreeMap<String, u32>,
}

#[derive(serde::Serialize)]
struct RankEntryJson {
    rank: u32,
    gateway: String,
    value: f64,
}

impl From<&GatewayComparison> for ComparisonJson {
    fn from(c: &GatewayComparison) -> Self {
        Self {
            gateways: c.gateways.clone(),
            summary: ComparisonSummaryJson {
                gateway_count: c.summary.gateway_count,
                test_count: c.summary.test_count,
                best_overall: c.summary.best_overall.clone(),
                most_reliable: c.summary.most_reliable.clone(),
                fastest: c.summary.fastest.clone(),
            },
            rankings: RankingsJson {
                by_pass_rate: c
                    .rankings
                    .by_pass_rate
                    .iter()
                    .map(|r| RankEntryJson {
                        rank: r.rank,
                        gateway: r.gateway.clone(),
                        value: r.value,
                    })
                    .collect(),
                by_duration: c
                    .rankings
                    .by_duration
                    .iter()
                    .map(|r| RankEntryJson {
                        rank: r.rank,
                        gateway: r.gateway.clone(),
                        value: r.value,
                    })
                    .collect(),
                wins: c.rankings.wins.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_comparison() {
        let comparison = GatewayComparator::compare(&[]);
        assert_eq!(comparison.gateways.len(), 0);
        assert_eq!(comparison.summary.gateway_count, 0);
    }

    #[test]
    fn test_winner_criteria() {
        let mut results = BTreeMap::new();
        results.insert(
            "Gateway A".to_string(),
            TestComparisonResult {
                pass_rate: 1.0,
                avg_duration_ms: 100,
                pass_count: 10,
                fail_count: 0,
                score: 95.0,
            },
        );
        results.insert(
            "Gateway B".to_string(),
            TestComparisonResult {
                pass_rate: 0.8,
                avg_duration_ms: 50,
                pass_count: 8,
                fail_count: 2,
                score: 85.0,
            },
        );

        let (winner, criteria) = GatewayComparator::determine_winner(&results);
        assert_eq!(winner, Some("Gateway A".to_string()));
        assert_eq!(criteria, WinnerCriteria::PassRate);
    }
}
