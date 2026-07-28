use crate::effect::{EffectChannel, EffectResolution, UnwrapEvidence};
use crate::error::{CirError, NodeRole};
use crate::provenance::{DiscardSpan, Provenance, SourceSpan, StableId};
use crate::shape::Shape;
use crate::TrustProvenance;

/// A node in the Composition IR, representing a callable or declaration.
///
/// Each node has a domain shape, codomain shape, and effect channel.
/// Nodes carry a stable identity and source span for traceability.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CirNode {
    /// Stable identity for this node.
    pub id: StableId,
    /// The domain (input) shape.
    pub domain: Shape,
    /// The codomain (output) shape.
    pub codomain: Shape,
    /// The effect channel of this node's output.
    pub effect: EffectChannel,
    /// Source span in the original file.
    pub span: SourceSpan,
    /// Optional name of the declaration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Trust provenance for this node's output value.
    ///
    /// Tracks whether the value produced by this declaration originates from
    /// a declared trust-boundary source. Nodes with no trust information
    /// default to `Trusted` (same-origin default).
    #[serde(default)]
    pub trust_provenance: TrustProvenance,
}

/// An edge in the Composition IR, representing a call site.
///
/// Each edge connects a source node (caller) to a target node (callee)
/// and carries the effect resolution and provenance information.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CirEdge {
    /// Stable identity for this edge.
    pub id: StableId,
    /// The source (caller) node ID.
    pub source: StableId,
    /// The target (callee) node ID.
    pub target: StableId,
    /// How the effect channel is resolved at this call site.
    pub resolution: EffectResolution,
    /// Evidence for unwrap resolutions, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unwrap_evidence: Option<UnwrapEvidence>,
    /// Provenance of argument flow.
    pub provenance: Provenance,
    /// Source span of the call site.
    pub span: SourceSpan,
    /// Exact discard spans at this call site, if any.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discard_spans: Vec<DiscardSpan>,
    /// Trust provenance for the argument value at this call site.
    ///
    /// Tracks the trust classification of the value flowing through this
    /// edge. Defaults to `Trusted` when absent (backward-compatible with
    /// graphs produced before this field was added).
    #[serde(default)]
    pub trust_provenance: TrustProvenance,
}

/// A complete Composition IR graph for a single compilation unit.
///
/// Contains the nodes and edges extracted from a source file, along with
/// the file-level metadata.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CirGraph {
    /// The schema version of this CIR graph.
    pub version: String,
    /// The source file path.
    pub source_file: String,
    /// All nodes in the graph.
    pub nodes: Vec<CirNode>,
    /// All edges in the graph.
    pub edges: Vec<CirEdge>,
    /// Validation observations extracted by frontends, keyed by stable
    /// validation identity.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_observations: Vec<crate::ValidationObservation>,
}

impl CirGraph {
    /// Create a new empty CIR graph for the given source file.
    pub fn new(source_file: impl Into<String>) -> Self {
        CirGraph {
            version: "0.1.0".into(),
            source_file: source_file.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            validation_observations: Vec::new(),
        }
    }

    /// Add a node to the graph.
    pub fn add_node(&mut self, node: CirNode) {
        self.nodes.push(node);
    }

    /// Add an edge to the graph.
    pub fn add_edge(&mut self, edge: CirEdge) {
        self.edges.push(edge);
    }

    /// Find a node by its stable ID.
    ///
    /// Performs an O(n) linear scan. Callers that issue many lookups should
    /// build their own `HashMap<&StableId, &CirNode>` index rather than call
    /// this repeatedly.
    pub fn node_by_id(&self, id: &StableId) -> Option<&CirNode> {
        self.nodes.iter().find(|n| n.id == *id)
    }

