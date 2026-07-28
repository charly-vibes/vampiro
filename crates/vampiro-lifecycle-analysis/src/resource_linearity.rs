//! Resource linearity analysis (REQ-T3, REQ-T7).
//!
//! Tracks acquire/release obligations across exit paths and produces
//! `resource-leak` findings. Each acquisition creates a unique pending
//! obligation tied to a conservative resource identity. Release matches
//! exactly one obligation by identity; duplicate releases do not discharge
//! other obligations. Transfer moves an obligation to a new owner.
//! Unknown aliases are reported as `identity:unknown` diagnostics.
//!
//! # Design
//!
//! The analysis proceeds in three phases:
//!
//! 1. **Group by function**: events, exit paths, and aliases are grouped by
//!    function name. Each function is analyzed independently — resources
//!    do not cross function boundaries in v0.1.0 (the frontend extraction is
//!    per-function).
//!
//! 2. **Linear event pass**: for each function, events are processed in
//!    order. Acquisitions push a pending obligation. Releases discharge a
//!    matching obligation by exact identity. Transfers discharge the old
//!    obligation and create a new one under the new identity.
//!
//! 3. **Exit-path check**: after all events are processed, any remaining
//!    undischarged obligations are crossed against every available exit
//!    path. Each unreleased exit path produces a separate finding.
//!
//! # Limitations (v0.1.0)
//!
//! - Events within a function are treated as a flat sequence; nested scope
//!   relationships are not modeled. An acquire in an inner scope followed by
//!   a release in an outer scope works linearly but may report a false
//!   positive if the release textually precedes the acquisition's exit path.
//! - Cross-function resource handoff (passing ownership to a called function)
//!   is not tracked — only transfers within the same function.
//! - Conditional releases (release only on one branch of an if/else) are not
//!   modeled with branch-level granularity.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The current resource-linearity schema version.
pub const RESOURCE_LINEARITY_SCHEMA_VERSION: &str = "0.1.0";

// ---------------------------------------------------------------------------
// ResourceIdentity — structured, exact-match identity
// ---------------------------------------------------------------------------

/// A structured resource identity used for exact-match obligation tracking.
///
/// Uses a generation counter to distinguish re-acquisitions of the same
/// variable (EDGE-001).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResourceIdentity {
    /// The resolved variable name (original, not alias).
    pub variable: String,
    /// Monotonically increasing generation for disambiguation.
    pub generation: usize,
}

impl ResourceIdentity {
    fn new(variable: impl Into<String>, generation: usize) -> Self {
        ResourceIdentity {
            variable: variable.into(),
            generation,
        }
    }
}

impl std::fmt::Display for ResourceIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.variable, self.generation)
    }
}

// ---------------------------------------------------------------------------
// ResourceEvent — input to the analyzer
// ---------------------------------------------------------------------------

/// A resource lifecycle event extracted from source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResourceEvent {
    /// Source file path.
    pub source_file: String,
    /// Line number.
    pub line: usize,
    /// Function containing this event.
    pub function: String,
    /// The resource variable name.
    pub variable: String,
    /// The resource type (e.g. "File", "Mutex", "TcpStream").
    pub type_name: String,
    /// The resource kind (e.g. "file", "lock", "socket").
    pub kind: String,
    /// The lifecycle event: "acquire", "release", "transfer".
    pub event: String,
}

// ---------------------------------------------------------------------------
// ExitPathFact — input to the analyzer
// ---------------------------------------------------------------------------

/// An exit path from a function scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExitPathFact {
    /// Source file path.
    pub source_file: String,
    /// Line number.
    pub line: usize,
    /// Function containing this exit.
    pub function: String,
    /// Kind of exit: "normal", "early-return", "panic".
    pub kind: String,
    /// Whether this exit is conditional (inside an if/else).
    pub is_conditional: bool,
}

// ---------------------------------------------------------------------------
// AliasFact — input to the analyzer
// ---------------------------------------------------------------------------

