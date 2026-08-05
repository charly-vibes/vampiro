use std::path::{Path, PathBuf};

use genesis::config::{ConfigError, ConfigFile, ConfigRegistry, ConfigStore, ConfigValidation};
use serde::{Deserialize, Serialize};

/// Vampiro configuration loaded from TOML files.
///
/// Config is loaded from `.vampiro/config.toml` relative to the repo root.
///
/// Implements `genesis::config::ConfigFile` for shared suite-wide config
/// discovery and validation via `ConfigStore`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {}

// ── genesis::config::ConfigFile adoption ──────────────────────────────
//
// vampiro adopts `genesis::config` for shared config I/O: `read`/`parse`
// come from the trait's blanket impl (delegated to genesis), and `validate`
// is a no-op for the current single-field config. Registering with a
// `ConfigRegistry` enables suite-wide discovery and `ConfigStore` validation.

impl ConfigFile for Config {
    fn path(repo_root: &Path) -> PathBuf {
        repo_root.join(".vampiro").join("config.toml")
    }

    fn validate(&self) -> Result<Vec<ConfigValidation>, ConfigError> {
        Ok(Vec::new())
    }
}

/// Build a [`ConfigStore`] registering vampiro's config struct.
///
/// Registers `Config` under the `vampiro` tool name with the
/// `.vampiro/config.toml` marker so suite tools (doctor, managed block
/// generation) can discover and validate it alongside other suite configs.
pub fn vampiro_config_store() -> ConfigStore {
    let mut registry = ConfigRegistry::new();
    registry.register::<Config>("vampiro", ".vampiro/config.toml");
    ConfigStore::new(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use genesis::config::ConfigFile;

    #[test]
    fn config_path_is_dot_vampiro_config_toml_relative_to_root() {
        let dir = tempfile::tempdir().unwrap();
        let path = Config::path(dir.path());
        assert_eq!(path, dir.path().join(".vampiro").join("config.toml"));
    }

    #[test]
    fn config_not_found_returns_missing_file_error() {
        let dir = tempfile::tempdir().unwrap();
        let result = Config::read(dir.path());
        assert!(result.is_err());
        match result.unwrap_err() {
            genesis::config::ConfigError::MissingFile { .. } => {} // expected
            e => panic!("expected MissingFile, got: {:?}", e),
        }
    }

    #[test]
    fn config_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".vampiro");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.toml"), "# empty config\n").unwrap();

        let config = Config::read(dir.path()).unwrap();
        // Empty config — only verifies the file roundtrips through genesis
        let _ = config;
    }

    #[test]
    fn config_invalid_format_returns_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".vampiro");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.toml"), "invalid [[[\n").unwrap();

        let result = Config::read(dir.path());
        assert!(result.is_err());
        match result.unwrap_err() {
            genesis::config::ConfigError::ParseError { .. } => {} // expected
            e => panic!("expected ParseError, got: {:?}", e),
        }
    }

    #[test]
    fn config_default_is_empty() {
        let config = Config::default();
        // Default config has no fields; verify it roundtrips
        assert!(serde_json::to_string(&config).is_ok());
    }

    #[test]
    fn config_validate_default_returns_empty() {
        let config = Config::default();
        let results = config.validate().unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn vampiro_config_store_registers_vampiro() {
        let store = vampiro_config_store();
        assert!(store.registry().is_registered("vampiro"));
        assert_eq!(
            store.registry().marker("vampiro"),
            Some(".vampiro/config.toml")
        );
    }

    #[test]
    fn vampiro_config_store_discovers_missing_config() {
        let dir = tempfile::tempdir().unwrap();
        let store = vampiro_config_store();
        let discovered = ConfigStore::discover(dir.path(), store.registry());
        assert!(discovered
            .iter()
            .any(|d| d.tool_name == "vampiro" && !d.found));
    }

    #[test]
    fn vampiro_config_store_discovers_present_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".vampiro");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.toml"), "# config\n").unwrap();

        let store = vampiro_config_store();
        let discovered = ConfigStore::discover(dir.path(), store.registry());
        assert!(discovered
            .iter()
            .any(|d| d.tool_name == "vampiro" && d.found));
    }

    #[test]
    fn config_works_with_store_get() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".vampiro");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.toml"), "# empty\n").unwrap();

        let store = vampiro_config_store();
        let config: Config = store.get("vampiro", dir.path()).unwrap();
        // Empty config loads successfully through ConfigStore
        let _ = config;
    }
}