    /// Validate the graph's internal invariants.
    ///
    /// Checks:
    /// - All edge source/target references point to existing nodes.
    /// - All effect channels are within the maximum depth limit.
    /// - All shapes are within the maximum depth limit.
    ///
    /// Returns `Ok(())` if all invariants hold, or the first error encountered.
    pub fn validate(&self) -> Result<(), CirError> {
        // Collect node IDs for O(1) lookup and detect duplicates.
        let mut node_ids: std::collections::HashSet<&StableId> = std::collections::HashSet::new();
        for node in &self.nodes {
            if !node_ids.insert(&node.id) {
                return Err(CirError::DuplicateNode {
                    id: node.id.to_string(),
                });
            }
        }

        // Validate edges
        for edge in &self.edges {
            if !node_ids.contains(&edge.source) {
                return Err(CirError::MissingNode {
                    edge_id: edge.id.to_string(),
                    node_id: edge.source.to_string(),
                    role: NodeRole::Source,
                });
            }
            if !node_ids.contains(&edge.target) {
                return Err(CirError::MissingNode {
                    edge_id: edge.id.to_string(),
                    node_id: edge.target.to_string(),
                    role: NodeRole::Target,
                });
            }
        }

        // Validate effect channel depths
        for node in &self.nodes {
            let depth = node.effect.depth();
            if depth > crate::effect::MAX_EFFECT_DEPTH {
                return Err(CirError::EffectDepthExceeded {
                    max_depth: crate::effect::MAX_EFFECT_DEPTH,
                    observed: depth,
                });
            }
        }

        // Validate shape depths
        for node in &self.nodes {
            let dom_depth = node.domain.depth();
            if dom_depth > crate::shape::MAX_SHAPE_DEPTH {
                return Err(CirError::ShapeDepthExceeded {
                    max_depth: crate::shape::MAX_SHAPE_DEPTH,
                    observed: dom_depth,
                });
            }
            let cod_depth = node.codomain.depth();
            if cod_depth > crate::shape::MAX_SHAPE_DEPTH {
                return Err(CirError::ShapeDepthExceeded {
                    max_depth: crate::shape::MAX_SHAPE_DEPTH,
                    observed: cod_depth,
                });
            }
        }

        Ok(())
    }