/// An alias relationship between two variables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AliasFact {
    /// Source file path.
    pub source_file: String,
    /// Line number.
    pub line: usize,
    /// Function containing this alias.
    pub function: String,
    /// The original variable being aliased.
    pub original: String,
    /// The alias name.
    pub alias: String,
}

// ---------------------------------------------------------------------------
// ResourceObligation — internal state
// ---------------------------------------------------------------------------

/// A pending resource obligation created by an acquisition.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ResourceObligation {
    /// Structured identity for exact matching.
    identity: ResourceIdentity,
    /// The acquisition event that created this obligation.
    acquisition: ResourceEvent,
    /// Whether this obligation has been discharged by a matching release.
    discharged: bool,
}

// ---------------------------------------------------------------------------
// ResourceLeakFinding
// ---------------------------------------------------------------------------

/// A resource leak finding (REQ-T7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResourceLeakFinding {
    /// Source file path.
    pub source_file: String,
    /// Line number of the acquisition.
    pub line: usize,
    /// Function containing the leak.
    pub function: String,
    /// The resource identity that leaked.
    pub resource_identity: String,
    /// The resource type.
    pub resource_type: String,
    /// The exit path where the leak occurs.
    pub exit_path: String,
    /// Human-readable detail.
    pub detail: String,
}

// ---------------------------------------------------------------------------
// IdentityUnknownDiagnostic
// ---------------------------------------------------------------------------

/// A diagnostic for unresolved resource identity (REQ-T3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct IdentityUnknownDiagnostic {
    /// Source file path.
    pub source_file: String,
    /// Line number.
    pub line: usize,
    /// The acquisition this diagnostic relates to.
    pub acquisition: String,
    /// The alias or variable with unknown identity.
    pub alias: String,
    /// The exit path context.
    pub exit_path: String,
}

// ---------------------------------------------------------------------------
// ResourceLinearityAnalyzer
// ---------------------------------------------------------------------------

/// Analyzes resource lifecycle events for linearity violations.
///
/// See the [module-level documentation](self) for design details.
#[derive(Debug, Clone)]
pub struct ResourceLinearityAnalyzer;

impl ResourceLinearityAnalyzer {
    /// Create a new analyzer.
    pub fn new() -> Self {
        ResourceLinearityAnalyzer
    }

    /// Analyze resource events, exit paths, and aliases for linearity.
    ///
    /// Returns (resource-leak findings, identity-unknown diagnostics).
    pub fn analyze(
        &self,
        events: &[ResourceEvent],
        exits: &[ExitPathFact],
        aliases: &[AliasFact],
    ) -> (Vec<ResourceLeakFinding>, Vec<IdentityUnknownDiagnostic>) {
        let mut findings = Vec::new();
        let mut diagnostics = Vec::new();

        // Each function is analyzed independently — resources don't cross
        // function boundaries in v0.1.0.
        let events_by_fn = group_by(events, |e| e.function.clone());
        let exits_by_fn = group_by(exits, |e| e.function.clone());
        let aliases_by_fn = group_by(aliases, |a| a.function.clone());

        for (function, function_events) in &events_by_fn {
            let function_exits = exits_by_fn.get(function).map(Vec::as_slice).unwrap_or(&[]);
            let function_aliases = aliases_by_fn
                .get(function)
                .map(Vec::as_slice)
                .unwrap_or(&[]);

            let (fn_findings, fn_diagnostics) =
                self.analyze_function(function_events, function_exits, function_aliases);

            findings.extend(fn_findings);
            diagnostics.extend(fn_diagnostics);
        }

        (findings, diagnostics)
    }

