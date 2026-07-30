//! Vampiro status contributor for cross-tool dashboard.
//!
//! Implements [`StatusContributor`](genesis::status::StatusContributor) to
//! surface vampiro's config state, scan history, and doctor health in the
//! unified suite dashboard.

use std::path::Path;

use genesis::status::{StatusContributor, StatusItem, StatusSection};

use genesis::config::ConfigFile;
use crate::config::Config;

/// Vampiro's status contributor.
#[derive(Debug, Clone, Copy)]
pub struct VampiroStatus;

impl StatusContributor for VampiroStatus {
    fn name(&self) -> &'static str {
        "vampiro"
    }
    fn status(&self, repo: &Path) -> Result<StatusSection, String> {
        let mut items = Vec::new();

        // Config state.
        let config_path = repo.join(".vampiro").join("config.toml");
        if config_path.exists() {
            match Config::read_from(&config_path) {
                Ok(_) => {
                    items.push(StatusItem::healthy("Config", "valid"));
                }
                Err(e) => {
                    items.push(StatusItem::error(
                        "Config",
                        format!("parse error: {e}"),
                    ));
                }
            }
        } else {
            items.push(StatusItem::healthy("Config", "not found (using defaults)"));
        }

        // Doctor health.
        let checks = crate::doctor::default_checks();
        let runner = genesis::doctor::DoctorRunner::new(checks).with_tool_name("vampiro");
        match runner.run(repo, false) {
            Ok(report) => {
                if report.summary.has_failures() {
                    items.push(StatusItem::error(
                        "Doctor",
                        format!("{} failures", report.summary.fail),
                    ));
                } else if report.summary.has_issues() {
                    items.push(StatusItem::warning(
                        "Doctor",
                        format!("{} warnings", report.summary.warn),
                    ));
                } else {
                    items.push(StatusItem::healthy("Doctor", "all checks pass"));
                }
            }
            Err(e) => {
                items.push(StatusItem::error("Doctor", format!("error: {e}")));
            }
        }

        // Git context.
        match git2::Repository::open(repo) {
            Ok(_) => {
                items.push(StatusItem::healthy("Git", "repository found"));
            }
            Err(_) => {
                items.push(StatusItem::warning("Git", "not a repository"));
            }
        }

        Ok(StatusSection::with_items("vampiro", "vampiro status", items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_contributor_name() {
        let status = VampiroStatus;
        assert_eq!(status.name(), "vampiro");
    }

    #[test]
    fn status_returns_section() {
        let dir = std::env::temp_dir().join("vampiro-status-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let status = VampiroStatus;
        let section = status.status(&dir).unwrap();
        assert_eq!(section.summary, "vampiro status");
        assert_eq!(section.items.len(), 3);
        assert!(!section.items.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}