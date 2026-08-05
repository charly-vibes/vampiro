use genesis::config::ConfigFile;

#[test]
fn rust_cli_foundation_2_config_discovery_project_local() {
    // Project-local .vampiro/config.toml should be readable through ConfigFile
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join(".vampiro");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "# empty config\n").unwrap();

    let config = vampiro_cli::config::Config::read(dir.path())
        .expect("config should load from project-local dir");
    // Empty config loads successfully
    let _ = config;
}

#[test]
fn rust_cli_foundation_2_config_not_found_returns_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let result = vampiro_cli::config::Config::read(dir.path());
    match result {
        Err(genesis::config::ConfigError::MissingFile { .. }) => {} // expected
        other => panic!("expected MissingFile, got: {:?}", other),
    }
}

#[test]
fn rust_cli_foundation_2_config_invalid_format() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join(".vampiro");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "invalid [[[\n").unwrap();

    let result = vampiro_cli::config::Config::read(dir.path());
    assert!(result.is_err(), "invalid config should return error");
    match result.unwrap_err() {
        genesis::config::ConfigError::ParseError { .. } => {} // expected
        other => panic!("expected ParseError, got: {:?}", other),
    }
}

#[test]
fn rust_cli_foundation_2_config_not_found_uses_defaults() {
    // When no config file exists, Config::default() provides sensible defaults
    let config = vampiro_cli::config::Config::default();
    let _ = config;
}

#[test]
fn rust_cli_foundation_2_config_via_store() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join(".vampiro");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "# empty config\n").unwrap();

    let store = vampiro_cli::config::vampiro_config_store();
    let config: vampiro_cli::config::Config = store
        .get("vampiro", dir.path())
        .expect("config should load via ConfigStore");
    let _ = config;
}