/// Provenance of an edge — how arguments flow from callee to caller.
///
/// Provenance tracks callee-to-caller argument flow through configured `H`
/// local-binding hops. Chains exceeding `H` hops are terminated explicitly
/// as `OverBound`, preserving the hops that were successfully traced.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provenance {
    /// Direct argument flow (no hops).
    Direct,
    /// Within the configured `H` hop limit.
    WithinH {
        /// Number of hops taken.
        hops: u32,
    },
    /// Chain exceeds the configured `H` hop limit.
    ///
    /// The hops that were successfully traced before termination are
    /// preserved so debugging and analysis tools can inspect the partial chain.
    OverBound {
        /// The maximum hops allowed.
        max_hops: u32,
        /// The number of hops actually traced before termination.
        actual: u32,
        /// Hops that were successfully traced within the bound.
        /// Empty if the chain was too long from the first hop.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        traced_hops: Vec<TracedHop>,
    },
}

/// A single traced hop in a provenance chain.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TracedHop {
    /// The hop index (1-indexed within the chain).
    pub index: u32,
    /// A description of where the hop occurred (e.g., variable name, binding site).
    pub location: String,
}

/// A source span in a file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceSpan {
    /// The file path (relative to project root).
    pub file: String,
    /// The start line (1-indexed).
    pub start_line: usize,
    /// The start column (1-indexed).
    pub start_column: usize,
    /// The end line (1-indexed).
    pub end_line: usize,
    /// The end column (1-indexed).
    pub end_column: usize,
}

/// A stable identity for a node or edge.
///
/// Identities are deterministic and stable across repeated extractions
/// on unchanged input. The identity scheme implemented by the reference
/// Rust frontend is:
///
/// `StableId = SHA256(name + ":" + path + ":" + line + ":" + column + ":" + content)`
///           truncated to 128 bits (16 bytes), hex-encoded
///
/// where `name` is the declaration/callee name, `path` is the relative file
/// path, `line`/`column` are the start location, and `content` is the source
/// text spanning the declaration or call site. This ensures that:
/// - Same source + same location → same ID (repeatable)
/// - Different source at the same location → different ID (content-sensitive)
/// - Same source at a different location → different ID (location-sensitive)
/// - Two call sites on the same line → different IDs (column-sensitive)
///
/// Frontends in other languages may choose a different scheme, but they must
/// preserve the three properties above. `StableId::new` itself is scheme-
/// agnostic; it only stores the producer-computed string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct StableId(String);

impl StableId {
    /// Create a new stable identity from a string.
    ///
    /// Frontends should pass the producer-computed identity (see the
    /// type-level docstring for the reference scheme). Consumers constructing
    /// fixtures may pass any opaque string.
    pub fn new(id: impl Into<String>) -> Self {
        StableId(id.into())
    }

    /// Get the identity string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for StableId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for StableId {
    fn from(s: &str) -> Self {
        StableId(s.to_string())
    }
}

impl From<String> for StableId {
    fn from(s: String) -> Self {
        StableId(s)
    }
}

/// A range of discarded source lines (exact discard span).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DiscardSpan {
    /// The file path.
    pub file: String,
    /// The start line (1-indexed).
    pub start_line: usize,
    /// The end line (1-indexed).
    pub end_line: usize,
}

/// Trust provenance classification for a CIR value occurrence.
///
/// Tracks whether a value originates from a declared trust boundary and
/// whether it has passed through a recognized refinement step.
///
/// The join order is `Untrusted > Unknown > Trusted`: any untrusted
/// contributor makes a derived occurrence untrusted; otherwise any unknown
/// contributor makes it unknown; otherwise it is trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrustProvenance {
    /// Value originates from a declared trust-boundary source.
    Untrusted,
    /// Value originates within the trust domain or has been refined.
    Trusted,
    /// No trust classification could be determined.
    Unknown,
}

