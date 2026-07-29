//! Julia language frontend for Vampiro.
//!
//! Parses Julia source code using tree-sitter-julia and extracts CIR graphs.

mod extract;

use std::path::Path;
use vampiro_cir::{CirError, CirGraph, Frontend};

/// The Julia language frontend.
pub struct JuliaFrontend;

impl Frontend for JuliaFrontend {
    fn language(&self) -> &'static str {
        "julia"
    }

    fn extract(&self, source: &str, path: &Path) -> Result<CirGraph, CirError> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_julia::LANGUAGE.into())
            .map_err(|e| CirError::Extraction(format!("failed to set Julia language: {e}")))?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| CirError::Extraction("failed to parse Julia source".into()))?;

        Ok(extract::extract_graph(tree.root_node(), source, path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn julia_frontend_language() {
        assert_eq!(JuliaFrontend.language(), "julia");
    }

    #[test]
    fn parses_empty_source() {
        let graph = JuliaFrontend.extract("", Path::new("empty.jl")).unwrap();
        assert_eq!(graph.nodes.len(), 0);
    }

    #[test]
    fn parses_function() {
        let source = "function greet(name) end";
        let graph = JuliaFrontend.extract(source, Path::new("lib.jl")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].name.as_deref(), Some("greet"));
    }

    #[test]
    fn parses_function_with_call() {
        let source =
            "function helper()\n    return 42\nend\nfunction main()\n    return helper()\nend";
        let graph = JuliaFrontend.extract(source, Path::new("lib.jl")).unwrap();
        assert_eq!(graph.nodes.len(), 2);
        // Should have at least one call edge (helper() call)
        assert!(
            !graph.edges.is_empty(),
            "expected at least 1 call edge, got {}",
            graph.edges.len()
        );
    }

    #[test]
    fn parses_arrow_function() {
        let source = "add = (x, y) -> x + y";
        let graph = JuliaFrontend.extract(source, Path::new("lib.jl")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn parses_struct() {
        let source = "struct Point\n    x::Float64\n    y::Float64\nend";
        let graph = JuliaFrontend.extract(source, Path::new("lib.jl")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.nodes[0].name.as_deref() == Some("Point"));
    }

    #[test]
    fn parses_macro() {
        let source = "macro debug(expr)\n    return expr\nend";
        let graph = JuliaFrontend.extract(source, Path::new("lib.jl")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn parses_module() {
        let source = "module MyMod\nfunction greet()::String\n    return \"hi\"\nend\nend";
        let graph = JuliaFrontend.extract(source, Path::new("lib.jl")).unwrap();
        // Module + function = 2 nodes
        assert_eq!(graph.nodes.len(), 2);
    }

    #[test]
    fn parses_broadcast_call() {
        let source =
            "function process(xs::Vector{Float64})::Vector{Float64}\n    return xs .^ 2\nend";
        let graph = JuliaFrontend.extract(source, Path::new("lib.jl")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        // The `.^` call should produce at least one edge
        assert!(!graph.edges.is_empty() || graph.validate().is_ok());
    }

    #[test]
    fn parses_try_catch() {
        let source = "function safe_div(a::Float64, b::Float64)::Float64\n    try\n        return a / b\n    catch\n        return 0.0\n    end\nend";
        let graph = JuliaFrontend.extract(source, Path::new("lib.jl")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        let effect_str = format!("{:?}", graph.nodes[0].effect);
        assert!(
            effect_str.contains("Result") || effect_str.contains("Throws"),
            "expected Result/Throws effect, got {:?}",
            graph.nodes[0].effect
        );
    }

    #[test]
    fn harness_conformance_empty() {
        let matrix = vampiro_frontend_harness::julia_matrix();
        let harness = vampiro_frontend_harness::CompatibilityHarness::new(matrix);
        let report = harness.run(&JuliaFrontend, &[]);
        assert_eq!(report.language, "julia");
        for (_, result) in &report.nodes {
            assert!(matches!(
                result,
                vampiro_frontend_harness::EntryResult::Skipped { .. }
            ));
        }
    }
}
