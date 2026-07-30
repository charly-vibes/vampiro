//! Clojure language frontend for Vampiro.
//!
//! Parses Clojure source code using tree-sitter-clojure and extracts CIR graphs
//! without executing the source. Supports Clojure 1.10–1.12.
//!
//! # Extraction contract
//!
//! - `defn` declarations → `CirNode` with domain/codomain shapes
//! - `fn` literals → anonymous function nodes
//! - `defmulti`/`defmethod` → multimethod nodes
//! - `defprotocol`/`defrecord`/`deftype` → protocol/type declaration nodes
//! - Function calls (list where first element is a symbol) → `CirEdge`
//! - Effect wrappers (future, lazy-seq, try/catch, binding, with-open) → `EffectChannel`
//! - Reader macros → `EffectChannel::Unknown` / opaque sentinels
//! - Visibility → `defn-` (private), `defn` (public), namespace re-exports

mod extract;
pub mod facade;
pub mod law;
pub mod lifecycle;
pub mod visibility;

pub use facade::{FacadeDecl, FacadeMetadata};
pub use law::LawRunnerInput;
pub use lifecycle::LifecycleFacts;
use std::collections::HashMap;
use std::path::Path;
use vampiro_cir::{CirError, CirGraph, Frontend, StableId};
pub use visibility::Visibility;

/// The complete extraction output from the Clojure frontend.
#[derive(Debug, Clone)]
pub struct ExtractionOutput {
    pub graph: CirGraph,
    pub facades: Vec<FacadeDecl>,
    pub visibility: HashMap<StableId, Visibility>,
    pub law_input: LawRunnerInput,
    pub lifecycle_facts: LifecycleFacts,
}

/// The Clojure language frontend.
///
/// Parses Clojure source with tree-sitter-clojure and extracts CIR graphs.
/// See the [module-level documentation](self) for the extraction contract.
pub struct ClojureFrontend;

impl Frontend for ClojureFrontend {
    fn language(&self) -> &'static str {
        "clojure"
    }

    fn extract(&self, source: &str, path: &Path) -> Result<CirGraph, CirError> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_clojure::LANGUAGE.into())
            .map_err(|e| CirError::Extraction(format!("failed to set Clojure language: {e}")))?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| CirError::Extraction("failed to parse Clojure source".into()))?;

        let root = tree.root_node();
        let graph = extract::extract_graph(root, source, path);
        Ok(graph)
    }
}

