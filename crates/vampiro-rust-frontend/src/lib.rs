//! Rust language frontend for Vampiro.
//!
//! Parses Rust source code using `syn` and extracts CIR graphs
//! without executing the source. Supports Rust 2021+.
//!
//! # Extraction contract
//!
//! - Function/closure declarations → `CirNode` with domain/codomain shapes
//! - Function calls → `CirEdge` with provenance
//! - Effect wrappers (Result, Option, async, Iterator) → `EffectChannel`
//! - Unwrap operations (`?`, `unwrap`, `expect`, panic unwrap) → `EffectResolution` + `UnwrapEvidence`
//! - Unrecognized patterns → `EffectChannel::Unknown` / `Shape::Opaque`
//! - Visibility levels → `Visibility` enum (independently versioned table)
//! - Re-exports → `FacadeDecl` entries with original paths
//! - Law runner-input (impl clusters, tagged fns, serializable values, generators)
//! - Lifecycle facts (writes, retries, resources, exit paths, aliases)
//!
//! The language-neutral [`Frontend`] trait returns only the [`CirGraph`]; the
//! additional contract data (visibility, facades, law runner-input, lifecycle
//! facts) is returned by [`RustFrontend::extract_full`] as an
//! [`ExtractionOutput`]. Analysis layers that need the full surface should
//! call `extract_full`; consumers that only need the graph can use the trait.

mod extract;
pub mod law;
pub mod lifecycle;
pub mod visibility;
pub mod visibility_adapter;

use std::collections::HashMap;
use std::path::Path;
use vampiro_cir::{CirError, CirGraph, Frontend, StableId};
pub use visibility::{FacadeDecl, Visibility};

pub use law::LawRunnerInput;
pub use lifecycle::LifecycleFacts;

/// The complete extraction output from the Rust frontend.
///
/// Bundles the CIR graph with the additional contract data that the
/// language-neutral [`Frontend`] trait cannot express. Produced by
/// [`RustFrontend::extract_full`].
#[derive(Debug, Clone)]
pub struct ExtractionOutput {
    /// The extracted CIR graph.
    pub graph: CirGraph,
    /// Facade declarations (re-exports) at each module level.
    pub facades: Vec<FacadeDecl>,
    /// Visibility map: node stable ID → visibility level.
    pub visibility: HashMap<StableId, Visibility>,
    /// Law runner-input data (impl clusters, tagged fns, etc.).
    pub law_input: LawRunnerInput,
    /// Lifecycle facts (writes, retries, resources, exit paths, aliases).
    pub lifecycle_facts: LifecycleFacts,
}

/// The Rust language frontend.
///
/// Parses Rust source with `syn` and extracts CIR graphs.
/// See the [module-level documentation](self) for the extraction contract.
pub struct RustFrontend;

impl Frontend for RustFrontend {
    fn language(&self) -> &'static str {
        "rust"
    }

    fn extract(&self, source: &str, path: &Path) -> Result<CirGraph, CirError> {
        let syntax = syn::parse_file(source)
            .map_err(|e| CirError::Extraction(format!("failed to parse Rust source: {e}")))?;
        let result = extract::extract_graph(&syntax, path, source);
        Ok(result.graph)
    }
}

impl RustFrontend {
    /// Extract the full CIR and runner-input surface from Rust source.
    ///
    /// Returns the graph plus visibility, facades, law runner-input, and
    /// lifecycle facts that the [`Frontend`] trait cannot carry.
    pub fn extract_full(&self, source: &str, path: &Path) -> Result<ExtractionOutput, CirError> {
        let syntax = syn::parse_file(source)
            .map_err(|e| CirError::Extraction(format!("failed to parse Rust source: {e}")))?;
        let result = extract::extract_graph(&syntax, path, source);
        let law_input = law::extract_law_input(&syntax, path);
        let lifecycle_facts = lifecycle::extract_lifecycle_facts(&syntax, path);
        Ok(ExtractionOutput {
            graph: result.graph,
            facades: result.facades,
            visibility: result.visibility,
            law_input,
            lifecycle_facts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vampiro_cir::EffectChannel;

    #[test]
    fn rust_frontend_language() {
        let frontend = RustFrontend;
        assert_eq!(frontend.language(), "rust");
    }

    #[test]
    fn rust_frontend_language_is_static() {
        let frontend = RustFrontend;
        let _lang: &'static str = frontend.language();
    }

    #[test]
    fn parses_empty_source() {
        let frontend = RustFrontend;
        let graph = frontend.extract("", Path::new("empty.rs")).unwrap();
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
    }

    #[test]
    fn parses_simple_function() {
        let frontend = RustFrontend;
        let source = r#"
fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}
"#;
        let graph = frontend.extract(source, Path::new("lib.rs")).unwrap();
        // One function declaration = one node
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].name.as_deref(), Some("greet"));
        // Domain: &str, Codomain: String
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn parses_function_with_async_effect() {
        let frontend = RustFrontend;
        let source = r#"
async fn fetch_data(url: &str) -> Result<String, Error> {
    // ...
    Ok("data".to_string())
}
"#;
        let graph = frontend.extract(source, Path::new("lib.rs")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].name.as_deref(), Some("fetch_data"));
        assert_eq!(
            graph.nodes[0].effect,
            EffectChannel::Recursive(Box::new(EffectChannel::Result))
        );
        let result = graph.validate();
        assert!(result.is_ok(), "validation failed: {result:?}");
    }

    #[test]
    fn parses_function_with_calls() {
        let frontend = RustFrontend;
        let source = r#"
fn helper() -> i32 { 42 }

fn main() -> i32 {
    helper()
}
"#;
        let graph = frontend.extract(source, Path::new("lib.rs")).unwrap();
        // Two function declarations + one call = 2 nodes, 1 edge
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn rejects_invalid_rust() {
        let frontend = RustFrontend;
        let source = "fn broken( { }";
        let result = frontend.extract(source, Path::new("broken.rs"));
        assert!(result.is_err());
        match result.unwrap_err() {
            CirError::Extraction(msg) => {
                assert!(msg.contains("failed to parse"));
            }
            other => panic!("expected Extraction error, got: {other:?}"),
        }
    }

    #[test]
    fn extract_full_returns_full_surface() {
        let frontend = RustFrontend;
        let source = "pub use foo::bar;\npub fn greet() -> String { format!(\"hi\") }";
        let out = frontend.extract_full(source, Path::new("lib.rs")).unwrap();
        assert_eq!(out.graph.nodes.len(), 1);
        assert!(!out.facades.is_empty(), "facades should be populated");
        assert!(!out.visibility.is_empty(), "visibility should be populated");
        assert_eq!(out.law_input.version, law::RUNNER_INPUT_SCHEMA_VERSION);
        assert_eq!(
            out.lifecycle_facts.version,
            lifecycle::LIFECYCLE_FACT_SCHEMA_VERSION
        );
    }
}
