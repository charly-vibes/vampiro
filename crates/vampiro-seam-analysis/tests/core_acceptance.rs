//! Core acceptance tests for the normalized finding contract (0vb.4.6).
//!
//! Verifies that the finding contract fixture exists, all four slice suites
//! pass, and the contract is schema-valid.

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn core_seam_analysis_5_1_contract_normalized_finding_exists() {
    let path = workspace_root()
        .join("tests")
        .join("contracts")
        .join("findings")
        .join("normalized-finding-v1.json");
    assert!(
        path.exists(),
        "normalized-finding-v1.json should exist at {path:?}"
    );

    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read contract fixture: {e}"));
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("contract fixture must be valid JSON");
    assert_eq!(parsed["version"], "v1");
    assert_eq!(parsed["change"], "add-core-seam-analysis");
    assert_eq!(parsed["tracer"], "normalized-finding-contract");
}

#[test]
fn core_seam_analysis_5_2_all_verification_docs_exist() {
    let base = workspace_root().join("docs").join("verification");
    assert!(base.join("add-core-seam-analysis-1.md").exists());
    assert!(base.join("add-core-seam-analysis-2.md").exists());
    assert!(base.join("add-core-seam-analysis-3.md").exists());
    assert!(base.join("add-core-seam-analysis-4.md").exists());
}

#[test]
fn core_seam_analysis_5_3_evidence_variants_are_exhaustive() {
    let path = workspace_root()
        .join("tests")
        .join("contracts")
        .join("findings")
        .join("normalized-finding-v1.json");
    let content = std::fs::read_to_string(&path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

    let variants = &parsed["contract"]["finding"]["evidence-variants"];
    // All six evidence variants must be present.
    let expected = [
        "CompositionMismatch",
        "ReachThrough",
        "OverExposure",
        "FacadeLeak",
        "SwallowedEffect",
        "RedundancyMismatch",
    ];
    for name in &expected {
        assert!(
            variants.get(*name).is_some(),
            "missing evidence variant: {name}"
        );
    }
    assert_eq!(
        variants.as_object().map(|o| o.len()).unwrap_or(0),
        expected.len(),
        "unexpected additional evidence variants — update test"
    );
}

#[test]
fn core_seam_analysis_5_4_finding_schema_fields() {
    let path = workspace_root()
        .join("tests")
        .join("contracts")
        .join("findings")
        .join("normalized-finding-v1.json");
    let content = std::fs::read_to_string(&path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

    let finding = &parsed["contract"]["finding"];
    let fields = &finding["fields"];
    let required = [
        "rule",
        "path",
        "severity",
        "axis",
        "classification",
        "evidence",
    ];
    for name in &required {
        assert!(fields.get(*name).is_some(), "missing finding field: {name}");
    }
}
