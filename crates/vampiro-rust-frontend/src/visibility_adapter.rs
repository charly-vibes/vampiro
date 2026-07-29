//! Rust visibility adapter — the Rust idiom table (REQ-V1).
//!
//! Maps the Rust frontend's language-specific extraction output
//! ([`ExtractionOutput`]: CIR graph + Rust [`Visibility`] + [`FacadeDecl`]s)
//! into language-neutral [`VisibilityFacts`] that the analysis layer consumes.
//!
//! This is the Rust instantiation of the Addendum V per-language visibility
//! idiom table:

use std::collections::BTreeSet;

use vampiro_cir::visibility::{
    BoundaryKind, FacadeReexport, LatticeLevel, VisibilityFact, VisibilityFacts,
};
use vampiro_cir::StableId;

use crate::{ExtractionOutput, FacadeDecl, Visibility};

/// Map a Rust frontend extraction output to language-neutral visibility facts.
///
/// Implements the Rust row of the Addendum V visibility idiom table:
///
/// | Rust | Lattice | Boundary |
/// |------|---------|----------|
/// | `pub` + in crate-root facade, not doc(hidden), no leading `_` | L4 | enforced-open |
/// | `pub` + doc(hidden) / leading `_` / excluded from facade | L3 | enforced-open |
/// | `pub(crate)` / `pub(in path)` | L2 | enforced |
/// | `pub(super)` | L1.5 | enforced |
/// | `pub(self)` / inherited | L1 | enforced |
/// | local bindings | L0 | enforced |
pub fn to_visibility_facts(out: &ExtractionOutput) -> VisibilityFacts {
    let mut facts = VisibilityFacts::new(Visibility::TABLE_VERSION);

    // Collect the set of names re-exported at the crate-root facade (module_path
    // is empty), so we can distinguish L4 (in facade) from L3 (not in facade).
    let crate_root_exports: BTreeSet<String> = out
        .facades
        .iter()
        .filter(|fd| fd.module_path.is_empty())
        .flat_map(|fd| fd.entries.iter())
        .map(|e| e.name.clone())
        .collect();

    // Collect doc(hidden) names from facade entries.
    let doc_hidden_names: BTreeSet<String> = out
        .facades
        .iter()
        .flat_map(|fd| &fd.entries)
        .filter(|e| e.doc_hidden)
        .map(|e| e.name.clone())
        .collect();

    // Map each node's Rust visibility to a lattice level + boundary kind.
    for (node_id, vis) in &out.visibility {
        let node = match out.graph.node_by_id(node_id) {
            Some(n) => n,
            None => continue,
        };
        let scope = node.span.file.clone();
        let name = node.name.as_deref().unwrap_or("");

        let (level, boundary, internal) = match vis {
            Visibility::Public => {
                let in_facade = crate_root_exports.contains(name);
                let doc_hidden = doc_hidden_names.contains(name);
                let leading_underscore = name.starts_with('_');
                let internal = doc_hidden || leading_underscore || !in_facade;
                if in_facade && !doc_hidden && !leading_underscore {
                    (LatticeLevel::L4, BoundaryKind::EnforcedOpen, false)
                } else {
                    (LatticeLevel::L3, BoundaryKind::EnforcedOpen, internal)
                }
            }
            Visibility::Crate | Visibility::Restricted(_) => {
                (LatticeLevel::L2, BoundaryKind::Enforced, false)
            }
            Visibility::Super => (LatticeLevel::L1Half, BoundaryKind::Enforced, false),
            Visibility::Private => (LatticeLevel::L1, BoundaryKind::Enforced, false),
        };
        facts.add_fact(VisibilityFact {
            node: node_id.clone(),
            level,
            boundary,
            scope,
            internal_by_convention: internal,
        });
    }

    /// Returns `true` if the module path segment looks like a test module.
    ///
    /// Rust convention: test modules are typically named `*_test`, `*_tests`, or `tests`.
    /// These modules' `use super::*;` re-exports should not trigger facade-leak findings
    /// since they're private testing infrastructure, not public API facades.
    fn is_test_module(segment: &str) -> bool {
        segment == "tests" || segment.ends_with("_test") || segment.ends_with("_tests")
    }

    /// Returns `true` if a `::`-joined module path contains a test-module segment.
    fn module_path_contains_test_segment(module_path: &str) -> bool {
        module_path.split("::").any(|seg| is_test_module(seg))
    }

    // Map facades to language-neutral re-exports. Match by name to find the
    // underlying node. Skip facades in test modules — their `use super::*;`
    // re-exports are private testing infrastructure, not public API facades.
    for fd in &out.facades {
        if module_path_contains_test_segment(&fd.module_path) {
            continue;
        }
        for entry in &fd.entries {
            if let Some(node) = out
                .graph
                .nodes
                .iter()
                .find(|n| n.name.as_deref() == Some(entry.name.as_str()))
            {
                facts.add_facade(FacadeReexport {
                    facade_scope: fd.module_path.clone(),
                    exported_name: entry.name.clone(),
                    underlying_node: node.id.clone(),
                });
            }
        }
    }

    facts
}

