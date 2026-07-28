//! E2E test for the modularity tracer over a real Rust frontend extraction.
//!
//! Loads the negative fixture
//! `tests/fixtures/add-core-seam-analysis/2/modularity_break.rs`, extracts a
//! CIR graph + visibility/facade data via `vampiro-rust-frontend`, maps the
//! Rust-specific visibility/facade data to language-neutral `VisibilityFacts`,
//! and runs the modularity tracer.
//!
//! Note: Over-exposure findings (REQ-V4) now require a cross-file caller
//! (vampiro-6ty). In the single-file fixture, the `_helper` function has no
//! cross-file callers, so it is not flagged. The facade-leak check (REQ-V7)
//! is unaffected.

use std::path::Path;

use vampiro_rust_frontend::visibility_adapter::to_visibility_facts;
use vampiro_rust_frontend::RustFrontend;
use vampiro_seam_analysis::{modularity::ModularityAnalyzer, Axis, Evidence};

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

    let vis = to_visibility_facts(&out);
    let (findings, _diags) = ModularityAnalyzer::new().analyze(&out.graph, &vis);

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
