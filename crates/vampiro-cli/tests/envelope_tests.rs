use std::process::Command;

const TEST_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/minimal.rs");

#[test]
fn rust_cli_foundation_2_envelope_json_top_level_keys() {
    // Verify `vampiro check --json --path <file>` outputs a Genesis envelope
    // with the required top-level keys and findings under `data`.
    let output = Command::new(env!("CARGO_BIN_EXE_vampiro"))
        .arg("check")
        .arg("--json")
        .arg("--path")
        .arg(TEST_FIXTURE)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "check --json should succeed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");

    // Top-level envelope keys
    let required_keys = [
        "ok",
        "envelope_version",
        "cli_version",
        "envelope_kind",
        "data",
        "warnings",
        "meta",
    ];
    for key in &required_keys {
        assert!(
            parsed.get(*key).is_some(),
            "JSON missing envelope key '{key}': {stdout}"
        );
    }

    // Envelope structure
    assert_eq!(parsed["ok"], true, "check should succeed");
    assert_eq!(parsed["envelope_kind"], "check", "kind should be 'check'");
    assert!(
        parsed["data"].is_array(),
        "data should be an array of findings"
    );
    assert!(parsed["warnings"].is_array(), "warnings should be an array");
    assert!(parsed["meta"].is_object(), "meta should be an object");
}