/// Re-export for type aliases used by the adapter.
#[allow(dead_code)]
type _FacadeAnchor = FacadeDecl;

/// Re-export for the StableId anchor.
#[allow(dead_code)]
type _StableIdAnchor = StableId;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RustFrontend;
    use std::path::Path;

    fn extract(source: &str) -> ExtractionOutput {
        RustFrontend
            .extract_full(source, Path::new("lib.rs"))
            .expect("extraction must succeed")
    }

    #[test]
    fn pub_fn_not_in_facade_maps_to_l3() {
        // A bare `pub fn` without a `pub use` facade re-export is L3
        // (public-unstable) per Addendum V. The frontend does not currently
        // track crate-root pub declarations as facade entries — only `pub use`
        // re-exports — so a bare pub fn is not in the facade set.
        let out = extract("pub fn public_api() -> u32 { 0 }");
        let vis = to_visibility_facts(&out);
        let node = out
            .graph
            .nodes
            .iter()
            .find(|n| n.name.as_deref() == Some("public_api"))
            .expect("node must exist");
        let fact = vis.fact_for(&node.id).expect("fact must exist");
        assert_eq!(fact.level, LatticeLevel::L3);
        assert_eq!(fact.boundary, BoundaryKind::EnforcedOpen);
    }

    #[test]
    fn pub_fn_with_leading_underscore_maps_to_l3_internal() {
        let out = extract("pub fn _helper() -> u32 { 0 }");
        let vis = to_visibility_facts(&out);
        let node = out
            .graph
            .nodes
            .iter()
            .find(|n| n.name.as_deref() == Some("_helper"))
            .expect("node must exist");
        let fact = vis.fact_for(&node.id).expect("fact must exist");
        assert_eq!(fact.level, LatticeLevel::L3);
        assert_eq!(fact.boundary, BoundaryKind::EnforcedOpen);
        assert!(fact.internal_by_convention);
    }

    #[test]
    fn pub_crate_fn_maps_to_l2_enforced() {
        let out = extract("pub(crate) fn internal_fn() -> u32 { 0 }");
        let vis = to_visibility_facts(&out);
        let node = out
            .graph
            .nodes
            .iter()
            .find(|n| n.name.as_deref() == Some("internal_fn"))
            .expect("node must exist");
        let fact = vis.fact_for(&node.id).expect("fact must exist");
        assert_eq!(fact.level, LatticeLevel::L2);
        assert_eq!(fact.boundary, BoundaryKind::Enforced);
    }

    #[test]
    fn private_fn_maps_to_l1_enforced() {
        let out = extract("fn private_fn() -> u32 { 0 }");
        let vis = to_visibility_facts(&out);
        let node = out
            .graph
            .nodes
            .iter()
            .find(|n| n.name.as_deref() == Some("private_fn"))
            .expect("node must exist");
        let fact = vis.fact_for(&node.id).expect("fact must exist");
        assert_eq!(fact.level, LatticeLevel::L1);
        assert_eq!(fact.boundary, BoundaryKind::Enforced);
    }

    #[test]
    fn facade_reexport_maps_correctly() {
        let source = r#"
pub mod internal {
    pub(crate) fn raw_helper() -> u32 { 0 }
}
pub use internal::raw_helper;
"#;
        let out = extract(source);
        let vis = to_visibility_facts(&out);
        assert!(
            vis.facades.iter().any(|f| f.exported_name == "raw_helper"),
            "raw_helper must be in facades"
        );
    }

    #[test]
    fn test_module_facade_filtered_out() {
        // Test modules with `use super::*;` should not produce FacadeReexport
        // entries for the items they bring in, since they're private testing
        // infrastructure, not public API facades. Regression for vampiro-03s.
        let source = r#"
fn parse_line_span(input: &str) -> u32 { 0 }

pub(crate) fn source_key(doc: &str) -> u32 { 0 }

mod parse_line_span_tests {
    use super::*;
}

mod source_key_tests {
    use super::*;
}
"#;
        let out = extract(source);
        let vis = to_visibility_facts(&out);

        // Neither test module should produce a FacadeReexport
        for reexport in &vis.facades {
            assert!(
                reexport.facade_scope != "parse_line_span_tests",
                "test module facade_scope should not appear: {}",
                reexport.facade_scope
            );
            assert!(
                reexport.facade_scope != "source_key_tests",
                "test module facade_scope should not appear: {}",
                reexport.facade_scope
            );
        }
        // A real crate-root facade still works
    }

    #[test]
    fn test_module_glob_facade_not_in_reexports() {
        // The `use super::*;` in a test module should not create
        // FacadeReexport entries for the items it brings in scope.
        let source = r#"
pub fn exposed_api() -> u32 { 0 }

mod tests {
    use super::*;
}
"#;
        let out = extract(source);
        let vis = to_visibility_facts(&out);

        // The `tests` module's use-super-glob should not create a facade
        assert!(
            !vis.facades.iter().any(|f| f.facade_scope == "tests"),
            "tests module should not produce facades"
        );
    }
}
