//! E2E test for the modularity tracer over a real Rust frontend extraction.
//!
//! Loads the negative fixture
//! `tests/fixtures/add-core-seam-analysis/2/modularity_break.rs`, extracts a
//! CIR graph + visibility/facade data via `vampiro-rust-frontend`, maps the
//! Rust-specific visibility/facade data to language-neutral `VisibilityFacts`,
//! and runs the modularity tracer. Asserts that over-exposure (REQ-V4) and
//! facade-leak (REQ-V7) findings are produced with the required fields.

use std::path::Path;

use vampiro_rust_frontend::{ExtractionOutput, RustFrontend, Visibility};
use vampiro_seam_analysis::{
    modularity::ModularityAnalyzer, Axis, BoundaryKind, Evidence, FacadeReexport, LatticeLevel,
    VisibilityFact, VisibilityFacts,
};

/// Resolve the fixture path relative to the workspace root.
fn fixture_path() -> String {
    let manifest_dir = std::env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/add-core-seam-analysis/2/modularity_break.rs");
    path.to_string_lossy().to_string()
}

/// Map the Rust frontend's extraction output to language-neutral visibility
/// facts. This is the Rust idiom-table mapping (Addendum V, per-language
/// table) implemented as an adapter so the analysis crate does not depend on
/// the Rust frontend at runtime.
fn map_visibility(out: &ExtractionOutput) -> VisibilityFacts {
    let mut facts = VisibilityFacts::new(Visibility::TABLE_VERSION);

    // Map each node's Rust visibility to a lattice level + boundary kind.
    for (node_id, vis) in &out.visibility {
        let node = match out.graph.node_by_id(node_id) {
            Some(n) => n,
            None => continue,
        };
        // Derive scope from source_file + module context (simplified: use the
        // source file path as the scope for E2E purposes).
        let scope = node.span.file.clone();

        let (level, boundary, internal) = match vis {
            Visibility::Public => {
                // Check if this node is in the crate-root facade.
                let in_facade = out.facades.iter().any(|fd| {
                    fd.module_path.is_empty()
                        && fd
                            .entries
                            .iter()
                            .any(|e| e.name == node.name.clone().unwrap_or_default())
                });
                let doc_hidden = out
                    .facades
                    .iter()
                    .flat_map(|fd| &fd.entries)
                    .any(|e| e.doc_hidden && e.name == node.name.clone().unwrap_or_default());
                let leading_underscore = node.name.as_ref().is_some_and(|n| n.starts_with('_'));
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

    // Map facades to language-neutral re-exports.
    for fd in &out.facades {
        for entry in &fd.entries {
            // Find the underlying node by matching the re-exported name.
            if let Some(node) = out
                .graph
                .nodes
                .iter()
                .find(|n| n.name.as_ref().is_some_and(|n| n == &entry.name))
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

#[test]
fn modularity_e2e_over_exposure_and_facade_leak() {
    let path = fixture_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));

    let out = RustFrontend
        .extract_full(
            &source,
            Path::new("tests/fixtures/add-core-seam-analysis/2/modularity_break.rs"),
        )
        .expect("frontend extraction must succeed");

    let vis = map_visibility(&out);
    let (findings, _diags) = ModularityAnalyzer::new().analyze(&out.graph, &vis);

    // REQ-V4: _helper is pub + leading-underscore → over-exposure.
    let over_exposure: Vec<_> = findings
        .iter()
        .filter(|f| f.classification == "over-exposure")
        .collect();
    assert!(
        !over_exposure.is_empty(),
        "expected at least one over-exposure finding for `_helper`; got {findings:?}"
    );
    let f = &over_exposure[0];
    assert_eq!(f.rule, "REQ-V4");
    assert_eq!(f.axis, Axis::Modularity);
    let Evidence::OverExposure { target_level, .. } = &f.evidence else {
        panic!("expected over-exposure evidence");
    };
    assert_eq!(target_level, "L3");

    // REQ-V7: `pub use internal::RawTable` re-exports an L2 symbol at L4.
    let facade_leak: Vec<_> = findings
        .iter()
        .filter(|f| f.classification == "facade-leak")
        .collect();
    assert!(
        !facade_leak.is_empty(),
        "expected at least one facade-leak finding for `RawTable`; got {findings:?}"
    );
    let f = &facade_leak[0];
    assert_eq!(f.rule, "REQ-V7");
    assert_eq!(f.axis, Axis::Modularity);
    let Evidence::FacadeLeak {
        exported_name,
        underlying_level,
        ..
    } = &f.evidence
    else {
        panic!("expected facade-leak evidence");
    };
    assert_eq!(exported_name, "raw_helper");
    assert_eq!(underlying_level, "L2");
}
