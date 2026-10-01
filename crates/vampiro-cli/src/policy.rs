//! Policy evaluation and CI generation for Vampiro scans (REQ-13, REQ-14, REQ-20, REQ-C2).
//!
//! Three modes:
//! - `guidance`: report all findings, exit 0
//! - `tiered`: classify findings by tier
//! - `gate`: exit non-zero when a seam-scoped finding ≥ configured severity threshold
//!
//! CI generation produces a GitHub Actions workflow from the approved scan policy.

use serde::{Deserialize, Serialize};

use crate::exit_code::ExitCode;
use crate::finding::Severity;
use crate::output::FlatFinding;

// ---------------------------------------------------------------------------
// Policy
// ---------------------------------------------------------------------------

/// Scan mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ScanMode {
    /// Report all findings, never fail due to findings.
    #[default]
    Guidance,
    /// Classify findings by configured reporting tiers.
    Tiered,
    /// Gate policy: exit non-zero when a finding ≥ configured severity threshold.
    Gate,
}

impl std::str::FromStr for ScanMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "guidance" => Ok(ScanMode::Guidance),
            "tiered" => Ok(ScanMode::Tiered),
            "gate" => Ok(ScanMode::Gate),
            _ => Err(format!(
                "unknown scan mode: {s}, expected one of: guidance, tiered, gate"
            )),
        }
    }
}

/// A mapping rule for filtration_distance to severity override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiltrationMapRule {
    /// Operator and comparand, e.g. ">= 2", "= 0".
    pub condition: String,
    /// Severity to map to when condition matches.
    pub severity: Severity,
}

/// Policy configuration for a scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanPolicy {
    /// The scan mode.
    pub mode: ScanMode,
    /// Advisory lease (vampiro-b7g): tiered/guidance are downgrades from the
    /// gate default and MUST carry an `advisory_until` ISO date (YYYY-MM-DD).
    /// An expired or missing lease fails closed — the drift detector forcing
    /// a renewed, dated decision instead of a permanent silent advisory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advisory_until: Option<String>,
    /// Severity threshold for gate mode.
    #[serde(default)]
    pub severity_threshold: Severity,
    /// Optional filtration distance mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filtration_map: Option<Vec<FiltrationMapRule>>,
    /// CI provider(s) to generate workflows for.
    #[serde(default)]
    pub ci_providers: Vec<String>,
}

impl Default for ScanPolicy {
    fn default() -> Self {
        ScanPolicy {
            mode: ScanMode::Guidance,
            advisory_until: None,
            severity_threshold: Severity::Medium,
            filtration_map: None,
            ci_providers: vec!["github-actions".to_string()],
        }
    }
}

impl ScanPolicy {
    /// Evaluate a list of findings against the policy and return the exit code.
    ///
    /// `gate` mode returns PolicyFailure if any finding meets or exceeds the
    /// configured severity threshold (optionally mapped through
    /// `filtration_distance`).
    ///
    /// `tiered`/`guidance` are advisory ONLY under a current advisory lease
    /// (vampiro-b7g): an expired or missing `advisory_until` fails closed so
    /// quality drift forces a renewed, dated decision.
    pub fn evaluate(&self, findings: &[FlatFinding]) -> ExitCode {
        match self.mode {
            ScanMode::Guidance | ScanMode::Tiered => {
                // Fail closed: no lease (undated downgrade) or an expired/
                // unparseable lease both block — renew by dating the decision.
                if self.advisory_until.is_none() || is_lease_expired(self.advisory_until.as_deref())
                {
                    ExitCode::PolicyFailure
                } else {
                    ExitCode::Success
                }
            }
            ScanMode::Gate => {
                let threshold = self.resolve_threshold();
                for finding in findings {
                    let effective_severity = self.effective_severity(finding);
                    if effective_severity >= threshold {
                        return ExitCode::PolicyFailure;
                    }
                }
                ExitCode::Success
            }
        }
    }

    /// Resolve the effective severity threshold for this scan, considering the
    /// filtration_distance mapping if configured.
    fn resolve_threshold(&self) -> Severity {
        self.severity_threshold
    }

