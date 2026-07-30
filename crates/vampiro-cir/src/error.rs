/// Errors that can occur during CIR construction, validation, or extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CirError {
    /// An edge references a source or target node that does not exist in the graph.
    MissingNode {
        edge_id: String,
        node_id: String,
        role: NodeRole,
    },
    /// Two nodes share the same stable identity.
    DuplicateNode { id: String },
    /// An effect channel exceeds the maximum allowed nesting depth.
    EffectDepthExceeded { max_depth: u32, observed: u32 },
    /// A shape exceeds the maximum allowed nesting depth.
    ShapeDepthExceeded { max_depth: u32, observed: u32 },
    /// An expression node violates the invariant (domain != codomain or
    /// effect != Plain).
    ExpressionInvariant { node_id: String, detail: String },
    /// An expression node's `containing_function` references a node that
    /// does not exist or is not a Declaration.
    OrphanedExpression {
        node_id: String,
        containing_id: String,
    },
    /// Deserialization of CIR data failed.
    Deserialization(String),
    /// Extraction by a frontend failed.
    Extraction(String),
}

/// Whether the missing node is the source or target of an edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeRole {
    Source,
    Target,
}

impl std::fmt::Display for CirError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CirError::MissingNode {
                edge_id,
                node_id,
                role,
            } => {
                let role_str = match role {
                    NodeRole::Source => "source",
                    NodeRole::Target => "target",
                };
                write!(
                    f,
                    "edge `{edge_id}` references missing {role_str} node `{node_id}`"
                )
            }
            CirError::DuplicateNode { id } => {
                write!(f, "duplicate stable identity `{id}`")
            }
            CirError::EffectDepthExceeded {
                max_depth,
                observed,
            } => {
                write!(
                    f,
                    "effect channel depth {observed} exceeds maximum {max_depth}"
                )
            }
            CirError::ShapeDepthExceeded {
                max_depth,
                observed,
            } => {
                write!(f, "shape depth {observed} exceeds maximum {max_depth}")
            }
            CirError::ExpressionInvariant { node_id, detail } => {
                write!(
                    f,
                    "expression node `{node_id}` violates invariant: {detail}"
                )
            }
            CirError::OrphanedExpression {
                node_id,
                containing_id,
            } => {
                write!(
                    f,
                    "expression node `{node_id}` references missing or non-declaration node `{containing_id}`"
                )
            }
            CirError::Deserialization(msg) => {
                write!(f, "CIR deserialization error: {msg}")
            }
            CirError::Extraction(msg) => {
                write!(f, "CIR extraction error: {msg}")
            }
        }
    }
}

impl std::error::Error for CirError {}

/// Convenience conversion: treats an ad-hoc `String` message as an extraction
/// error. Use `CirError::Deserialization` explicitly when the error originated
/// from a deserializer to preserve that classification.
impl From<String> for CirError {
    fn from(msg: String) -> Self {
        CirError::Extraction(msg)
    }
}

/// Convenience conversion: treats an ad-hoc `&str` message as an extraction
/// error. See the `From<String>` impl for classification caveats.
impl From<&str> for CirError {
    fn from(msg: &str) -> Self {
        CirError::Extraction(msg.to_string())
    }
}

impl From<serde_json::Error> for CirError {
    fn from(e: serde_json::Error) -> Self {
        CirError::Deserialization(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cir_error_missing_node_display() {
        let err = CirError::MissingNode {
            edge_id: "e1".into(),
            node_id: "n42".into(),
            role: NodeRole::Source,
        };
        assert_eq!(
            err.to_string(),
            "edge `e1` references missing source node `n42`"
        );
    }

    #[test]
    fn cir_error_target_node_display() {
        let err = CirError::MissingNode {
            edge_id: "e2".into(),
            node_id: "n99".into(),
            role: NodeRole::Target,
        };
        assert_eq!(
            err.to_string(),
            "edge `e2` references missing target node `n99`"
        );
    }

    #[test]
    fn cir_error_depth_display() {
        let err = CirError::EffectDepthExceeded {
            max_depth: 64,
            observed: 128,
        };
        assert!(err.to_string().contains("128"));
        assert!(err.to_string().contains("64"));
    }

    #[test]
    fn cir_error_error_trait() {
        let err = CirError::Extraction("something went wrong".into());
        let msg = format!("{}", err);
        assert_eq!(msg, "CIR extraction error: something went wrong");
    }

    #[test]
    fn cir_error_from_serde_json() {
        let serde_err = serde_json::from_str::<String>("invalid").unwrap_err();
        let cir_err: CirError = serde_err.into();
        assert!(matches!(cir_err, CirError::Deserialization(_)));
    }

    #[test]
    fn cir_error_from_str() {
        let err: CirError = "test error".into();
        assert!(matches!(err, CirError::Extraction(_)));
        assert_eq!(err.to_string(), "CIR extraction error: test error");
    }

    #[test]
    fn cir_error_duplicate_node_display() {
        let err = CirError::DuplicateNode {
            id: "abc123".into(),
        };
        assert!(err.to_string().contains("abc123"));
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn cir_error_expression_invariant_display() {
        let err = CirError::ExpressionInvariant {
            node_id: "expr-1".into(),
            detail: "domain != codomain".into(),
        };
        let s = err.to_string();
        assert!(s.contains("expr-1"));
        assert!(s.contains("domain != codomain"));
    }

    #[test]
    fn cir_error_orphaned_expression_display() {
        let err = CirError::OrphanedExpression {
            node_id: "expr-2".into(),
            containing_id: "fn-missing".into(),
        };
        let s = err.to_string();
        assert!(s.contains("expr-2"));
        assert!(s.contains("fn-missing"));
    }
}