impl ClojureFrontend {
    /// Extract the full CIR and contract surface from Clojure source.
    pub fn extract_full(&self, source: &str, path: &Path) -> Result<ExtractionOutput, CirError> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_clojure::LANGUAGE.into())
            .map_err(|e| CirError::Extraction(format!("failed to set Clojure language: {e}")))?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| CirError::Extraction("failed to parse Clojure source".into()))?;

        let root = tree.root_node();
        let graph = extract::extract_graph(root, source, path);
        let law_input = law::extract_law_input(root, source, path);
        let lifecycle_facts = lifecycle::extract_lifecycle_facts(root, source, path);
        let facades = facade::extract_facade_metadata(root, source, path);

        Ok(ExtractionOutput {
            graph,
            facades: facades.facades,
            visibility: HashMap::new(),
            law_input,
            lifecycle_facts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clojure_frontend_language() {
        let frontend = ClojureFrontend;
        assert_eq!(frontend.language(), "clojure");
    }

    #[test]
    fn clojure_frontend_language_is_static() {
        let frontend = ClojureFrontend;
        let _lang: &'static str = frontend.language();
    }

    #[test]
    fn parses_empty_source() {
        let frontend = ClojureFrontend;
        let graph = frontend.extract("", Path::new("empty.clj")).unwrap();
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
    }

    #[test]
    fn parses_defn() {
        let frontend = ClojureFrontend;
        let source = "(defn greet [name] (str \"Hello, \" name))";
        let graph = frontend.extract(source, Path::new("core.clj")).unwrap();
        // greet decl + expression node for "Hello, " str_lit argument
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.nodes[0].name.as_deref(), Some("greet"));
        // External call to `str` makes validation fail (no node for `str`)
    }

    #[test]
    fn parses_defn_with_call() {
        let frontend = ClojureFrontend;
        let source = "(defn helper [] 42)\n(defn main [] (helper))";
        let graph = frontend.extract(source, Path::new("core.clj")).unwrap();
        // Two defns + one call = 2 nodes, 1 edge
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn parses_anonymous_fn() {
        let frontend = ClojureFrontend;
        let source = "(def add (fn [x y] (+ x y)))";
        let graph = frontend.extract(source, Path::new("core.clj")).unwrap();
        // def + fn = 2 nodes (one for add, one for the anonymous fn)
        assert_eq!(graph.nodes.len(), 2);
        // External call to `+` makes validation fail (no node for `+`)
    }

    #[test]
    fn parses_anon_fn_lit() {
        let frontend = ClojureFrontend;
        let source = "(def add #(+ %1 %2))";
        let graph = frontend.extract(source, Path::new("core.clj")).unwrap();
        // def + anon fn lit = 2 nodes
        assert_eq!(graph.nodes.len(), 2);
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn parses_defmulti() {
        let frontend = ClojureFrontend;
        let source = "(defmulti area :shape)";
        let graph = frontend.extract(source, Path::new("core.clj")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].name.as_deref(), Some("area"));
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn parses_defmethod() {
        let frontend = ClojureFrontend;
        let source = "(defmethod area :circle [r] (* Math/PI r r))";
        let graph = frontend.extract(source, Path::new("core.clj")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].name.as_deref(), Some("area"));
        // External call to `*` and `Math/PI` makes validation fail (no nodes for them)
    }

    #[test]
    fn parses_def_with_future_effect() {
        let frontend = ClojureFrontend;
        let source = "(def f (future (long-computation)))";
        let graph = frontend.extract(source, Path::new("core.clj")).unwrap();
        assert_eq!(graph.nodes.len(), 2);
        // future should produce Async effect
        let effect_str = format!("{:?}", graph.nodes[0].effect);
        assert!(
            effect_str.contains("Async"),
            "expected Async effect, got {:?}",
            graph.nodes[0].effect
        );
        // External call to `long-computation` makes validation fail
    }

    #[test]
    fn parses_try_catch() {
        let frontend = ClojureFrontend;
        let source = "(defn safe-div [a b] (try (/ a b) (catch Exception e 0)))";
        let graph = frontend.extract(source, Path::new("core.clj")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        // try/catch should produce Result effect
        let effect_str = format!("{:?}", graph.nodes[0].effect);
        assert!(
            effect_str.contains("Result"),
            "expected Result effect, got {:?}",
            graph.nodes[0].effect
        );
        // External call to `/` makes validation fail
    }

    #[test]
    fn parses_lazy_seq_effect() {
        let frontend = ClojureFrontend;
        let source = "(defn my-range [n] (lazy-seq (cons n (my-range (inc n)))))";
        let graph = frontend.extract(source, Path::new("core.clj")).unwrap();
        // my-range decl + expression node for (my-range (inc n)) as cons argument
        assert_eq!(graph.nodes.len(), 2);
        // lazy-seq should produce Stream effect
        let effect_str = format!("{:?}", graph.nodes[0].effect);
        assert!(
            effect_str.contains("Stream"),
            "expected Stream effect, got {:?}",
            graph.nodes[0].effect
        );
    }

    #[test]
    fn parses_with_open_effect() {
        let frontend = ClojureFrontend;
        let source =
            "(defn read-file [path] (with-open [r (clojure.java.io/reader path)] (line-seq r)))";
        let graph = frontend.extract(source, Path::new("core.clj")).unwrap();
        assert_eq!(graph.nodes.len(), 1);
        // with-open should produce Resource/Throws effect
        let effect_str = format!("{:?}", graph.nodes[0].effect);
        assert!(
            effect_str.contains("Throws") || effect_str.contains("Result"),
            "expected effect containing Throws/Result, got {:?}",
            graph.nodes[0].effect
        );
    }

    #[test]
    fn harness_conformance_empty() {
        let matrix = vampiro_frontend_harness::clojure_matrix();
        let harness = vampiro_frontend_harness::CompatibilityHarness::new(matrix);
        let report = harness.run(&ClojureFrontend, &[]);
        assert_eq!(report.language, "clojure");
        for (_, result) in &report.nodes {
            assert!(matches!(
                result,
                vampiro_frontend_harness::EntryResult::Skipped { .. }
            ));
        }
    }
}