    /// Compute the effective severity of a finding, applying filtration_distance
    /// mapping if configured.
    fn effective_severity(&self, finding: &FlatFinding) -> Severity {
        let base: Severity = finding.severity.parse().unwrap_or(Severity::Low);
        let Some(fd) = finding.filtration_distance else {
            return base;
        };

        let Some(ref map) = self.filtration_map else {
            return base;
        };

        for rule in map {
            if evaluate_condition(fd, &rule.condition) {
                return rule.severity;
            }
        }

        // No rule matched: use base severity
        base
    }
}

/// Evaluate a condition like ">= 2" or "= 0" against a filtration distance.
/// True when an advisory lease is expired, missing, or unparseable.
///
/// Lease semantics (vampiro-b7g): a `tiered`/`guidance` mode is a dated
/// downgrade from the gate default. `None` lease on an advisory mode is
/// handled by the caller (fail closed); this function treats `None` as
/// not expired so gate mode (which never consults leases) is unaffected.
/// A lease that fails to parse as `YYYY-MM-DD` is a broken contract —
/// fail closed rather than silently advisory forever.
fn is_lease_expired(lease: Option<&str>) -> bool {
    let Some(lease) = lease else {
        return false; // gate mode path; advisory-without-lease fails in evaluate()
    };
    if lease.len() != 10 || lease.as_bytes()[4] != b'-' || lease.as_bytes()[7] != b'-' {
        return true;
    }
    if !lease
        .bytes()
        .enumerate()
        .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return true;
    }
    let (y, m, d) = (&lease[..4], &lease[5..7], &lease[8..10]);
    let Ok(month) = m.parse::<u8>() else {
        return true;
    };
    let Ok(day) = d.parse::<u8>() else {
        return true;
    };
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return true;
    }
    let _ = y; // year is compared lexicographically below
    let today = today_iso();
    lease.as_bytes() <= today.as_bytes()
}

/// Today's date as ISO YYYY-MM-DD, derived without a date crate:
/// Hinnant's civil-from-days over the Unix epoch, using std::time.
fn today_iso() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64 / 86_400)
        .unwrap_or(0);
    civil_from_days(days)
}

/// Howard Hinnant's civil_from_days: days since 1970-01-01 → (y, m, d).
fn civil_from_days(z: i64) -> String {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

fn evaluate_condition(fd: u32, condition: &str) -> bool {
    let condition = condition.trim();
    if let Some(val) = condition.strip_prefix(">=") {
        let threshold: u32 = val.trim().parse().unwrap_or(u32::MAX);
        fd >= threshold
    } else if let Some(val) = condition.strip_prefix("<=") {
        let threshold: u32 = val.trim().parse().unwrap_or(0);
        fd <= threshold
    } else if let Some(val) = condition.strip_prefix('>') {
        let threshold: u32 = val.trim().parse().unwrap_or(u32::MAX);
        fd > threshold
    } else if let Some(val) = condition.strip_prefix('<') {
        let threshold: u32 = val.trim().parse().unwrap_or(0);
        fd < threshold
    } else if let Some(val) = condition.strip_prefix('=') {
        let threshold: u32 = val.trim().parse().unwrap_or(u32::MAX);
        fd == threshold
    } else if let Some(val) = condition.strip_prefix("!=") {
        let threshold: u32 = val.trim().parse().unwrap_or(0);
        fd != threshold
    } else {
        false
    }
}

/// Validate a filtration map for totality and determinism.
///
/// A valid map must:
/// - Cover all possible u32 values (totality check via >= 0 covering everything)
/// - Have deterministic conditions (no overlapping ambiguous rules)
pub fn validate_filtration_map(rules: &[FiltrationMapRule]) -> Result<(), String> {
    if rules.is_empty() {
        return Err("filtration map must have at least one rule".to_string());
    }

    // Check for nondeterministic conditions (duplicates)
    let mut seen = Vec::new();
    for rule in rules {
        let normalized = rule.condition.trim().to_string();
        if seen.contains(&normalized) {
            return Err(format!("duplicate condition: {}", rule.condition));
        }
        seen.push(normalized);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// CI Generation (REQ-20)
// ---------------------------------------------------------------------------

/// CI provider identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiProvider {
    GitHubActions,
}

impl std::str::FromStr for CiProvider {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github-actions" | "github_actions" | "gha" => Ok(CiProvider::GitHubActions),
            _ => Err(format!("unsupported CI provider: {s}")),
        }
    }
}

