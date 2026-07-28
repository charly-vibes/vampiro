use std::process::Command;

#[test]
fn rust_cli_foundation_2_exit_success() {
    let output = Command::new(env!("CARGO_BIN_EXE_vampiro"))
        .arg("--help")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "exit 0 for help");
}

#[test]
fn rust_cli_foundation_2_exit_usage_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_vampiro"))
        .arg("--nonexistent-flag")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "exit 2 for usage error");
}

#[test]
fn rust_cli_foundation_2_exit_invalid_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join(".vampiro");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "invalid toml [[[").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_vampiro"))
        .current_dir(dir.path())
        .arg("--help")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "exit 1 for invalid config");
}
