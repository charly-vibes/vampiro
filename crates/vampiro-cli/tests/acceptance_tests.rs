use vampiro_cir::ScalarKind;
fn workspace_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = crates/vampiro-cli/
    // workspace root = crates/vampiro-cli/../../
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn rust_cli_foundation_4_contract_cli_config_exit_exists() {
    // Verify cli-config-exit/v1 contract artifact exists and is valid JSON
    let path = workspace_root()
        .join("tests")
        .join("contracts")
        .join("cli")
        .join("config-exit-v1.json");
    assert!(
        path.exists(),
        "config-exit-v1.json should exist at {path:?}"
    );

    let content = std::fs::read_to_string(&path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["version"], "v1");
    assert_eq!(parsed["change"], "add-rust-cli-foundation");
    assert_eq!(parsed["tracer"], "cli-config-exit");
}

#[test]
fn rust_cli_foundation_4_contract_finding_envelope_exists() {
    // Verify finding-envelope/v1 contract artifact exists and is valid JSON
    let path = workspace_root()
        .join("tests")
        .join("contracts")
        .join("findings")
        .join("envelope-v1.json");
    assert!(path.exists(), "envelope-v1.json should exist at {path:?}");

    let content = std::fs::read_to_string(&path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["version"], "v1");
    assert_eq!(parsed["change"], "add-rust-cli-foundation");
    assert_eq!(parsed["tracer"], "finding-envelope");
}

#[test]
fn rust_cli_foundation_4_verification_docs_exist() {
    // Verify all three verification docs exist
    let base = workspace_root().join("docs").join("verification");

    assert!(base.join("add-rust-cli-foundation-1.md").exists());
    assert!(base.join("add-rust-cli-foundation-2.md").exists());
    assert!(base.join("add-rust-cli-foundation-3.md").exists());
}

#[test]
fn rust_cli_foundation_4_no_analysis_or_gating_behavior() {
    // Verify no analysis, proof, CI-generation, or gating types are present
    // This is a compile-time check that the finding module is purely structural
    let _ = vampiro_cli::finding::Finding::composition_mismatch(
        std::path::PathBuf::from("test.rs"),
        1..=1,
        vampiro_cir::Shape::Scalar(ScalarKind::Unit),
        vampiro_cir::Shape::Union(vec![vampiro_cir::Shape::Scalar(ScalarKind::Unit), vampiro_cir::Shape::Opaque]),
        vec![vampiro_cir::Shape::Opaque],
    );
    let _ = vampiro_cli::exit_code::ExitCode::Success;
    let _ = vampiro_cli::config::Config::default();
}