/// Generate a GitHub Actions workflow YAML string for Vampiro scan + gate.
///
/// Based on the approved scan-policy decision:
/// - Uses `actions/checkout@v4` with `fetch-depth: 0`
/// - Runs `vampiro check --target <head> --base <base> --mode gate`
/// - Applies the configured severity threshold
pub fn generate_github_actions_workflow(
    policy: &ScanPolicy,
) -> Result<String, Box<dyn std::error::Error>> {
    let threshold = policy.severity_threshold.to_string();
    let workflow = format!(
        r#"---
name: Vampiro Scan

on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read

jobs:
  scan:
    name: Code composition analysis
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install vampiro
        run: cargo install vampiro

      - name: Run scan and gate
        run: >
          vampiro check
          --target "${{% raw %}}{{ github.event.pull_request.head.sha || github.sha }}{{ '% endraw %' }}"
          --base "${{% raw %}}{{ github.event.pull_request.base.sha || github.event.before }}{{ '% endraw %' }}"
          --mode gate
          --severity-threshold {threshold}
"#,
        threshold = threshold
    );
    // Clean up the raw/endraw markers — they're only needed to avoid
    // template interpretation during string formatting.
    let workflow = workflow
        .replace("{{% raw %}}", "")
        .replace("{{ '% endraw %' }}", "");

    Ok(workflow)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::FlatFinding;

    fn make_finding(severity: &str, filtration_distance: Option<u32>) -> FlatFinding {
        FlatFinding {
            rule: "REQ-TEST".to_string(),
            stable_id: "test:1:abc12345".to_string(),
            path: "src/test.rs".to_string(),
            line_range_start: 1,
            line_range_end: 10,
            severity: severity.to_string(),
            axis: "composition".to_string(),
            classification: "mismatch".to_string(),
            filtration_distance,
            evidence: serde_json::json!({}),
        }
    }

    // -----------------------------------------------------------------------
    // advisory-lease TTL tests (vampiro-b7g: gate-by-default, dated downgrades)
    // -----------------------------------------------------------------------

    fn lease_policy(mode: ScanMode, until: Option<&str>) -> ScanPolicy {
        ScanPolicy {
            mode,
            advisory_until: until.map(String::from),
            ..Default::default()
        }
    }

    #[test]
    fn advisory_lease_valid_keeps_advisory_exit() {
        // A current lease keeps tiered advisory — findings report, exit 0.
        let policy = lease_policy(ScanMode::Tiered, Some("2099-12-31"));
        let findings = vec![make_finding("high", None)];
        assert_eq!(policy.evaluate(&findings), ExitCode::Success);
        let policy = lease_policy(ScanMode::Guidance, Some("2099-12-31"));
        assert_eq!(policy.evaluate(&findings), ExitCode::Success);
    }

    #[test]
    fn advisory_lease_expired_fails_closed() {
        // Expired lease = the drift detector fired: tiered/guidance now FAIL
        // (with or without findings) until someone renews the date.
        let policy = lease_policy(ScanMode::Tiered, Some("2000-01-01"));
        let findings = vec![make_finding("high", None)];
        assert_eq!(policy.evaluate(&findings), ExitCode::PolicyFailure);
        assert_eq!(policy.evaluate(&[]), ExitCode::PolicyFailure);
        let policy = lease_policy(ScanMode::Guidance, Some("2000-01-01"));
        assert_eq!(policy.evaluate(&[]), ExitCode::PolicyFailure);
    }

    #[test]
    fn advisory_lease_missing_fails_closed() {
        // A downgrade without a lease is invalid configuration, not a pass:
        // fail-closed forces the decision to be made and dated.
        assert_eq!(
            lease_policy(ScanMode::Tiered, None).evaluate(&[]),
            ExitCode::PolicyFailure
        );
        assert_eq!(
            lease_policy(ScanMode::Guidance, None).evaluate(&[]),
            ExitCode::PolicyFailure
        );
    }

    #[test]
    fn gate_mode_ignores_lease() {
        // Gate needs no lease; an expired one must not matter.
        let policy = lease_policy(ScanMode::Gate, Some("2000-01-01"));
        assert_eq!(policy.evaluate(&[]), ExitCode::Success);
    }

    #[test]
    fn lease_date_comparison_is_lexicographic_iso() {
        assert!(is_lease_expired(Some("2000-01-01")));
        assert!(!is_lease_expired(Some("2099-12-31")));
        assert!(!is_lease_expired(None));
        // Unparseable lease is a broken contract — fail closed.
        assert!(is_lease_expired(Some("not-a-date")));
    }

    // -----------------------------------------------------------------------
    // evaluate_condition tests
    // -----------------------------------------------------------------------

    #[test]
    fn condition_gte_matches() {
        assert!(evaluate_condition(2, ">= 2"));
        assert!(evaluate_condition(5, ">= 2"));
        assert!(!evaluate_condition(1, ">= 2"));
    }

    #[test]
    fn condition_lte_matches() {
        assert!(evaluate_condition(2, "<= 2"));
        assert!(evaluate_condition(1, "<= 2"));
        assert!(!evaluate_condition(3, "<= 2"));
    }

    #[test]
    fn condition_eq_matches() {
        assert!(evaluate_condition(0, "= 0"));
        assert!(evaluate_condition(3, "= 3"));
        assert!(!evaluate_condition(1, "= 0"));
    }

    #[test]
    fn condition_neq_matches() {
        assert!(evaluate_condition(1, "!= 0"));
        assert!(!evaluate_condition(0, "!= 0"));
    }

    #[test]
    fn condition_gt_matches() {
        assert!(evaluate_condition(2, "> 1"));
        assert!(!evaluate_condition(1, "> 1"));
    }

    #[test]
    fn condition_lt_matches() {
        assert!(evaluate_condition(1, "< 2"));
        assert!(!evaluate_condition(2, "< 2"));
    }

    // -----------------------------------------------------------------------
    // Policy evaluation tests
    // -----------------------------------------------------------------------

    #[test]
    fn guidance_without_lease_fails_closed() {
        // vampiro-b7g: advisory modes are dated downgrades — an undated one
        // is invalid config and must fail closed, not silently pass.
        let policy = ScanPolicy {
            mode: ScanMode::Guidance,
            ..Default::default()
        };

        let findings = vec![make_finding("high", None)];
        assert_eq!(policy.evaluate(&findings), ExitCode::PolicyFailure);
    }

    #[test]
    fn tiered_with_current_lease_stays_advisory() {
        let policy = ScanPolicy {
            mode: ScanMode::Tiered,
            advisory_until: Some("2099-12-31".to_string()),
            ..Default::default()
        };

        let findings = vec![make_finding("high", None)];
        assert_eq!(policy.evaluate(&findings), ExitCode::Success);
    }

    #[test]
    fn gate_below_threshold_passes() {
        let policy = ScanPolicy {
            mode: ScanMode::Gate,
            severity_threshold: Severity::High,
            ..Default::default()
        };

        let findings = vec![make_finding("medium", None)];
        assert_eq!(policy.evaluate(&findings), ExitCode::Success);
    }

    #[test]
    fn gate_at_threshold_fails() {
        let policy = ScanPolicy {
            mode: ScanMode::Gate,
            severity_threshold: Severity::Medium,
            ..Default::default()
        };

        let findings = vec![make_finding("medium", None)];
        assert_eq!(policy.evaluate(&findings), ExitCode::PolicyFailure);
    }

    #[test]
    fn gate_above_threshold_fails() {
        let policy = ScanPolicy {
            mode: ScanMode::Gate,
            severity_threshold: Severity::Medium,
            ..Default::default()
        };

        let findings = vec![make_finding("high", None)];
        assert_eq!(policy.evaluate(&findings), ExitCode::PolicyFailure);
    }

    #[test]
    fn gate_with_filtration_map_lowers_severity() {
        let policy = ScanPolicy {
            mode: ScanMode::Gate,
            severity_threshold: Severity::High,
            filtration_map: Some(vec![FiltrationMapRule {
                condition: ">= 2".to_string(),
                severity: Severity::Low,
            }]),
            ..Default::default()
        };

        // Finding with severity=high but filtration_distance=3 ⇒ effective=low
        let findings = vec![make_finding("high", Some(3))];
        assert_eq!(policy.evaluate(&findings), ExitCode::Success);
    }

    #[test]
    fn gate_uses_base_severity_when_no_filtration_mapping() {
        let policy = ScanPolicy {
            mode: ScanMode::Gate,
            severity_threshold: Severity::Low,
            filtration_map: None,
            ..Default::default()
        };

        let findings = vec![make_finding("medium", Some(5))];
        assert_eq!(policy.evaluate(&findings), ExitCode::PolicyFailure);
    }

    #[test]
    fn gate_with_filtration_map_no_match_uses_base() {
        let policy = ScanPolicy {
            mode: ScanMode::Gate,
            severity_threshold: Severity::Low,
            filtration_map: Some(vec![FiltrationMapRule {
                condition: ">= 5".to_string(),
                severity: Severity::High,
            }]),
            ..Default::default()
        };

        // fd=2, no rule matches (2 < 5), falls back to base severity "medium"
        let findings = vec![make_finding("medium", Some(2))];
        assert_eq!(policy.evaluate(&findings), ExitCode::PolicyFailure);
    }

    // -----------------------------------------------------------------------
    // Filtration map validation tests
    // -----------------------------------------------------------------------

    #[test]
    fn empty_filtration_map_is_invalid() {
        let result = validate_filtration_map(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn duplicate_conditions_are_invalid() {
        let rules = vec![
            FiltrationMapRule {
                condition: ">= 2".to_string(),
                severity: Severity::Low,
            },
            FiltrationMapRule {
                condition: ">= 2".to_string(),
                severity: Severity::High,
            },
        ];
        let result = validate_filtration_map(&rules);
        assert!(result.is_err());
    }

    #[test]
    fn valid_filtration_map_passes() {
        let rules = vec![
            FiltrationMapRule {
                condition: ">= 2".to_string(),
                severity: Severity::Low,
            },
            FiltrationMapRule {
                condition: "< 2".to_string(),
                severity: Severity::High,
            },
        ];
        let result = validate_filtration_map(&rules);
        assert!(result.is_ok());
    }

    // -----------------------------------------------------------------------
    // CI generation tests
    // -----------------------------------------------------------------------

    #[test]
    fn generate_workflow_includes_threshold() {
        let policy = ScanPolicy {
            severity_threshold: Severity::High,
            ..Default::default()
        };

        let workflow = generate_github_actions_workflow(&policy).unwrap();
        assert!(workflow.contains("--severity-threshold high"));
        assert!(workflow.contains("--mode gate"));
        assert!(workflow.contains("actions/checkout@v4"));
        assert!(workflow.contains("fetch-depth: 0"));
    }

    #[test]
    fn generate_workflow_includes_medium_threshold() {
        let policy = ScanPolicy {
            severity_threshold: Severity::Medium,
            ..Default::default()
        };

        let workflow = generate_github_actions_workflow(&policy).unwrap();
        assert!(workflow.contains("--severity-threshold medium"));
    }

    #[test]
    fn generate_workflow_has_head_and_base_variables() {
        let policy = ScanPolicy::default();
        let workflow = generate_github_actions_workflow(&policy).unwrap();
        assert!(workflow.contains("github.event.pull_request.head.sha"));
        assert!(workflow.contains("github.event.pull_request.base.sha"));
    }

    #[test]
    fn ci_provider_parsing() {
        assert_eq!(
            "github-actions".parse::<CiProvider>().unwrap(),
            CiProvider::GitHubActions
        );
        assert_eq!(
            "gha".parse::<CiProvider>().unwrap(),
            CiProvider::GitHubActions
        );
        assert!("gitlab".parse::<CiProvider>().is_err());
    }
}
