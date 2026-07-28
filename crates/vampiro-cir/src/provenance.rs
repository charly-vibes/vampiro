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
/// on unchanged input. The identity scheme is:
///
/// `StableId = SHA256(content_hash + ":" + path + ":" + line) truncated to 128 bits`
///
/// where `content_hash` is a hash of the source content at the declaration/call
/// site, `path` is the relative file path, and `line` is the start line.
/// This ensures that:
/// - Same source + same location → same ID (repeatable)
/// - Different source at same location → different ID (content-sensitive)
/// - Same source at different location → different ID (location-sensitive)
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct StableId(String);

impl StableId {
    /// Create a new stable identity from a string.
    ///
    /// In production, the string should be derived from
    /// `SHA256(content_hash:path:line)` as documented.
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
