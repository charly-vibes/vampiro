//! Obligation and evidence contract types for Vampiro law verification.
//!
//! This crate defines the versioned, backend-neutral types that flow through
//! the law-verification pipeline: from declared theories → obligation IR →
//! runner inputs → evidence (property + proof).

pub mod aggregation;
pub mod prover;
pub mod runner;

// All types are `Serialize`/`Deserialize` for cross-version contract testing.
// The schema version is `0.1.0` for the initial contract milestone.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Theories
// ---------------------------------------------------------------------------

/// A declared law theory (e.g. "monoid", "commutative", custom suite name).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Theory {
    /// The theory name (e.g. "monoid", "commutative", "my-custom-suite").
    pub name: String,
    /// Whether this suite replaces or augments a built-in.
    #[serde(default)]
    pub kind: TheoryKind,
}

/// Whether a theory suite replaces or augments a built-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TheoryKind {
    /// Replaces the built-in theory entirely.
    Replacing,
    /// Augments (adds to) the built-in theory.
    #[default]
    Augmenting,
}

// ---------------------------------------------------------------------------
// Clusters
// ---------------------------------------------------------------------------

/// Identifies a single cluster member (implementation) for law verification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClusterMember {
    /// Unique identifier within the cluster.
    pub id: String,
    /// The Rust module path or function name.
    pub path: String,
    /// Tags for this member (e.g. "law:commutative", "proof:lean").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// A cluster of implementations that share a declared signature.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImplementationCluster {
    /// Cluster identifier.
    pub id: String,
    /// The declared signature name.
    pub signature: String,
    /// Members of this cluster.
    pub members: Vec<ClusterMember>,
    /// Theories that apply to this cluster.
    pub theories: Vec<Theory>,
}

// ---------------------------------------------------------------------------
// Obligation IR (backend-neutral)
// ---------------------------------------------------------------------------

/// A backend-neutral obligation — a single law to verify for one member.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Obligation {
    /// Schema version for forward compatibility.
    pub schema_version: String,
    /// The obligation identifier.
    pub id: String,
    /// The implementation cluster member.
    pub member: ClusterMember,
    /// The theory this obligation belongs to.
    pub theory: Theory,
    /// The law expression or equation identifier.
    pub law: String,
    /// Generator configuration (for property testing).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator_config: Option<GeneratorConfig>,
    /// Whether a prover is requested for this obligation.
    #[serde(default)]
    pub prover_requested: bool,
    /// Which prover to use when `prover_requested` is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prover: Option<String>,
    /// Tags forwarded from the member.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Configuration for property-test generators.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GeneratorConfig {
    /// Number of test cases to generate.
    #[serde(default = "default_cases")]
    pub cases: u32,
    /// Seed for deterministic runs.
    #[serde(default)]
    pub seed: Option<u64>,
}

fn default_cases() -> u32 {
    1000
}

impl GeneratorConfig {
    /// Default generator configuration.
    pub fn default_with_seed(seed: u64) -> Self {
        GeneratorConfig {
            cases: default_cases(),
            seed: Some(seed),
        }
    }
}

// ---------------------------------------------------------------------------
// Evidence (property + proof)
// ---------------------------------------------------------------------------

/// The status of a law-verification check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceStatus {
    /// The obligation was satisfied.
    Passed,
    /// The obligation was violated (counterexample found).
    Failed,
    /// The check did not complete (e.g. timeout).
    Inconclusive,
    /// The runner or prover could not execute.
    Error,
}

impl std::fmt::Display for EvidenceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            EvidenceStatus::Passed => "passed",
            EvidenceStatus::Failed => "failed",
            EvidenceStatus::Inconclusive => "inconclusive",
            EvidenceStatus::Error => "error",
        })
    }
}

impl std::str::FromStr for EvidenceStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "passed" => Ok(EvidenceStatus::Passed),
            "failed" => Ok(EvidenceStatus::Failed),
            "inconclusive" => Ok(EvidenceStatus::Inconclusive),
            "error" => Ok(EvidenceStatus::Error),
            _ => Err(format!("unknown evidence status: {s}")),
        }
    }
}

/// Prover-specific result status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProverStatus {
    /// The prover proved the obligation.
    Proved,
    /// The prover found a counterexample.
    Disproved,
    /// The prover did not complete within timeout.
    Timeout,
    /// The prover tool was unavailable or errored.
    ProverUnavailable,
}

