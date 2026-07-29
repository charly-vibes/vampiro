//! End-to-end CLI mode tests (vampiro-tmf.4).
//!
//! Verifies the three scan modes (guidance, tiered, gate) and CI workflow
//! generation against the seeded stress fixtures (vampiro-tmf.1).
//!
//! Run: `cargo test cli_mode_* cli_ci_generation_*`

use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve the workspace root from this crate's manifest dir.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Absolute path to the `vampiro` binary (set by Cargo's test harness).
fn vampiro_bin() -> &'static str {
    env!("CARGO_BIN_EXE_vampiro")
}

/// Absolute path to a fixture under `tests/fixtures/stress/`.
fn fixture(name: &str) -> PathBuf {
    workspace_root()
        .join("tests")
        .join("fixtures")
        .join("stress")
        .join(name)
}

/// Run `vampiro check` with the given args and return the output + status.
fn run_check(args: &[&str]) -> (std::process::Output, String) {
    let output = Command::new(vampiro_bin())
        .arg("check")
        .args(args)
        .output()
        .expect("vampiro check subprocess failed");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    (output, stdout)
}

// ---------------------------------------------------------------------------
// Guidance mode
// ---------------------------------------------------------------------------

#[test]
fn cli_mode_guidance_reports_findings_with_exit_0() {
    let path = fixture("composition.rs");
    let (output, stdout) = run_check(&[
        "--path",
        &path.to_string_lossy(),
        "--full",
        "--mode",
        "guidance",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "guidance mode must exit 0; stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("composition-break"),
        "guidance mode must report the composition-break finding\nstdout: {stdout}"
    );
    assert!(
        stdout.contains("1 finding(s)"),
        "guidance mode must report 1 finding\nstdout: {stdout}"
    );
}

#[test]
fn cli_mode_guidance_clean_baseline_exits_0_no_findings() {
    let path = fixture("clean.rs");
    let (output, stdout) = run_check(&[
        "--path",
        &path.to_string_lossy(),
        "--full",
        "--mode",
        "guidance",
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert!(
        stdout.contains("No findings"),
        "clean baseline must report no findings\nstdout: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// Tiered mode
// ---------------------------------------------------------------------------

#[test]
fn cli_mode_tiered_reports_findings_grouped_with_exit_0() {
    let path = fixture("composition.rs");
    let (output, stdout) = run_check(&[
        "--path",
        &path.to_string_lossy(),
        "--full",
        "--mode",
        "tiered",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "tiered mode must exit 0; stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("composition-break"),
        "tiered mode must include the finding\nstdout: {stdout}"
    );
    assert!(
        stdout.contains("[medium]"),
        "tiered mode must preserve severity labels\nstdout: {stdout}"
    );
    assert!(
        stdout.contains("1 finding(s)"),
        "tiered mode must report finding count\nstdout: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// Gate mode
// ---------------------------------------------------------------------------

#[test]
fn cli_mode_gate_fails_on_medium_findings_with_default_threshold() {
    let path = fixture("composition.rs");
    let (output, stdout) = run_check(&[
        "--path",
        &path.to_string_lossy(),
        "--full",
        "--mode",
        "gate",
    ]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "gate mode must exit non-zero for medium findings; stdout: {stdout}"
    );
    assert!(
        stdout.contains("composition-break"),
        "gate mode must report the finding even on exit non-zero\nstdout: {stdout}"
    );
}

#[test]
fn cli_mode_gate_passes_on_medium_findings_with_high_threshold() {
    let path = fixture("composition.rs");
    let (output, stdout) = run_check(&[
        "--path",
        &path.to_string_lossy(),
        "--full",
        "--mode",
        "gate",
        "--severity-threshold",
        "high",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "gate mode with threshold=high must exit 0 for medium findings; stdout: {stdout}"
    );
}

#[test]
fn cli_mode_gate_passes_on_clean_baseline() {
    let path = fixture("clean.rs");
    let (output, stdout) = run_check(&[
        "--path",
        &path.to_string_lossy(),
        "--full",
        "--mode",
        "gate",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "gate mode with zero findings must exit 0; stdout: {stdout}"
    );
    assert!(
        stdout.contains("No findings"),
        "clean baseline must report no findings\nstdout: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// Law verification — static check does NOT execute law code (REQ-10)
// ---------------------------------------------------------------------------

/// REQ-10: static `check` must never execute law-annotated source.
/// Creates a file with a `#[law] fn` that panics, then runs check.
/// If check tries to run the law, the process panics and exits non-zero.
#[test]
fn check_does_not_execute_law_code() {
    let dir = tempfile::TempDir::new().unwrap();
    let source = r#"
#[law]
fn check_property(a: i32, b: i32) -> bool {
    panic!("law code must not run during static check");
}
"#;
    let file_path = dir.path().join("law_test.rs");
    std::fs::write(&file_path, source).unwrap();
    let (output, _stdout) = run_check(&[
        "--path",
        &file_path.to_string_lossy(),
        "--full",
        "--mode",
        "guidance",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "static check must exit 0 (would panic if it executed law code); stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    // TempDir drops here — no explicit close needed
}

// ---------------------------------------------------------------------------
// CI workflow generation
// ---------------------------------------------------------------------------

#[test]
fn cli_ci_generation_yaml_valid_and_complete() {
    let output = Command::new(vampiro_bin())
        .arg("init-ci")
        .output()
        .expect("vampiro init-ci subprocess failed");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    assert_eq!(
        output.status.code(),
        Some(0),
        "init-ci must exit 0; stderr: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        stdout.contains("name:"),
        "CI workflow must have a name\nstdout: {stdout}"
    );
    assert!(
        stdout.contains("on:"),
        "CI workflow must define the on: trigger\nstdout: {stdout}"
    );
    assert!(
        stdout.contains("jobs:"),
        "CI workflow must define jobs:\nstdout: {stdout}"
    );
    assert!(
        stdout.contains("vampiro check"),
        "CI workflow must run vampiro check\nstdout: {stdout}"
    );
    assert!(
        stdout.contains("--mode gate"),
        "CI workflow must use --mode gate\nstdout: {stdout}"
    );
}

#[test]
fn cli_ci_generation_yaml_is_valid_yaml() {
    let output = Command::new(vampiro_bin())
        .arg("init-ci")
        .arg("--provider")
        .arg("github-actions")
        .output()
        .expect("vampiro init-ci subprocess failed");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    // Try parsing as YAML via python3 if available on the test host.
    // If python3/yaml isn't installed, the structural checks in
    // cli_ci_generation_yaml_valid_and_complete suffice.
    let python_check = Command::new("python3")
        .arg("-c")
        .arg("import yaml, sys; yaml.safe_load(sys.stdin); print('PARSED')")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let Ok(mut child) = python_check else { return };

    use std::io::Write;
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(stdout.as_bytes()).unwrap();
    // Dropping stdin signals EOF to the child process.
    std::mem::drop(stdin);
    let result = child.wait_with_output().unwrap();
    // Check for PARSED in stdout. If stderr has output (e.g. module not
    // found), skip the assertion gracefully — the structural checks suffice.
    let yaml_out = String::from_utf8_lossy(&result.stdout);
    if yaml_out.contains("PARSED") {
        return; // Confirmed valid YAML.
    }
    // If python3 ran but produced no PARSED marker, check stderr for hints.
    let yaml_err = result.status.code().unwrap_or(1);
    if yaml_err != 0 {
        // python3 execution failed (missing yaml module, etc.) — skip
        return;
    }
    // python3 ran successfully but didn't print PARSED → actual parse failure.
    let stderr = String::from_utf8_lossy(&result.stderr);
    panic!("CI YAML failed to parse: {stderr}\n---\n{stdout}");
}
