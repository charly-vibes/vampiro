//! Refinement-confirmation evidence import (REQ-B5).
//!
//! Imports versioned boundary-coverage evidence from a JSON file and
//! correlates it against the analyzed revision and constructor identity/hash.
//! Produces `refinement_confirmation` status: `confirmed` or `unknown` with
//! a closed primary reason vocabulary.
//!
//! Evidence schema (v0.1.0):
//!
//! ```json
//! {
//!   "schema_version": "v0.1.0",
//!   "producer": { "name": "<tool>", "version": "<semver>" },
//!   "analyzed_revision": "<git-sha>",
//!   "constructor": {
//!     "stable_identity": "<id>",
//!     "source_hash": "<sha256-hex>",
//!     "shape_hash": "<sha256-hex>"
//!   },
//!   "boundary_classes": [
//!     { "id": "<class-id>", "status": "passing|failing", "details": "<text>" }
//!   ]
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// The current supported evidence schema version.
pub const EVIDENCE_SCHEMA_VERSION: &str = "v0.1.0";

/// A single boundary-class result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundaryClassResult {
    /// The boundary-class ID (must match a project declaration).
    pub id: String,
    /// Whether this class passed or failed.
    pub status: String,
    /// Optional human-readable details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// The imported evidence payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidencePayload {
    /// The evidence schema version.
    pub schema_version: String,
    /// The companion tool that produced this evidence.
    pub producer: EvidenceProducer,
    /// The analyzed git revision (commit SHA).
    pub analyzed_revision: String,
    /// The smart constructor identity and content hashes.
    pub constructor: EvidenceConstructor,
    /// Per-class coverage results.
    pub boundary_classes: Vec<BoundaryClassResult>,
}

/// The companion tool that produced the evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceProducer {
    /// Tool name (e.g., `"vampiro-coverage"`).
    pub name: String,
    /// Tool version (semver).
    pub version: String,
}

/// Smart constructor identity and hashes for correlation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceConstructor {
    /// The constructor's stable identity.
    pub stable_identity: String,
    /// SHA-256 hex hash of the constructor source.
    pub source_hash: String,
    /// SHA-256 hex hash of the constructor's refined shape.
    pub shape_hash: String,
}

/// The result of importing and correlating evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefinementConfirmation {
    /// Whether refinement is confirmed or unknown.
    pub status: RefinementStatus,
    /// The primary reason when status is `Unknown`.
    pub reason: Option<RefinementReason>,
}

/// Whether refinement is confirmed by evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefinementStatus {
    /// Every declared boundary class has current, complete passing evidence.
    Confirmed,
    /// Evidence is absent, malformed, mismatched, or incomplete.
    Unknown,
}

/// The ordered closed vocabulary for `unknown` reasons.
///
/// The first applicable reason in this order is used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefinementReason {
    /// No evidence file found at the configured path.
    Absent,
    /// Evidence file exists but cannot be parsed.
    Malformed,
    /// Evidence schema version is not recognized.
    UnsupportedVersion,
    /// `analyzed_revision` does not match the current analyzed commit.
    Stale,
    /// Constructor identity, source_hash, or shape_hash does not match.
    Mismatched,
    /// `boundary_classes` is empty.
    EmptyClasses,
    /// A boundary-class ID appears more than once.
    DuplicateClass,
    /// Not every declared boundary class appears in evidence.
    Incomplete,
    /// An evidence boundary-class ID has no matching project declaration.
    UnknownClass,
    /// Any boundary class has `status=failing`.
    Failing,
}

impl std::fmt::Display for RefinementReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefinementReason::Absent => f.write_str("absent"),
            RefinementReason::Malformed => f.write_str("malformed"),
            RefinementReason::UnsupportedVersion => f.write_str("unsupported-version"),
            RefinementReason::Stale => f.write_str("stale"),
            RefinementReason::Mismatched => f.write_str("mismatched"),
            RefinementReason::EmptyClasses => f.write_str("empty-classes"),
            RefinementReason::DuplicateClass => f.write_str("duplicate-class"),
            RefinementReason::Incomplete => f.write_str("incomplete"),
            RefinementReason::UnknownClass => f.write_str("unknown-class"),
            RefinementReason::Failing => f.write_str("failing"),
        }
    }
}