impl TrustProvenance {
    /// Join two trust provenance values using the order
    /// `Untrusted > Unknown > Trusted`.
    ///
    /// Used when combining contributors to a derived value.
    pub fn join(self, other: TrustProvenance) -> TrustProvenance {
        use TrustProvenance::*;
        match (self, other) {
            (Untrusted, _) | (_, Untrusted) => Untrusted,
            (Unknown, _) | (_, Unknown) => Unknown,
            (Trusted, Trusted) => Trusted,
        }
    }

    /// Returns `true` if this is `Untrusted`.
    pub fn is_untrusted(self) -> bool {
        matches!(self, TrustProvenance::Untrusted)
    }

    /// Returns `true` if this is `Trusted`.
    pub fn is_trusted(self) -> bool {
        matches!(self, TrustProvenance::Trusted)
    }

    /// Returns `true` if this is `Unknown`.
    pub fn is_unknown(self) -> bool {
        matches!(self, TrustProvenance::Unknown)
    }
}

impl std::fmt::Display for TrustProvenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustProvenance::Untrusted => f.write_str("untrusted"),
            TrustProvenance::Trusted => f.write_str("trusted"),
            TrustProvenance::Unknown => f.write_str("unknown"),
        }
    }
}

/// Default trust provenance is `Trusted` (same-origin default).
impl Default for TrustProvenance {
    fn default() -> Self {
        TrustProvenance::Trusted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_direct() {
        assert_eq!(
            serde_json::to_value(Provenance::Direct).unwrap(),
            serde_json::json!("direct")
        );
    }

    #[test]
    fn provenance_within_h() {
        let p = Provenance::WithinH { hops: 2 };
        assert_eq!(
            serde_json::to_value(p).unwrap(),
            serde_json::json!({"within-h": {"hops": 2}})
        );
    }

    #[test]
    fn provenance_over_bound() {
        let p = Provenance::OverBound {
            max_hops: 3,
            actual: 5,
            traced_hops: vec![
                TracedHop {
                    index: 1,
                    location: "let x = callee()".into(),
                },
                TracedHop {
                    index: 2,
                    location: "let y = x.transform()".into(),
                },
                TracedHop {
                    index: 3,
                    location: "z = pass_to(y)".into(),
                },
            ],
        };
        let json = serde_json::to_value(p).unwrap();
        assert_eq!(json["over-bound"]["max_hops"], 3);
        assert_eq!(json["over-bound"]["actual"], 5);
        assert_eq!(
            json["over-bound"]["traced_hops"].as_array().unwrap().len(),
            3
        );
    }

    #[test]
    fn provenance_over_bound_no_traced_hops() {
        let p = Provenance::OverBound {
            max_hops: 3,
            actual: 5,
            traced_hops: vec![],
        };
        let json = serde_json::to_value(p).unwrap();
        assert_eq!(json["over-bound"]["max_hops"], 3);
        assert!(json["over-bound"].get("traced_hops").is_none());
    }

    #[test]
    fn stable_id_construction() {
        let id = StableId::new("node-42");
        assert_eq!(id.as_str(), "node-42");
        assert_eq!(id.to_string(), "node-42");
    }

    #[test]
    fn source_span_serialization() {
        let span = SourceSpan {
            file: "src/lib.rs".into(),
            start_line: 10,
            start_column: 1,
            end_line: 10,
            end_column: 42,
        };
        let json = serde_json::to_value(span).unwrap();
        assert_eq!(json["file"], "src/lib.rs");
        assert_eq!(json["start_line"], 10);
        assert_eq!(json["end_column"], 42);
    }

    #[test]
    fn discard_span_serialization() {
        let ds = DiscardSpan {
            file: "src/lib.rs".into(),
            start_line: 5,
            end_line: 8,
        };
        let json = serde_json::to_value(ds).unwrap();
        assert_eq!(json["file"], "src/lib.rs");
        assert_eq!(json["start_line"], 5);
        assert_eq!(json["end_line"], 8);
    }

    // --- TrustProvenance ---

    #[test]
    fn trust_provenance_variants() {
        assert_eq!(
            serde_json::to_value(TrustProvenance::Untrusted).unwrap(),
            serde_json::json!("untrusted")
        );
        assert_eq!(
            serde_json::to_value(TrustProvenance::Trusted).unwrap(),
            serde_json::json!("trusted")
        );
        assert_eq!(
            serde_json::to_value(TrustProvenance::Unknown).unwrap(),
            serde_json::json!("unknown")
        );
    }

    #[test]
    fn trust_provenance_join_untrusted_dominates() {
        use TrustProvenance::*;
        // Untrusted + anything = Untrusted
        assert_eq!(Untrusted.join(Trusted), Untrusted);
        assert_eq!(Untrusted.join(Unknown), Untrusted);
        assert_eq!(Untrusted.join(Untrusted), Untrusted);
        assert_eq!(Trusted.join(Untrusted), Untrusted);
        assert_eq!(Unknown.join(Untrusted), Untrusted);
    }

    #[test]
    fn trust_provenance_join_unknown_second() {
        use TrustProvenance::*;
        assert_eq!(Unknown.join(Trusted), Unknown);
        assert_eq!(Trusted.join(Unknown), Unknown);
        assert_eq!(Unknown.join(Unknown), Unknown);
    }

    #[test]
    fn trust_provenance_join_trusted_only() {
        use TrustProvenance::*;
        assert_eq!(Trusted.join(Trusted), Trusted);
    }

    #[test]
    fn trust_provenance_join_truth_table() {
        use TrustProvenance::*;
        let cases = [
            (Untrusted, Untrusted, Untrusted),
            (Untrusted, Trusted, Untrusted),
            (Untrusted, Unknown, Untrusted),
            (Trusted, Untrusted, Untrusted),
            (Trusted, Trusted, Trusted),
            (Trusted, Unknown, Unknown),
            (Unknown, Untrusted, Untrusted),
            (Unknown, Trusted, Unknown),
            (Unknown, Unknown, Unknown),
        ];
        for (a, b, expected) in &cases {
            assert_eq!(
                a.join(*b),
                *expected,
                "{a:?} join {b:?} should be {expected:?}"
            );
        }
    }

    #[test]
    fn trust_provenance_join_is_commutative() {
        use TrustProvenance::*;
        let all = [Untrusted, Trusted, Unknown];
        for a in &all {
            for b in &all {
                assert_eq!(
                    a.join(*b),
                    b.join(*a),
                    "{a:?} join {b:?} should be commutative"
                );
            }
        }
    }

    #[test]
    fn trust_provenance_display() {
        assert_eq!(TrustProvenance::Untrusted.to_string(), "untrusted");
        assert_eq!(TrustProvenance::Trusted.to_string(), "trusted");
        assert_eq!(TrustProvenance::Unknown.to_string(), "unknown");
    }

    #[test]
    fn trust_provenance_predicates() {
        assert!(TrustProvenance::Untrusted.is_untrusted());
        assert!(!TrustProvenance::Untrusted.is_trusted());
        assert!(!TrustProvenance::Untrusted.is_unknown());
        assert!(TrustProvenance::Trusted.is_trusted());
        assert!(TrustProvenance::Unknown.is_unknown());
    }

    #[test]
    fn trust_provenance_serde_round_trip() {
        let cases = [
            TrustProvenance::Untrusted,
            TrustProvenance::Trusted,
            TrustProvenance::Unknown,
        ];
        for tp in &cases {
            let json = serde_json::to_string(tp).unwrap();
            let deser: TrustProvenance = serde_json::from_str(&json).unwrap();
            assert_eq!(*tp, deser);
        }
    }

    #[test]
    fn traced_hop_serialization() {
        let hop = TracedHop {
            index: 1,
            location: "x = foo()".into(),
        };
        let json = serde_json::to_value(hop).unwrap();
        assert_eq!(json["index"], 1);
        assert_eq!(json["location"], "x = foo()");
    }
}
