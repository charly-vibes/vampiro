use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn cli_snapshots() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cmd");
    trycmd::TestCases::new()
        .case(path.join("help.toml"))
        .case(path.join("version.toml"))
        .case(path.join("check-help.toml"))
        .case(path.join("prove-help.toml"));
}

/// Smoke test: the README quickstart command must succeed.
///
/// Executes: `vampiro check --path <file> --mode guidance`
/// on a minimal Rust source file in a temp directory.
#[test]
fn quickstart_smoke_test() {
    let dir = std::env::temp_dir().join("vampiro-quickstart-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("hello.rs");
    std::fs::write(&file, "fn main() { let x = 1; }\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_vampiro"))
        .args([
            "check",
            "--path",
            &file.to_string_lossy(),
            "--mode",
            "guidance",
        ])
        .output()
        .expect("quickstart command failed to start");

    assert!(
        output.status.success(),
        "quickstart command failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Test that the CLI accepts Python files and dispatches to PythonFrontend.
#[test]
fn python_cli_accepts_py_file() {
    let dir = std::env::temp_dir().join("vampiro-python-cli-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Use the existing cross-language fixture
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("stress")
        .join("cross-language")
        .join("python")
        .join("cli.py");

    let output = Command::new(env!("CARGO_BIN_EXE_vampiro"))
        .args([
            "check",
            "--path",
            &fixture.to_string_lossy(),
            "--mode",
            "guidance",
        ])
        .output()
        .expect("python check command failed to start");

    assert!(
        output.status.success(),
        "python check failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Test that the CLI accepts a directory containing Python files.
#[test]
fn python_cli_accepts_py_directory() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("stress")
        .join("cross-language")
        .join("python");

    let output = Command::new(env!("CARGO_BIN_EXE_vampiro"))
        .args([
            "check",
            "--path",
            &fixture_dir.to_string_lossy(),
            "--mode",
            "guidance",
        ])
        .output()
        .expect("python directory check command failed to start");

    assert!(
        output.status.success(),
        "python directory check failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // Verify we got composition analysis output (stdout should mention files)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Files scanned") || stdout.contains("finding"),
        "Expected scan output, got: {stdout}"
    );
}

/// Helper: run `vampiro check --path <path>` and assert success.
fn assert_check_succeeds(label: &str, path: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_vampiro"))
        .args([
            "check",
            "--path",
            &path.to_string_lossy(),
            "--mode",
            "guidance",
        ])
        .output()
        .unwrap_or_else(|_| panic!("{label} check command failed to start"));

    assert!(
        output.status.success(),
        "{label} check failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Test that the CLI accepts Clojure files and dispatches to ClojureFrontend.
#[test]
fn clojure_cli_accepts_clj_file() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("stress")
        .join("cross-language")
        .join("clojure")
        .join("data_processing.clj");
    assert_check_succeeds("clojure file", &fixture);
}

/// Test that the CLI accepts a directory containing Clojure files.
#[test]
fn clojure_cli_accepts_clj_directory() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("stress")
        .join("cross-language")
        .join("clojure");
    assert_check_succeeds("clojure directory", &fixture_dir);
}

/// Test that the CLI accepts Julia files and dispatches to JuliaFrontend.
#[test]
fn julia_cli_accepts_jl_file() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("stress")
        .join("cross-language")
        .join("julia")
        .join("data_analysis.jl");
    assert_check_succeeds("julia file", &fixture);
}

/// Test that the CLI accepts a directory containing Julia files.
#[test]
fn julia_cli_accepts_jl_directory() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("stress")
        .join("cross-language")
        .join("julia");
    assert_check_succeeds("julia directory", &fixture_dir);
}
