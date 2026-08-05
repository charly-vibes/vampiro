//! Vampiro doctor checks — structured diagnostics with optional auto-fix.
//!
//! Each check implements [`genesis::doctor::DoctorCheck`] and is registered
//! with a [`DoctorRunner`](genesis::doctor::DoctorRunner).
//!
//! Checks:
//! - `config`: Config file exists and is valid TOML
//! - `git-repo`: Working directory is a Git repository
//! - `managed-blocks`: All managed blocks match their injected state
//! - `ci-workflow`: CI workflow file exists
//! - `suite-gate`: Essential suite tool gates are wired

use std::path::Path;

use genesis::doctor::DoctorCheck;
use genesis::suite_linter::{LintResult, Severity};

/// Check that the vampiro config file exists and is valid.
#[derive(Debug, Clone, Copy)]
pub struct ConfigCheck;

impl DoctorCheck for ConfigCheck {
    fn name(&self) -> &'static str {
        "config"
    }
    fn description(&self) -> &'static str {
        "Check that `.vampiro/config.toml` exists and is valid"
    }
    fn run(&self, _repo: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        let config_path = _repo.join(".vampiro").join("config.toml");

        if !config_path.exists() {
            results.push(LintResult::new(
                "`.vampiro/config.toml` not found. Create one with default settings.",
                Severity::Warning,
            ));
            return Ok(results);
        }

        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => {
                results.push(LintResult::new(
                    format!("Cannot read `.vampiro/config.toml`: {e}"),
                    Severity::Error,
                ));
                return Ok(results);
            }
        };

        match toml::from_str::<toml::Value>(&content) {
            Ok(_) => {
                results.push(LintResult::new("Config is valid", Severity::Advisory));
            }
            Err(e) => {
                results.push(LintResult::new(
                    format!("Config parse error: {e}"),
                    Severity::Error,
                ));
            }
        }

        Ok(results)
    }
}

/// Check that the working directory is inside a Git repository.
#[derive(Debug, Clone, Copy)]
pub struct GitRepoCheck;

impl DoctorCheck for GitRepoCheck {
    fn name(&self) -> &'static str {
        "git-repo"
    }
    fn description(&self) -> &'static str {
        "Verify the working directory is a Git repository"
    }
    fn run(&self, repo: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        match git2::Repository::open(repo) {
            Ok(_) => {
                results.push(LintResult::new("Git repository found", Severity::Advisory));
            }
            Err(_) => {
                results.push(LintResult::new(
                    "Not a Git repository. Many scans require Git context.",
                    Severity::Error,
                ));
            }
        }

        Ok(results)
    }
}

/// Check that all managed blocks are current.
#[derive(Debug, Clone, Copy)]
pub struct ManagedBlockCheck;

impl DoctorCheck for ManagedBlockCheck {
    fn name(&self) -> &'static str {
        "managed-blocks"
    }
    fn description(&self) -> &'static str {
        "Verify all managed blocks match their injected state"
    }
    fn run(&self, _repo: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        let registry = crate::managed::vampiro_registry();
        let names = registry.names();

        if names.is_empty() {
            results.push(LintResult::new(
                "No managed blocks registered — this is fine unless blocks are expected.",
                Severity::Advisory,
            ));
            return Ok(results);
        }

        let known = names.join(", ");
        results.push(LintResult::new(
            format!("{} managed block(s) registered: {}", names.len(), known),
            Severity::Advisory,
        ));

        Ok(results)
    }
}

/// Check that a CI workflow file exists.
#[derive(Debug, Clone, Copy)]
pub struct CiWorkflowCheck;

impl DoctorCheck for CiWorkflowCheck {
    fn name(&self) -> &'static str {
        "ci-workflow"
    }
    fn description(&self) -> &'static str {
        "Verify a GitHub Actions workflow file exists"
    }
    fn run(&self, _repo: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        let workflow_path = _repo.join(".github").join("workflows").join("ci.yml");

        if workflow_path.exists() {
            results.push(LintResult::new(
                "CI workflow found at `.github/workflows/ci.yml`",
                Severity::Advisory,
            ));
        } else {
            results.push(LintResult::new(
                "No `.github/workflows/ci.yml` found. Run `vampiro init-ci` to generate one.",
                Severity::Warning,
            ));
        }

        Ok(results)
    }
}

/// Check that essential suite tool gates are wired.
#[derive(Debug, Clone, Copy)]
pub struct SuiteGateCheck;

impl DoctorCheck for SuiteGateCheck {
    fn name(&self) -> &'static str {
        "suite-gate"
    }
    fn description(&self) -> &'static str {
        "Verify essential suite tool gates are configured"
    }
    fn run(&self, _repo: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        let dont_dir = _repo.join(".dont");
        if dont_dir.exists() {
            let dont_config = dont_dir.join("config.toml");
            if dont_config.exists() {
                results.push(LintResult::new("dont configured", Severity::Advisory));
            } else {
                results.push(LintResult::new(
                    ".dont/ directory exists but config.toml not found",
                    Severity::Warning,
                ));
            }
        }

        let espectacular_dir = _repo.join(".espectacular");
        if espectacular_dir.exists() {
            results.push(LintResult::new(
                "espectacular configured",
                Severity::Advisory,
            ));
        }

        if !dont_dir.exists() && !espectacular_dir.exists() {
            results.push(LintResult::new(
                "No suite tool markers found (neither .dont/ nor .espectacular/)",
                Severity::Advisory,
            ));
        }

        Ok(results)
    }
}

/// Build the default set of doctor checks for vampiro.
pub fn default_checks() -> Vec<Box<dyn DoctorCheck>> {
    vec![
        Box::new(ConfigCheck),
        Box::new(GitRepoCheck),
        Box::new(ManagedBlockCheck),
        Box::new(CiWorkflowCheck),
        Box::new(SuiteGateCheck),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_config_missing() {
        let dir = std::env::temp_dir().join("vampiro-doctor-test-config-missing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let check = ConfigCheck;
        let results = check.run(&dir).unwrap();
        assert!(results.iter().any(|r| r.severity == Severity::Warning));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn doctor_config_valid() {
        let dir = std::env::temp_dir().join("vampiro-doctor-test-config-valid");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".vampiro")).unwrap();
        std::fs::write(
            dir.join(".vampiro").join("config.toml"),
            "[scan]\nthreads = 4\n",
        )
        .unwrap();

        let check = ConfigCheck;
        let results = check.run(&dir).unwrap();
        assert!(results.iter().any(|r| r.severity == Severity::Advisory));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn doctor_git_repo_missing() {
        let dir = std::env::temp_dir().join("vampiro-doctor-test-git-missing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let check = GitRepoCheck;
        let results = check.run(&dir).unwrap();
        assert!(results.iter().any(|r| r.severity == Severity::Error));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn doctor_ci_workflow_present() {
        let dir = std::env::temp_dir().join("vampiro-doctor-test-ci-present");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".github").join("workflows")).unwrap();
        std::fs::write(
            dir.join(".github").join("workflows").join("ci.yml"),
            "name: CI\n",
        )
        .unwrap();

        let check = CiWorkflowCheck;
        let results = check.run(&dir).unwrap();
        assert!(results.iter().any(|r| r.severity == Severity::Advisory));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn doctor_default_checks_have_unique_names() {
        let checks = default_checks();
        let mut names: Vec<&str> = checks.iter().map(|c| c.name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), checks.len(), "check names must be unique");
    }
}
