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