impl std::fmt::Display for ProverStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ProverStatus::Proved => "proved",
            ProverStatus::Disproved => "disproved",
            ProverStatus::Timeout => "timeout",
            ProverStatus::ProverUnavailable => "unavailable",
        })
    }
}

/// Evidence from a single verification channel (property test or prover).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    /// Schema version for forward compatibility.
    pub schema_version: String,
    /// The obligation this evidence is for.
    pub obligation_id: String,
    /// The verification channel.
    pub channel: EvidenceChannel,
    /// The status of the check.
    pub status: EvidenceStatus,
    /// Trace / counterexample details (free-form).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Prover-specific result, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prover_result: Option<ProverStatus>,
    /// Round-trip version tag for contract testing.
    pub version: String,
}

/// The verification channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceChannel {
    /// Property-testing (proptest).
    Property,
    /// Formal proof (Lean, Dafny, TLA+).
    Proof,
}

/// Combined evidence from both property and proof channels for a single
/// obligation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombinedEvidence {
    /// Schema version.
    pub schema_version: String,
    /// The obligation this evidence is for.
    pub obligation_id: String,
    /// Property-test evidence (always present when property testing is enabled).
    pub property_evidence: Option<Evidence>,
    /// Proof evidence (present only when a prover was configured and ran).
    pub proof_evidence: Option<Evidence>,
    /// Lifecycle cross-reference (for idempotency tracking).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle_ref: Option<String>,
}

// ---------------------------------------------------------------------------
// Runner input (consumed by language runners)
// ---------------------------------------------------------------------------

/// Input to a language runner for law verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunnerInput {
    /// Schema version.
    pub schema_version: String,
    /// Language identifier (e.g. "rust").
    pub language: String,
    /// The obligations to verify.
    pub obligations: Vec<Obligation>,
    /// Serialized values for property testing (JSON blob).
    ///
    /// Each entry maps a variable name to its generated value.
    /// Example: `{"a": 42, "b": 7}`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Contract test helpers
// ---------------------------------------------------------------------------

/// Current contract version for this milestone.
pub const CONTRACT_VERSION: &str = "0.1.0";

/// Create a minimal example obligation for contract testing.
pub fn example_obligation() -> Obligation {
    Obligation {
        schema_version: CONTRACT_VERSION.to_string(),
        id: "test-obligation-1".to_string(),
        member: ClusterMember {
            id: "member-1".to_string(),
            path: "my_crate::my_module::my_fn".to_string(),
            tags: vec!["law:commutative".to_string()],
        },
        theory: Theory {
            name: "commutative".to_string(),
            kind: TheoryKind::Augmenting,
        },
        law: "a + b = b + a".to_string(),
        generator_config: Some(GeneratorConfig::default_with_seed(42)),
        prover_requested: false,
        prover: None,
        tags: vec![],
    }
}