    /// Deserialize a `CirGraph` from a JSON string, with validation.
    pub fn from_json(json: &str) -> Result<Self, CirError> {
        let graph: CirGraph = serde_json::from_str(json)?;
        graph.validate()?;
        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{EffectChannel, Totality, UnwrapEvidence, UnwrapKind};
    use crate::provenance::Provenance;

    fn make_node(id: &str, name: &str, domain: Shape, codomain: Shape) -> CirNode {
        CirNode {
            id: StableId::new(id),
            domain,
            codomain,
            effect: EffectChannel::Plain,
            span: SourceSpan {
                file: "test.rs".into(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 10,
            },
            name: Some(name.into()),
            trust_provenance: Default::default(),
        }
    }

    #[test]
    fn cir_graph_round_trip() {
        // Build a simple graph: one function calling another
        let mut graph = CirGraph::new("src/lib.rs");

        let caller = make_node("caller", "caller_fn", Shape::Scalar, Shape::Scalar);
        let callee = make_node("callee", "callee_fn", Shape::Scalar, Shape::Scalar);

        let edge = CirEdge {
            id: StableId::new("edge-1"),
            source: caller.id.clone(),
            target: callee.id.clone(),
            resolution: EffectResolution::Propagated,
            unwrap_evidence: None,
            provenance: Provenance::Direct,
            span: SourceSpan {
                file: "src/lib.rs".into(),
                start_line: 5,
                start_column: 1,
                end_line: 5,
                end_column: 20,
            },
            discard_spans: vec![],
            trust_provenance: Default::default(),
        };

        graph.add_node(caller);
        graph.add_node(callee);
        graph.add_edge(edge);

        // Round-trip through JSON
        let json = serde_json::to_string_pretty(&graph).unwrap();
        let deserialized: CirGraph = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, "0.1.0");
        assert_eq!(deserialized.source_file, "src/lib.rs");
        assert_eq!(deserialized.nodes.len(), 2);
        assert_eq!(deserialized.edges.len(), 1);
        assert_eq!(
            deserialized
                .node_by_id(&StableId::new("caller"))
                .unwrap()
                .name
                .as_deref(),
            Some("caller_fn")
        );
        assert_eq!(
            deserialized
                .node_by_id(&StableId::new("callee"))
                .unwrap()
                .name
                .as_deref(),
            Some("callee_fn")
        );
    }

    #[test]
    fn cir_graph_with_effects() {
        // A graph with recursive effects and unwrap evidence
        let mut graph = CirGraph::new("effects.rs");

        let node = CirNode {
            id: StableId::new("fn-1"),
            domain: Shape::Scalar,
            codomain: Shape::Scalar,
            effect: EffectChannel::Recursive(Box::new(EffectChannel::Option)),
            span: SourceSpan {
                file: "effects.rs".into(),
                start_line: 1,
                start_column: 1,
                end_line: 3,
                end_column: 1,
            },
            name: Some("risky_fn".into()),
            trust_provenance: Default::default(),
        };

        let edge = CirEdge {
            id: StableId::new("call-1"),
            source: StableId::new("fn-1"),
            target: StableId::new("fn-2"),
            resolution: EffectResolution::Unwrapped,
            unwrap_evidence: Some(UnwrapEvidence {
                kind: UnwrapKind::Ordinary,
                totality: Totality::Partial,
            }),
            provenance: Provenance::WithinH { hops: 1 },
            span: SourceSpan {
                file: "effects.rs".into(),
                start_line: 5,
                start_column: 1,
                end_line: 5,
                end_column: 15,
            },
            discard_spans: vec![DiscardSpan {
                file: "effects.rs".into(),
                start_line: 6,
                end_line: 8,
            }],
            trust_provenance: Default::default(),
        };

        graph.add_node(node);
        graph.add_edge(edge);

        let json = serde_json::to_string_pretty(&graph).unwrap();
        let _deserialized: CirGraph = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn cir_graph_empty() {
        let graph = CirGraph::new("empty.rs");
        let json = serde_json::to_string_pretty(&graph).unwrap();
        let deserialized: CirGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.nodes.len(), 0);
        assert_eq!(deserialized.edges.len(), 0);
        assert_eq!(deserialized.validation_observations.len(), 0);
    }

    #[test]
    fn cir_graph_with_validation_observations() {
        // Test that ValidationObservations round-trip through CirGraph
        let mut graph = CirGraph::new("lib.rs");
        graph
            .validation_observations
            .push(crate::ValidationObservation::new(
                "validate_user",
                "rust",
                crate::StableId::new("User::new"),
                "User",
                crate::SourceSpan {
                    file: "lib.rs".into(),
                    start_line: 10,
                    start_column: 1,
                    end_line: 10,
                    end_column: 30,
                },
                "idiom",
            ));

        let json = serde_json::to_string_pretty(&graph).unwrap();
        let deserialized: CirGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.validation_observations.len(), 1);
        assert_eq!(
            deserialized.validation_observations[0].identity,
            "validate_user"
        );
        assert_eq!(deserialized.validation_observations[0].origin, "idiom");
    }

