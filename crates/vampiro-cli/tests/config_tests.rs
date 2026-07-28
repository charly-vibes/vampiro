#[test]
fn rust_cli_foundation_2_config_discovery_project_local() {
    // Project-local .vampiro/config.toml should be discovered
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join(".vampiro");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "scan-threads = 4\n").unwrap();

    let config = vampiro_cli::config::load_config(Some(dir.path()))
        .expect("config should load from project-local dir");
    assert_eq!(config.scan_threads, Some(4));
}

#[test]
fn rust_cli_foundation_2_config_discovery_xdg_fallback() {
    // XDG config should be used when no project-local config exists
    let dir = tempfile::tempdir().unwrap();
    let xdg_home = dir.path().join("xdg");
    std::fs::create_dir_all(&xdg_home.join("vampiro")).unwrap();
    std::fs::write(
        xdg_home.join("vampiro").join("config.toml"),
        "scan-threads = 2\n",
    )
    .unwrap();

    let config = vampiro_cli::config::load_config_with_xdg(Some(dir.path()), Some(&xdg_home))
        .expect("config should load from XDG fallback");
    assert_eq!(config.scan_threads, Some(2));
}

#[test]
fn rust_cli_foundation_2_config_precedence_project_overrides_xdg() {
    // Project-local config should override XDG config
    let dir = tempfile::tempdir().unwrap();

    // XDG config
    let xdg_home = dir.path().join("xdg");
    std::fs::create_dir_all(&xdg_home.join("vampiro")).unwrap();
    std::fs::write(
        xdg_home.join("vampiro").join("config.toml"),
        "scan-threads = 2\n",
    )
    .unwrap();

    // Project-local config
    let config_dir = dir.path().join(".vampiro");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "scan-threads = 8\n").unwrap();

    let config = vampiro_cli::config::load_config_with_xdg(Some(dir.path()), Some(&xdg_home))
        .expect("config should load with project-local override");
    assert_eq!(config.scan_threads, Some(8));
}

#[test]
fn rust_cli_foundation_2_config_invalid_format() {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join(".vampiro");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "invalid [[[\n").unwrap();

    let result = vampiro_cli::config::load_config(Some(dir.path()));
    assert!(result.is_err(), "invalid config should return error");
}

#[test]
fn rust_cli_foundation_2_config_not_found_uses_defaults() {
    // When no config exists, defaults should be used without error
    let dir = tempfile::tempdir().unwrap();
    let config = vampiro_cli::config::load_config(Some(dir.path()))
        .expect("no config should use defaults, not error");
    assert_eq!(config.scan_threads, None);
}
