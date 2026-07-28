use std::path::Path;

/// Vampiro configuration loaded from TOML files.
///
/// Fields are optional — any unset field falls back to the built-in default.
/// Config is loaded from the first file found, in order of precedence:
/// 1. `./.vampiro/config.toml` (project-local)
/// 2. `~/.config/vampiro/config.toml` (XDG)
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Number of threads to use for scanning (default: auto-detect)
    pub scan_threads: Option<u32>,
}

/// Errors that can occur during config loading.
#[derive(Debug)]
pub enum ConfigError {
    /// TOML parse error
    InvalidFormat(String),
    /// I/O error reading the file
    IoError(std::io::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidFormat(msg) => write!(f, "invalid config format: {msg}"),
            ConfigError::IoError(err) => write!(f, "config I/O error: {err}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Load configuration from the project-local or XDG config directory.
///
/// `project_root` is typically the current working directory. If `None`,
/// uses `std::env::current_dir()`.
///
/// Returns `Ok(Config::default())` if no config file is found — missing
/// config is not an error.
pub fn load_config(project_root: Option<&Path>) -> Result<Config, ConfigError> {
    let root = project_root.map_or_else(
        || std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
        |p| p.to_path_buf(),
    );

    // 1. Try project-local config
    let project_config = root.join(".vampiro").join("config.toml");
    if project_config.exists() {
        return load_from_file(&project_config);
    }

    // 2. Try XDG config
    if let Some(xdg_path) = xdg_config_path() {
        if xdg_path.exists() {
            return load_from_file(&xdg_path);
        }
    }

    // 3. No config found — use defaults
    Ok(Config::default())
}

/// Read and parse a TOML config file from the given path.
fn load_from_file(path: &Path) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path).map_err(ConfigError::IoError)?;
    toml::from_str(&content).map_err(|e| ConfigError::InvalidFormat(e.to_string()))
}

/// Resolve the XDG config path for vampiro.
///
/// Uses `$XDG_CONFIG_HOME/vampiro/config.toml` if set and non-empty,
/// otherwise falls back to `~/.config/vampiro/config.toml`.
/// Returns `None` if `HOME` is unset and `XDG_CONFIG_HOME` is not provided.
fn xdg_config_path() -> Option<std::path::PathBuf> {
    // Use XDG_CONFIG_HOME if set to a non-empty value, otherwise ~/.config
    let base = if let Ok(val) = std::env::var("XDG_CONFIG_HOME") {
        if val.is_empty() {
            let home = std::env::var("HOME").ok()?;
            std::path::PathBuf::from(home).join(".config")
        } else {
            std::path::PathBuf::from(val)
        }
    } else {
        let home = std::env::var("HOME").ok()?;
        std::path::PathBuf::from(home).join(".config")
    };
    Some(base.join("vampiro").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_not_found_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config = load_config(Some(dir.path())).unwrap();
        assert_eq!(config.scan_threads, None);
    }

    #[test]
    fn config_none_root_uses_cwd() {
        // Calling load_config(None) should not panic and should return defaults
        let config = load_config(None).unwrap();
        assert_eq!(config.scan_threads, None);
    }

    #[test]
    fn config_loads_project_local() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".vampiro");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.toml"), "scan-threads = 4\n").unwrap();

        let config = load_config(Some(dir.path())).unwrap();
        assert_eq!(config.scan_threads, Some(4));
    }

    #[test]
    fn config_invalid_format_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".vampiro");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.toml"), "invalid [[[\n").unwrap();

        let result = load_config(Some(dir.path()));
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::InvalidFormat(_) => {} // expected
            other => panic!("expected InvalidFormat, got {other:?}"),
        }
    }
}
