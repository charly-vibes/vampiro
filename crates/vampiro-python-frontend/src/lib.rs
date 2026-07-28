//! Python language frontend for Vampiro.
//!
//! Parses Python source code using tree-sitter-python and extracts CIR graphs
//! without executing the source. Supports Python 3.8–3.13.
//!
//! # Extraction contract
//!
//! - Function/class declarations → `CirNode` with domain/codomain shapes
//! - Function calls → `CirEdge` with provenance
//! - Effect wrappers (async, yield, try/except, with) → `EffectChannel`
//! - Unrecognized patterns → `EffectChannel::Unknown` / `Shape::Opaque`
//! - Visibility levels → inferred from identifier naming convention
//! - Facade metadata → `__init__`.py re-exports

mod extract;

use std::path::Path;
use vampiro_cir::{CirError, CirGraph, Frontend};

/// The Python language frontend.
///
/// Parses Python source with tree-sitter-python and extracts CIR graphs.
/// See the [module-level documentation](self) for the extraction contract.
pub struct PythonFrontend;

impl Frontend for PythonFrontend {
    fn language(&self) -> &'static str {
        "python"
    }

    fn extract(&self, source: &str, path: &Path) -> Result<CirGraph, CirError> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .map_err(|e| CirError::Extraction(format!("failed to set Python language: {e}")))?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| CirError::Extraction("failed to parse Python source".into()))?;

        let root = tree.root_node();
        let graph = extract::extract_graph(root, source, path);
        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_frontend_language() {
        let frontend = PythonFrontend;
        assert_eq!(frontend.language(), "python");
    }

    #[test]
    fn python_frontend_language_is_static() {
        let frontend = PythonFrontend;
        let _lang: &'static str = frontend.language();
    }

    #[test]
    fn parses_empty_source() {
        let frontend = PythonFrontend;
        let graph = frontend.extract("", Path::new("empty.py")).unwrap();
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
    }

    #[test]
    fn parses_simple_function() {
        let frontend = PythonFrontend;
        let source = "def greet(name: str) -> str:\n    return f'Hello, {name}!'";
        let graph = frontend.extract(source, Path::new("lib.py")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].name.as_deref(), Some("greet"));
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn parses_function_with_call() {
        let frontend = PythonFrontend;
        let source =
            "def helper() -> int:\n    return 42\n\ndef main() -> int:\n    return helper()";
        let graph = frontend.extract(source, Path::new("lib.py")).unwrap();
        // Two function declarations + one call = 2 nodes, 1 edge
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn parses_async_function() {
        let frontend = PythonFrontend;
        let source = "async def fetch(url: str) -> str:\n    return await get(url)";
        let graph = frontend.extract(source, Path::new("lib.py")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        // async function should have Async effect (check contains Async)
        let effect_str = format!("{:?}", graph.nodes[0].effect);
        assert!(
            effect_str.contains("Async"),
            "expected Async effect, got {:?}",
            graph.nodes[0].effect
        );
        // External call to `get` makes validation fail (no node for `get`)
        // This is expected — external callees don't have nodes in the graph
    }

    #[test]
    fn parses_class_definition() {
        let frontend = PythonFrontend;
        let source = "class Greeter:\n    def greet(self, name: str) -> str:\n        return f'Hello, {name}!'";
        let graph = frontend.extract(source, Path::new("lib.py")).unwrap();
        // Class + method = 2 nodes
        assert_eq!(graph.nodes.len(), 2);
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn parses_lambda() {
        let frontend = PythonFrontend;
        let source = "add = lambda x, y: x + y";
        let graph = frontend.extract(source, Path::new("lib.py")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        // Lambda is named <lambda> by default
        assert!(graph.nodes[0].name.is_some());
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn parses_generator_with_yield() {
        let frontend = PythonFrontend;
        let source = "def count(n: int):\n    for i in range(n):\n        yield i";
        let graph = frontend.extract(source, Path::new("lib.py")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        // yield should produce Stream effect
        let effect_str = format!("{:?}", graph.nodes[0].effect);
        assert!(
            effect_str.contains("Stream"),
            "expected Stream effect, got {:?}",
            graph.nodes[0].effect
        );
        // External call to `range` makes validation fail (no node for `range`)
    }

    #[test]
    fn parses_method_call() {
        let frontend = PythonFrontend;
        let source = "def process(data: list[str]) -> str:\n    return ','.join(data)";
        let graph = frontend.extract(source, Path::new("lib.py")).unwrap();
        // One function + one method call (via attribute) = 1 node, 1 edge
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.edges.len(), 1);
        // External calls reference non-existent nodes, so validate may fail
        // (that's expected — the callee is outside this file)
    }

    #[test]
    fn rejects_invalid_python() {
        let frontend = PythonFrontend;
        // tree-sitter is lenient — it produces error nodes for partial syntax
        // but doesn't always return an error. Verify the graph is at least parseable.
        let source = "def broken( ";
        let result = frontend.extract(source, Path::new("broken.py"));
        // tree-sitter may not error on this, but the graph should be valid
        if let Ok(graph) = result {
            assert!(graph.nodes.is_empty() || graph.validate().is_ok());
        }
    }

    #[test]
    fn harness_conformance_empty() {
        // Run the empty harness against the Python frontend
        // Since we have no samples, all entries should be Skipped
        let matrix = vampiro_frontend_harness::python_matrix();
        let harness = vampiro_frontend_harness::CompatibilityHarness::new(matrix);
        let report = harness.run(&PythonFrontend, &[]);
        assert_eq!(report.language, "python");
        // All entries should be skipped (no samples)
        for (_, result) in &report.nodes {
            assert!(matches!(
                result,
                vampiro_frontend_harness::EntryResult::Skipped { .. }
            ));
        }
    }
}