    #[test]
    fn cir_graph_custom_effect() {
        // Project-declared custom effect and resolution
        let mut graph = CirGraph::new("custom.rs");

        let node = CirNode {
            id: StableId::new("custom-node"),
            domain: Shape::Scalar,
            codomain: Shape::Scalar,
            effect: EffectChannel::Custom("my-eff".into()),
            span: SourceSpan {
                file: "custom.rs".into(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 1,
            },
            name: Some("custom_fn".into()),
            trust_provenance: Default::default(),
        };

        let edge = CirEdge {
            id: StableId::new("custom-edge"),
            source: StableId::new("custom-node"),
            target: StableId::new("target"),
            resolution: EffectResolution::Custom("my-res".into()),
            unwrap_evidence: None,
            provenance: Provenance::Direct,
            span: SourceSpan {
                file: "custom.rs".into(),
                start_line: 2,
                start_column: 1,
                end_line: 2,
                end_column: 1,
            },
            discard_spans: vec![],
            trust_provenance: Default::default(),
        };

        graph.add_node(node);
        graph.add_edge(edge);

        let json = serde_json::to_string_pretty(&graph).unwrap();
        let deserialized: CirGraph = serde_json::from_str(&json).unwrap();

        match &deserialized.nodes[0].effect {
            EffectChannel::Custom(name) => assert_eq!(name, "my-eff"),
            _ => panic!("expected custom effect"),
        }
        match &deserialized.edges[0].resolution {
            EffectResolution::Custom(name) => assert_eq!(name, "my-res"),
            _ => panic!("expected custom resolution"),
        }
    }

    #[test]
    fn cir_graph_validate_missing_node() {
        let mut graph = CirGraph::new("test.rs");
        let node = make_node("a", "fn_a", Shape::Scalar, Shape::Scalar);
        let edge = CirEdge {
            id: StableId::new("e1"),
            source: StableId::new("a"),
            target: StableId::new("b"), // target 'b' doesn't exist
            resolution: EffectResolution::Propagated,
            unwrap_evidence: None,
            provenance: Provenance::Direct,
            span: SourceSpan {
                file: "test.rs".into(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 1,
            },
            discard_spans: vec![],
            trust_provenance: Default::default(),
        };
        graph.add_node(node);
        graph.add_edge(edge);

        let result = graph.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            CirError::MissingNode {
                edge_id,
                node_id,
                role,
            } => {
                assert_eq!(edge_id, "e1");
                assert_eq!(node_id, "b");
                assert_eq!(role, NodeRole::Target);
            }
            other => panic!("expected MissingNode, got {other:?}"),
        }
    }

    #[test]
    fn cir_graph_validate_effect_depth() {
        let mut graph = CirGraph::new("test.rs");
        let mut deep_effect = EffectChannel::Plain;
        for _ in 0..(crate::effect::MAX_EFFECT_DEPTH + 1) {
            deep_effect = EffectChannel::Recursive(Box::new(deep_effect));
        }

        let node = CirNode {
            id: StableId::new("deep"),
            domain: Shape::Scalar,
            codomain: Shape::Scalar,
            effect: deep_effect,
            span: SourceSpan {
                file: "test.rs".into(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 1,
            },
            name: None,
            trust_provenance: Default::default(),
        };
        graph.add_node(node);

        let result = graph.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            CirError::EffectDepthExceeded { .. } => {} // expected
            other => panic!("expected EffectDepthExceeded, got {other:?}"),
        }
    }

    #[test]
    fn cir_graph_validate_shape_depth() {
        let mut graph = CirGraph::new("test.rs");
        let mut deep_shape = Shape::Scalar;
        for _ in 0..(crate::shape::MAX_SHAPE_DEPTH + 1) {
            deep_shape = Shape::Record(vec![deep_shape]);
        }

        let node = CirNode {
            id: StableId::new("deep"),
            domain: deep_shape,
            codomain: Shape::Scalar,
            effect: EffectChannel::Plain,
            span: SourceSpan {
                file: "test.rs".into(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 1,
            },
            name: None,
            trust_provenance: Default::default(),
        };
        graph.add_node(node);

        let result = graph.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            CirError::ShapeDepthExceeded { .. } => {} // expected
            other => panic!("expected ShapeDepthExceeded, got {other:?}"),
        }
    }

    #[test]
    fn cir_graph_from_json_valid() {
        let json = r#"{
            "version": "0.1.0",
            "source_file": "test.rs",
            "nodes": [{
                "id": "n1",
                "domain": "scalar",
                "codomain": "scalar",
                "effect": "plain",
                "span": { "file": "test.rs", "start_line": 1, "start_column": 1, "end_line": 1, "end_column": 1 }
            }],
            "edges": []
        }"#;
        let graph = CirGraph::from_json(json).unwrap();
        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn cir_graph_validate_duplicate_node() {
        let mut graph = CirGraph::new("test.rs");
        let mut node = make_node("dup", "fn_a", Shape::Scalar, Shape::Scalar);
        graph.add_node(node.clone());
        // Same id, different name — should be rejected.
        node.name = Some("fn_b".into());
        graph.add_node(node);

        let result = graph.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CirError::DuplicateNode { .. }
        ));
    }

    #[test]
    fn cir_graph_from_json_invalid_missing_node() {
        let json = r#"{
            "version": "0.1.0",
            "source_file": "test.rs",
            "nodes": [],
            "edges": [{
                "id": "e1",
                "source": "n1",
                "target": "n2",
                "resolution": "propagated",
                "provenance": "direct",
                "span": { "file": "test.rs", "start_line": 1, "start_column": 1, "end_line": 1, "end_column": 1 },
                "discard_spans": []
            }]
        }"#;
        let result = CirGraph::from_json(json);
        assert!(result.is_err());
    }
}