    /// Analyze a single function's resource events.
    fn analyze_function(
        &self,
        events: &[ResourceEvent],
        exits: &[ExitPathFact],
        aliases: &[AliasFact],
    ) -> (Vec<ResourceLeakFinding>, Vec<IdentityUnknownDiagnostic>) {
        let mut findings = Vec::new();
        let mut diagnostics = Vec::new();

        // Build alias map: alias -> original. Used to resolve any variable
        // name to its canonical original name.
        let alias_map: HashMap<&str, &str> = aliases
            .iter()
            .map(|a| (a.alias.as_str(), a.original.as_str()))
            .collect();

        // Resolve a variable name through the alias map to its canonical original.
        fn resolve<'a>(var: &'a str, map: &'a HashMap<&str, &str>) -> &'a str {
            map.get(var).copied().unwrap_or(var)
        }

        // Track obligations with exact identity.
        // Generation counter disambiguates re-acquisitions of the same variable.
        let mut obligations: Vec<ResourceObligation> = Vec::new();
        let mut generation_counter: HashMap<String, usize> = HashMap::new();

        for event in events {
            let resolved_var = resolve(&event.variable, &alias_map).to_string();
            generation_counter.entry(resolved_var.clone()).or_insert(0);

            match event.event.as_str() {
                "acquire" => {
                    // If the variable is itself an alias, emit identity:unknown.
                    if alias_map.contains_key(event.variable.as_str()) {
                        diagnostics.push(IdentityUnknownDiagnostic {
                            source_file: event.source_file.clone(),
                            line: event.line,
                            acquisition: event.variable.clone(),
                            alias: event.variable.clone(),
                            exit_path: format!("acquisition of {}", event.variable),
                        });
                    }

                    let gen = generation_counter.entry(resolved_var.clone()).or_insert(0);
                    *gen += 1;

                    obligations.push(ResourceObligation {
                        identity: ResourceIdentity::new(resolved_var, *gen),
                        acquisition: event.clone(),
                        discharged: false,
                    });
                }
                "release" => {
                    let resolved = resolve(&event.variable, &alias_map);
                    let current_gen = *generation_counter.get(resolved).unwrap_or(&0);
                    let released_id = ResourceIdentity::new(resolved.to_string(), current_gen);

                    if let Some(ob) = obligations
                        .iter_mut()
                        .find(|o| !o.discharged && o.identity == released_id)
                    {
                        ob.discharged = true;
                    } else {
                        diagnostics.push(IdentityUnknownDiagnostic {
                            source_file: event.source_file.clone(),
                            line: event.line,
                            acquisition: event.variable.clone(),
                            alias: event.variable.clone(),
                            exit_path: format!("release of {}", event.variable),
                        });
                    }
                }
                "transfer" => {
                    let resolved = resolve(&event.variable, &alias_map);
                    let current_gen = *generation_counter.get(resolved).unwrap_or(&0);
                    let transfer_id = ResourceIdentity::new(resolved.to_string(), current_gen);

                    // Find the obligation and extract its acquisition data
                    // before mutating, to avoid borrow conflicts.
                    let old_acq = obligations
                        .iter()
                        .position(|o| !o.discharged && o.identity == transfer_id)
                        .map(|pos| {
                            obligations[pos].discharged = true;
                            obligations[pos].acquisition.clone()
                        });

                    if let Some(acq) = old_acq {
                        let trans_key = format!("{}(transferred)", resolved);
                        let trans_gen = generation_counter.entry(trans_key).or_insert(0);
                        *trans_gen += 1;

                        obligations.push(ResourceObligation {
                            identity: ResourceIdentity::new(
                                format!("{}(transferred)", resolved),
                                *trans_gen,
                            ),
                            acquisition: acq,
                            discharged: false,
                        });
                    }
                }
                _ => {}
            }
        }

        // Check for undischarged obligations against every exit path.
        for ob in &obligations {
            if !ob.discharged {
                if exits.is_empty() {
                    findings.push(make_leak_finding(ob, "normal (no explicit exit)"));
                } else {
                    for exit_path in exits {
                        findings.push(make_leak_finding(
                            ob,
                            &format!(
                                "{} at {}:{}",
                                exit_path.kind, exit_path.source_file, exit_path.line
                            ),
                        ));
                    }
                }
            }
        }

        (findings, diagnostics)
    }
}