/// Create a minimal example evidence for contract testing.
pub fn example_property_evidence() -> Evidence {
    Evidence {
        schema_version: CONTRACT_VERSION.to_string(),
        obligation_id: "test-obligation-1".to_string(),
        channel: EvidenceChannel::Property,
        status: EvidenceStatus::Passed,
        detail: None,
        prover_result: None,
        version: CONTRACT_VERSION.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Round-trip tests ---

    #[test]
    fn obligation_round_trip_json() {
        let ob = example_obligation();
        let json = serde_json::to_string(&ob).unwrap();
        let back: Obligation = serde_json::from_str(&json).unwrap();
        assert_eq!(ob, back);
    }

    #[test]
    fn evidence_round_trip_json() {
        let ev = example_property_evidence();
        let json = serde_json::to_string(&ev).unwrap();
        let back: Evidence = serde_json::from_str(&json).unwrap();
        assert_eq!(ev, back);
    }

    #[test]
    fn combined_evidence_round_trip() {
        let ce = CombinedEvidence {
            schema_version: CONTRACT_VERSION.to_string(),
            obligation_id: "test-1".to_string(),
            property_evidence: Some(example_property_evidence()),
            proof_evidence: Some(Evidence {
                schema_version: CONTRACT_VERSION.to_string(),
                obligation_id: "test-1".to_string(),
                channel: EvidenceChannel::Proof,
                status: EvidenceStatus::Passed,
                detail: Some("proved by Lean".to_string()),
                prover_result: Some(ProverStatus::Proved),
                version: CONTRACT_VERSION.to_string(),
            }),
            lifecycle_ref: Some("check-abc".to_string()),
        };
        let json = serde_json::to_string(&ce).unwrap();
        let back: CombinedEvidence = serde_json::from_str(&json).unwrap();
        assert_eq!(ce, back);
    }

    #[test]
    fn runner_input_round_trip() {
        let input = RunnerInput {
            schema_version: CONTRACT_VERSION.to_string(),
            language: "rust".to_string(),
            obligations: vec![example_obligation()],
            values: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        let back: RunnerInput = serde_json::from_str(&json).unwrap();
        assert_eq!(input, back);
    }

    // --- Evidence status tests ---

    #[test]
    fn evidence_status_serialization() {
        for (status, expected) in [
            (EvidenceStatus::Passed, "\"passed\""),
            (EvidenceStatus::Failed, "\"failed\""),
            (EvidenceStatus::Inconclusive, "\"inconclusive\""),
            (EvidenceStatus::Error, "\"error\""),
        ] {
            assert_eq!(serde_json::to_string(&status).unwrap(), expected);
        }
    }

    #[test]
    fn evidence_status_from_str() {
        assert_eq!(
            "passed".parse::<EvidenceStatus>().unwrap(),
            EvidenceStatus::Passed
        );
        assert_eq!(
            "failed".parse::<EvidenceStatus>().unwrap(),
            EvidenceStatus::Failed
        );
        assert!("unknown".parse::<EvidenceStatus>().is_err());
    }

    #[test]
    fn prover_status_display() {
        assert_eq!(ProverStatus::Proved.to_string(), "proved");
        assert_eq!(ProverStatus::Disproved.to_string(), "disproved");
        assert_eq!(ProverStatus::Timeout.to_string(), "timeout");
        assert_eq!(ProverStatus::ProverUnavailable.to_string(), "unavailable");
    }

    // --- Theory tests ---

    #[test]
    fn theory_default_is_augmenting() {
        let t = Theory {
            name: "test".to_string(),
            kind: TheoryKind::default(),
        };
        assert_eq!(t.kind, TheoryKind::Augmenting);
    }

    #[test]
    fn theory_serialization() {
        let replacing = Theory {
            name: "custom".to_string(),
            kind: TheoryKind::Replacing,
        };
        let json = serde_json::to_string(&replacing).unwrap();
        assert!(json.contains("replacing"));
    }

    // --- Cluster member tests ---

    #[test]
    fn cluster_member_omits_empty_tags() {
        let member = ClusterMember {
            id: "m1".to_string(),
            path: "path".to_string(),
            tags: vec![],
        };
        let json = serde_json::to_string(&member).unwrap();
        assert!(!json.contains("tags"), "empty tags should be omitted");
    }

    #[test]
    fn cluster_member_with_tags() {
        let member = ClusterMember {
            id: "m1".to_string(),
            path: "path".to_string(),
            tags: vec!["law:commutative".to_string(), "proof:lean".to_string()],
        };
        let json = serde_json::to_string(&member).unwrap();
        assert!(json.contains("law:commutative"));
        assert!(json.contains("proof:lean"));
    }

    // --- Schema version tests ---

    #[test]
    fn obligation_has_schema_version() {
        let ob = example_obligation();
        assert_eq!(ob.schema_version, CONTRACT_VERSION);
    }

    #[test]
    fn evidence_has_version() {
        let ev = example_property_evidence();
        assert_eq!(ev.version, CONTRACT_VERSION);
    }

    // --- Lifecycle cross-reference test ---

    #[test]
    fn combined_evidence_with_lifecycle_ref() {
        let ce = CombinedEvidence {
            schema_version: CONTRACT_VERSION.to_string(),
            obligation_id: "test-idempotency".to_string(),
            property_evidence: Some(example_property_evidence()),
            proof_evidence: None,
            lifecycle_ref: Some("check-xyz".to_string()),
        };
        let json = serde_json::to_string(&ce).unwrap();
        assert!(
            json.contains("check-xyz"),
            "lifecycle ref should be in JSON"
        );
    }
}