/// Import and correlate evidence against the analyzed revision and constructor.
///
/// # Arguments
///
/// * `evidence_json` - Optional JSON string with the evidence payload.
///   `None` means the evidence file was absent.
/// * `current_revision` - The analyzed git commit SHA.
/// * `current_constructor_id` - The smart constructor's stable identity.
/// * `current_source_hash` - SHA-256 hex hash of the constructor source.
/// * `current_shape_hash` - SHA-256 hex hash of the constructor's refined shape.
/// * `declared_classes` - The set of declared boundary-class IDs.
///
/// # Returns
///
/// A `RefinementConfirmation` with status and reason.
pub fn correlate_evidence(
    evidence_json: Option<&str>,
    current_revision: &str,
    current_constructor_id: &str,
    current_source_hash: &str,
    current_shape_hash: &str,
    declared_classes: &HashSet<String>,
) -> RefinementConfirmation {
    // 1. Absent
    let evidence = match evidence_json {
        Some(json) => json,
        None => {
            return RefinementConfirmation {
                status: RefinementStatus::Unknown,
                reason: Some(RefinementReason::Absent),
            }
        }
    };

    // 2. Malformed
    let payload: EvidencePayload = match serde_json::from_str(evidence) {
        Ok(p) => p,
        Err(_) => {
            return RefinementConfirmation {
                status: RefinementStatus::Unknown,
                reason: Some(RefinementReason::Malformed),
            }
        }
    };

    // 3. Unsupported version
    if payload.schema_version != EVIDENCE_SCHEMA_VERSION {
        return RefinementConfirmation {
            status: RefinementStatus::Unknown,
            reason: Some(RefinementReason::UnsupportedVersion),
        };
    }

    // 4. Stale revision
    if payload.analyzed_revision != current_revision {
        return RefinementConfirmation {
            status: RefinementStatus::Unknown,
            reason: Some(RefinementReason::Stale),
        };
    }

    // 5. Mismatched constructor
    if payload.constructor.stable_identity != current_constructor_id
        || payload.constructor.source_hash != current_source_hash
        || payload.constructor.shape_hash != current_shape_hash
    {
        return RefinementConfirmation {
            status: RefinementStatus::Unknown,
            reason: Some(RefinementReason::Mismatched),
        };
    }

    // 6. Empty classes
    if payload.boundary_classes.is_empty() {
        return RefinementConfirmation {
            status: RefinementStatus::Unknown,
            reason: Some(RefinementReason::EmptyClasses),
        };
    }

    // 7. Duplicate class IDs in evidence
    let mut seen_ids = HashSet::new();
    for bc in &payload.boundary_classes {
        if !seen_ids.insert(&bc.id) {
            return RefinementConfirmation {
                status: RefinementStatus::Unknown,
                reason: Some(RefinementReason::DuplicateClass),
            };
        }
    }

    // 8. Incomplete: not every declared class appears in evidence
    let evidence_ids: HashSet<&str> = payload
        .boundary_classes
        .iter()
        .map(|bc| bc.id.as_str())
        .collect();
    let declared: HashSet<&str> = declared_classes.iter().map(|s| s.as_str()).collect();
    if !declared.is_subset(&evidence_ids) {
        return RefinementConfirmation {
            status: RefinementStatus::Unknown,
            reason: Some(RefinementReason::Incomplete),
        };
    }

    // 9. Unknown class in evidence (not declared)
    for bc in &payload.boundary_classes {
        if !declared.contains(bc.id.as_str()) {
            return RefinementConfirmation {
                status: RefinementStatus::Unknown,
                reason: Some(RefinementReason::UnknownClass),
            };
        }
    }

    // 10. Any failing class
    for bc in &payload.boundary_classes {
        if bc.status == "failing" {
            return RefinementConfirmation {
                status: RefinementStatus::Unknown,
                reason: Some(RefinementReason::Failing),
            };
        }
    }

    // All checks passed — confirmed
    RefinementConfirmation {
        status: RefinementStatus::Confirmed,
        reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_evidence_json(
        schema_version: &str,
        revision: &str,
        constructor_id: &str,
        source_hash: &str,
        shape_hash: &str,
        classes: Vec<(&str, &str, Option<&str>)>,
    ) -> String {
        let classes_json: Vec<serde_json::Value> = classes
            .into_iter()
            .map(|(id, status, details)| {
                let mut obj = serde_json::json!({
                    "id": id,
                    "status": status,
                });
                if let Some(d) = details {
                    obj["details"] = serde_json::json!(d);
                }
                obj
            })
            .collect();

        serde_json::json!({
            "schema_version": schema_version,
            "producer": { "name": "test-producer", "version": "0.1.0" },
            "analyzed_revision": revision,
            "constructor": {
                "stable_identity": constructor_id,
                "source_hash": source_hash,
                "shape_hash": shape_hash
            },
            "boundary_classes": classes_json
        })
        .to_string()
    }

    fn make_declared(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    // --- Positive case ---

    #[test]
    fn all_checks_pass_returns_confirmed() {
        let json = make_evidence_json(
            "v0.1.0",
            "abc123",
            "User::new",
            "src_hash_abc",
            "shape_hash_xyz",
            vec![
                ("req-body", "passing", None),
                ("req-headers", "passing", None),
            ],
        );
        let result = correlate_evidence(
            Some(&json),
            "abc123",
            "User::new",
            "src_hash_abc",
            "shape_hash_xyz",
            &make_declared(&["req-body", "req-headers"]),
        );
        assert_eq!(result.status, RefinementStatus::Confirmed);
        assert!(result.reason.is_none());
    }

    // --- Each unknown reason ---

    #[test]
    fn absent_evidence_returns_unknown_absent() {
        let result = correlate_evidence(
            None,
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            &make_declared(&["cls"]),
        );
        assert_eq!(result.status, RefinementStatus::Unknown);
        assert_eq!(result.reason, Some(RefinementReason::Absent));
    }

    #[test]
    fn malformed_json_returns_unknown_malformed() {
        let result = correlate_evidence(
            Some("not valid json"),
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            &make_declared(&["cls"]),
        );
        assert_eq!(result.status, RefinementStatus::Unknown);
        assert_eq!(result.reason, Some(RefinementReason::Malformed));
    }

    #[test]
    fn unsupported_version_returns_unknown() {
        let json = make_evidence_json(
            "v0.2.0", // unsupported
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            vec![("cls", "passing", None)],
        );
        let result = correlate_evidence(
            Some(&json),
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            &make_declared(&["cls"]),
        );
        assert_eq!(result.status, RefinementStatus::Unknown);
        assert_eq!(result.reason, Some(RefinementReason::UnsupportedVersion));
    }

    #[test]
    fn stale_revision_returns_unknown() {
        let json = make_evidence_json(
            "v0.1.0",
            "old_revision", // stale
            "User::new",
            "src_hash",
            "shape_hash",
            vec![("cls", "passing", None)],
        );
        let result = correlate_evidence(
            Some(&json),
            "current_revision",
            "User::new",
            "src_hash",
            "shape_hash",
            &make_declared(&["cls"]),
        );
        assert_eq!(result.status, RefinementStatus::Unknown);
        assert_eq!(result.reason, Some(RefinementReason::Stale));
    }

    #[test]
    fn mismatched_constructor_identity_returns_unknown() {
        let json = make_evidence_json(
            "v0.1.0",
            "abc123",
            "Other::new", // different identity
            "src_hash",
            "shape_hash",
            vec![("cls", "passing", None)],
        );
        let result = correlate_evidence(
            Some(&json),
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            &make_declared(&["cls"]),
        );
        assert_eq!(result.status, RefinementStatus::Unknown);
        assert_eq!(result.reason, Some(RefinementReason::Mismatched));
    }

    #[test]
    fn mismatched_source_hash_returns_unknown() {
        let json = make_evidence_json(
            "v0.1.0",
            "abc123",
            "User::new",
            "different_hash",
            "shape_hash",
            vec![("cls", "passing", None)],
        );
        let result = correlate_evidence(
            Some(&json),
            "abc123",
            "User::new",
            "expected_hash",
            "shape_hash",
            &make_declared(&["cls"]),
        );
        assert_eq!(result.status, RefinementStatus::Unknown);
        assert_eq!(result.reason, Some(RefinementReason::Mismatched));
    }

    #[test]
    fn empty_classes_returns_unknown() {
        let json = make_evidence_json(
            "v0.1.0",
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            vec![],
        );
        let result = correlate_evidence(
            Some(&json),
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            &make_declared(&["cls"]),
        );
        assert_eq!(result.status, RefinementStatus::Unknown);
        assert_eq!(result.reason, Some(RefinementReason::EmptyClasses));
    }

    #[test]
    fn duplicate_class_ids_returns_unknown() {
        let json = make_evidence_json(
            "v0.1.0",
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            vec![
                ("cls", "passing", None),
                ("cls", "passing", None), // duplicate
            ],
        );
        let result = correlate_evidence(
            Some(&json),
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            &make_declared(&["cls"]),
        );
        assert_eq!(result.status, RefinementStatus::Unknown);
        assert_eq!(result.reason, Some(RefinementReason::DuplicateClass));
    }

    #[test]
    fn incomplete_classes_returns_unknown() {
        let json = make_evidence_json(
            "v0.1.0",
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            vec![("req-body", "passing", None)], // missing req-headers
        );
        let result = correlate_evidence(
            Some(&json),
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            &make_declared(&["req-body", "req-headers"]),
        );
        assert_eq!(result.status, RefinementStatus::Unknown);
        assert_eq!(result.reason, Some(RefinementReason::Incomplete));
    }

    #[test]
    fn unknown_class_in_evidence_returns_unknown() {
        // All declared classes are present, but there's an extra undeclared class
        let json = make_evidence_json(
            "v0.1.0",
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            vec![
                ("req-body", "passing", None),
                ("undeclared", "passing", None), // extra, not declared
            ],
        );
        let result = correlate_evidence(
            Some(&json),
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            &make_declared(&["req-body"]),
        );
        assert_eq!(result.status, RefinementStatus::Unknown);
        assert_eq!(result.reason, Some(RefinementReason::UnknownClass));
    }

    #[test]
    fn failing_class_returns_unknown() {
        let json = make_evidence_json(
            "v0.1.0",
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            vec![("req-body", "failing", Some("timeout error"))],
        );
        let result = correlate_evidence(
            Some(&json),
            "abc123",
            "User::new",
            "src_hash",
            "shape_hash",
            &make_declared(&["req-body"]),
        );
        assert_eq!(result.status, RefinementStatus::Unknown);
        assert_eq!(result.reason, Some(RefinementReason::Failing));
    }

    // --- Priority order: first applicable reason wins ---

    #[test]
    fn first_applicable_reason_wins_absent_before_malformed() {
        // Absent is checked first
        let result = correlate_evidence(None, "abc", "Ctor", "s", "h", &make_declared(&["cls"]));
        assert_eq!(result.reason, Some(RefinementReason::Absent));
    }

    #[test]
    fn malformed_before_unsupported_version() {
        let result = correlate_evidence(
            Some("not json"),
            "abc",
            "Ctor",
            "s",
            "h",
            &make_declared(&["cls"]),
        );
        assert_eq!(result.reason, Some(RefinementReason::Malformed));
    }

    #[test]
    fn evidenced_field_format_serialization() {
        // Verify the serde field names match the expected schema
        let payload = EvidencePayload {
            schema_version: "v0.1.0".into(),
            producer: EvidenceProducer {
                name: "vampiro-coverage".into(),
                version: "0.1.0".into(),
            },
            analyzed_revision: "abc123".into(),
            constructor: EvidenceConstructor {
                stable_identity: "User::new".into(),
                source_hash: "sha256hex".into(),
                shape_hash: "sha256hex".into(),
            },
            boundary_classes: vec![BoundaryClassResult {
                id: "req-body".into(),
                status: "passing".into(),
                details: None,
            }],
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["schema_version"], "v0.1.0");
        assert_eq!(json["producer"]["name"], "vampiro-coverage");
        assert_eq!(json["analyzed_revision"], "abc123");
        assert_eq!(json["constructor"]["stable_identity"], "User::new");
        assert_eq!(json["boundary_classes"][0]["id"], "req-body");
        assert_eq!(json["boundary_classes"][0]["status"], "passing");
    }
}