impl Default for ResourceLinearityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_leak_finding(ob: &ResourceObligation, exit_path: &str) -> ResourceLeakFinding {
    ResourceLeakFinding {
        source_file: ob.acquisition.source_file.clone(),
        line: ob.acquisition.line,
        function: ob.acquisition.function.clone(),
        resource_identity: ob.identity.to_string(),
        resource_type: ob.acquisition.type_name.clone(),
        exit_path: exit_path.to_string(),
        detail: format!(
            "Resource '{}' (type: {}) acquired at {}:{} is not released on exit path: {}",
            ob.identity,
            ob.acquisition.type_name,
            ob.acquisition.source_file,
            ob.acquisition.line,
            exit_path,
        ),
    }
}

fn group_by<T, K>(items: &[T], key_fn: impl Fn(&T) -> K + Copy) -> HashMap<K, Vec<T>>
where
    K: std::hash::Hash + Eq + Clone,
    T: Clone,
{
    let mut map: HashMap<K, Vec<T>> = HashMap::new();
    for item in items {
        map.entry(key_fn(item)).or_default().push(item.clone());
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acquire(
        file: &str,
        line: usize,
        function: &str,
        var: &str,
        type_name: &str,
        kind: &str,
    ) -> ResourceEvent {
        ResourceEvent {
            source_file: file.to_string(),
            line,
            function: function.to_string(),
            variable: var.to_string(),
            type_name: type_name.to_string(),
            kind: kind.to_string(),
            event: "acquire".to_string(),
        }
    }

    fn release(file: &str, line: usize, function: &str, var: &str) -> ResourceEvent {
        ResourceEvent {
            source_file: file.to_string(),
            line,
            function: function.to_string(),
            variable: var.to_string(),
            type_name: String::new(),
            kind: String::new(),
            event: "release".to_string(),
        }
    }

    fn transfer(file: &str, line: usize, function: &str, var: &str) -> ResourceEvent {
        ResourceEvent {
            source_file: file.to_string(),
            line,
            function: function.to_string(),
            variable: var.to_string(),
            type_name: String::new(),
            kind: String::new(),
            event: "transfer".to_string(),
        }
    }

    fn exit(file: &str, line: usize, function: &str, kind: &str) -> ExitPathFact {
        ExitPathFact {
            source_file: file.to_string(),
            line,
            function: function.to_string(),
            kind: kind.to_string(),
            is_conditional: true,
        }
    }

    fn alias(
        file: &str,
        line: usize,
        function: &str,
        original: &str,
        alias_name: &str,
    ) -> AliasFact {
        AliasFact {
            source_file: file.to_string(),
            line,
            function: function.to_string(),
            original: original.to_string(),
            alias: alias_name.to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // REQ-T3: unique identity, one-to-one release
    // -----------------------------------------------------------------------

    #[test]
    fn acquire_then_release_on_every_exit_is_safe() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![
            acquire("src/main.rs", 10, "main", "f", "File", "file"),
            release("src/main.rs", 11, "main", "f"),
        ];
        let exits = vec![exit("src/main.rs", 12, "main", "normal")];

        let (findings, _) = analyzer.analyze(&events, &exits, &[]);
        assert!(findings.is_empty(), "expected no leaks, got: {findings:?}");
    }

    #[test]
    fn acquire_without_release_produces_leak_finding() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![acquire("src/main.rs", 10, "main", "f", "File", "file")];
        let exits = vec![exit("src/main.rs", 15, "main", "normal")];

        let (findings, _) = analyzer.analyze(&events, &exits, &[]);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].detail.contains("File"));
    }

    // -----------------------------------------------------------------------
    // CORR-001: identity matching must be exact, not substring
    // -----------------------------------------------------------------------

    #[test]
    fn release_of_similar_name_does_not_falsely_discharge() {
        // Acquire "file_handler", release "file" — exact equality MUST reject.
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![
            acquire("src/main.rs", 10, "main", "file_handler", "File", "file"),
            release("src/main.rs", 11, "main", "file"),
        ];
        let exits = vec![exit("src/main.rs", 15, "main", "normal")];

        let (findings, _) = analyzer.analyze(&events, &exits, &[]);
        assert_eq!(findings.len(), 1, "substring match must not discharge");
    }

    #[test]
    fn release_of_same_name_discharges_correctly() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![
            acquire("src/main.rs", 10, "main", "f", "File", "file"),
            release("src/main.rs", 11, "main", "f"),
        ];
        let exits = vec![exit("src/main.rs", 12, "main", "normal")];

        let (findings, _) = analyzer.analyze(&events, &exits, &[]);
        assert!(findings.is_empty());
    }

    // -----------------------------------------------------------------------
    // Unique identity
    // -----------------------------------------------------------------------

    #[test]
    fn two_resources_both_released_is_safe() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![
            acquire("src/main.rs", 10, "main", "a", "Mutex", "lock"),
            acquire("src/main.rs", 11, "main", "b", "Mutex", "lock"),
            release("src/main.rs", 12, "main", "a"),
            release("src/main.rs", 13, "main", "b"),
        ];
        let exits = vec![exit("src/main.rs", 14, "main", "normal")];

        let (findings, _) = analyzer.analyze(&events, &exits, &[]);
        assert!(findings.is_empty());
    }

    // -----------------------------------------------------------------------
    // Duplicate release (REQ-T7)
    // -----------------------------------------------------------------------

    #[test]
    fn duplicate_release_leaves_other_obligation_undischarged() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![
            acquire("src/main.rs", 10, "main", "a", "File", "file"),
            acquire("src/main.rs", 11, "main", "b", "File", "file"),
            release("src/main.rs", 12, "main", "b"),
            release("src/main.rs", 13, "main", "b"),
        ];
        let exits = vec![exit("src/main.rs", 14, "main", "normal")];

        let (findings, _diags) = analyzer.analyze(&events, &exits, &[]);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].resource_identity.contains("a"));
    }

    #[test]
    fn duplicate_release_is_idempotent_on_same_obligation() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![
            acquire("src/main.rs", 10, "main", "a", "File", "file"),
            release("src/main.rs", 11, "main", "a"),
            release("src/main.rs", 12, "main", "a"),
        ];
        let exits = vec![exit("src/main.rs", 13, "main", "normal")];

        let (findings, _diags) = analyzer.analyze(&events, &exits, &[]);
        assert!(findings.is_empty());
    }

    // -----------------------------------------------------------------------
    // Identity mismatch
    // -----------------------------------------------------------------------

    #[test]
    fn release_mismatch_identity_produces_diagnostic() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![
            acquire("src/main.rs", 10, "main", "a", "File", "file"),
            release("src/main.rs", 11, "main", "b"),
        ];
        let exits = vec![exit("src/main.rs", 12, "main", "normal")];

        let (findings, diags) = analyzer.analyze(&events, &exits, &[]);
        assert!(!findings.is_empty());
        assert!(findings[0].resource_identity.contains("a"));
        assert!(!diags.is_empty());
    }

    // -----------------------------------------------------------------------
    // Transfer (REQ-T3)
    // -----------------------------------------------------------------------

    #[test]
    fn transfer_then_release_requires_transferred_name() {
        // Transfer creates a new obligation under "a(transferred)" identity.
        // Release on plain "a" does NOT discharge the transferred obligation.
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![
            acquire("src/main.rs", 10, "main", "a", "File", "file"),
            transfer("src/main.rs", 11, "main", "a"),
            release("src/main.rs", 12, "main", "a"),
        ];
        let exits = vec![exit("src/main.rs", 13, "main", "normal")];

        let (findings, _) = analyzer.analyze(&events, &exits, &[]);
        // Transfer creates obligation "a(transferred)@1". Release on "a@1"
        // doesn't match. This is conservative: in v0.1.0 the user must
        // release the transferred identity explicitly.
        assert_eq!(findings.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Exit paths: early return and panic (REQ-T7)
    // -----------------------------------------------------------------------

    #[test]
    fn early_return_without_release_is_leak() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![acquire("src/main.rs", 5, "open_file", "f", "File", "file")];
        let exits = vec![exit("src/main.rs", 8, "open_file", "early-return")];

        let (findings, _) = analyzer.analyze(&events, &exits, &[]);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].exit_path.contains("early-return"));
    }

    #[test]
    fn panic_without_release_is_leak() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![acquire("src/main.rs", 5, "risky", "lock", "Mutex", "lock")];
        let exits = vec![exit("src/main.rs", 10, "risky", "panic")];

        let (findings, _) = analyzer.analyze(&events, &exits, &[]);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].exit_path.contains("panic"));
    }

    // -----------------------------------------------------------------------
    // DRAFT-002: all exit paths reported
    // -----------------------------------------------------------------------

    #[test]
    fn multiple_exit_paths_all_reported() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![acquire("src/main.rs", 5, "main", "f", "File", "file")];
        let exits = vec![
            exit("src/main.rs", 10, "main", "normal"),
            exit("src/main.rs", 12, "main", "panic"),
            exit("src/main.rs", 14, "main", "early-return"),
        ];

        let (findings, _) = analyzer.analyze(&events, &exits, &[]);
        assert_eq!(findings.len(), 3, "each exit path should produce a leak");
        let kinds: Vec<&str> = findings
            .iter()
            .map(|f| {
                if f.exit_path.contains("panic") {
                    "panic"
                } else if f.exit_path.contains("early-return") {
                    "early-return"
                } else {
                    "normal"
                }
            })
            .collect();
        assert!(kinds.contains(&"panic"));
        assert!(kinds.contains(&"early-return"));
        assert!(kinds.contains(&"normal"));
    }

    // -----------------------------------------------------------------------
    // EDGE-001: acquire-release-reacquire
    // -----------------------------------------------------------------------

    #[test]
    fn reacquire_after_release_is_safe() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![
            acquire("src/main.rs", 10, "main", "f", "File", "file"),
            release("src/main.rs", 11, "main", "f"),
            acquire("src/main.rs", 12, "main", "f", "File", "file"),
            release("src/main.rs", 13, "main", "f"),
        ];
        let exits = vec![exit("src/main.rs", 14, "main", "normal")];

        let (findings, _) = analyzer.analyze(&events, &exits, &[]);
        assert!(findings.is_empty());
    }

    // -----------------------------------------------------------------------
    // identity:unknown diagnostics (REQ-T3)
    // -----------------------------------------------------------------------

    #[test]
    fn alias_without_clear_original_emits_identity_unknown() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![acquire("src/main.rs", 10, "main", "x", "File", "file")];
        let aliases = vec![alias("src/main.rs", 5, "main", "original", "x")];

        let (_findings, diagnostics) = analyzer.analyze(&events, &[], &aliases);
        assert!(!diagnostics.is_empty());
    }

    // -----------------------------------------------------------------------
    // Empty input
    // -----------------------------------------------------------------------

    #[test]
    fn no_events_is_safe() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let (findings, _) = analyzer.analyze(&[], &[], &[]);
        assert!(findings.is_empty());
    }

    // -----------------------------------------------------------------------
    // Multiple functions
    // -----------------------------------------------------------------------

    #[test]
    fn events_from_different_functions_are_independent() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![
            acquire("src/a.rs", 5, "fn_a", "f", "File", "file"),
            acquire("src/b.rs", 10, "fn_b", "g", "Mutex", "lock"),
            release("src/b.rs", 11, "fn_b", "g"),
        ];
        let exits = vec![
            exit("src/a.rs", 8, "fn_a", "normal"),
            exit("src/b.rs", 15, "fn_b", "normal"),
        ];

        let (findings, _) = analyzer.analyze(&events, &exits, &[]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].function, "fn_a");
    }

    // -----------------------------------------------------------------------
    // Resource type preserved
    // -----------------------------------------------------------------------

    #[test]
    fn finding_carries_resource_type() {
        let analyzer = ResourceLinearityAnalyzer::new();
        let events = vec![acquire(
            "src/main.rs",
            5,
            "main",
            "sock",
            "TcpStream",
            "socket",
        )];
        let exits = vec![exit("src/main.rs", 10, "main", "normal")];

        let (findings, _) = analyzer.analyze(&events, &exits, &[]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].resource_type, "TcpStream");
    }
}
