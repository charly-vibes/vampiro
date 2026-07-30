use std::path::{Path, PathBuf};

use clap::Parser;
use vampiro_rust_frontend::visibility_adapter::to_visibility_facts;
use vampiro_rust_frontend::RustFrontend;
use vampiro_seam_analysis::analyze_with_visibility;

use crate::exit_code::ExitCode;
use crate::output::ScanResult;
use crate::output::ScanResultMetadata;
use crate::output::ScopeKind;
use crate::policy::{generate_github_actions_workflow, ScanMode, ScanPolicy};
use crate::scan::GitContext;

/// A program analysis tool for verifying compliance with laws and policies.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Parser, Debug)]
pub enum Commands {
    /// Analyze source files for composition, modularity, and robustness breaks
    Check(CheckArgs),
    /// Generate CI workflow configuration
    InitCi {
        /// CI provider (default: github-actions)
        #[arg(long, default_value = "github-actions")]
        provider: String,
    },
    /// Reserved for proof commands
    Prove {
        #[command(subcommand)]
        command: Option<ProveCommands>,
    },
    /// Run diagnostic checks on the project
    Doctor {
        /// Auto-fix issues when possible
        #[arg(long)]
        fix: bool,
    },
}

#[derive(Parser, Debug)]
pub struct CheckArgs {
    /// Path(s) to Rust source file(s) or directories to scan
    #[arg(long, short)]
    pub path: Vec<PathBuf>,

    /// Explicit target revision (commit SHA or ref) for diff scope
    #[arg(long)]
    pub target: Option<String>,

    /// Base revision (commit SHA or ref) for diff scope
    #[arg(long)]
    pub base: Option<String>,

    /// Scan all files (full scope) instead of diff
    #[arg(long)]
    pub full: bool,

    /// Scan mode: guidance, tiered, or gate
    #[arg(long, default_value = "guidance")]
    pub mode: String,

    /// Severity threshold for gate mode (low, medium, high)
    #[arg(long)]
    pub severity_threshold: Option<String>,

    /// Output findings as JSON
    #[arg(long, short)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub enum ProveCommands {}

impl Cli {
    pub fn run(&self) -> ExitCode {
        match &self.command {
            Some(Commands::Check(args)) => run_check(args),
            Some(Commands::InitCi { provider }) => run_init_ci(provider),
            Some(Commands::Doctor { fix }) => run_doctor(*fix),
            _ => ExitCode::Success,
        }
    }
}

fn run_doctor(fix: bool) -> ExitCode {
    use genesis::doctor::DoctorRunner;

    let runner = DoctorRunner::new(crate::doctor::default_checks());
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    match runner.run(&cwd, fix) {
        Ok(report) => {
            println!("vampiro doctor — {} pass, {} warn, {} fail",
                report.summary.pass, report.summary.warn, report.summary.fail);

            for entry in &report.checks {
                let icon = match entry.status {
                    genesis::doctor::CheckStatus::Pass => "✅",
                    genesis::doctor::CheckStatus::Warn => "⚠️",
                    genesis::doctor::CheckStatus::Fail => "❌",
                };
                println!("  {} {} — {}", icon, entry.name, entry.message);
            }

            if report.summary.fail > 0 {
                ExitCode::PolicyFailure
            } else {
                ExitCode::Success
            }
        }
        Err(e) => {
            eprintln!("vampiro doctor: {e}");
            ExitCode::InternalError
        }
    }
}

fn run_init_ci(provider: &str) -> ExitCode {
    match provider {
        "github-actions" | "gha" => {
            let policy = ScanPolicy::default();
            match generate_github_actions_workflow(&policy) {
                Ok(workflow) => {
                    println!("{}", workflow);
                    ExitCode::Success
                }
                Err(e) => {
                    eprintln!("vampiro: failed to generate CI workflow: {e}");
                    ExitCode::InternalError
                }
            }
        }
        _ => {
            eprintln!("vampiro: unsupported CI provider: {provider}");
            ExitCode::UsageError
        }
    }
}

/// Collect all .rs files from the given paths, expanding directories recursively.
fn collect_rs_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            collect_dir(path, &mut files)?;
        } else if path.is_file() {
            if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                files.push(path.clone());
            } else {
                return Err(format!("not a .rs file: {}", path.display()));
            }
        } else {
            return Err(format!("path not found: {}", path.display()));
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_dir(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        std::fs::read_dir(dir).map_err(|e| format!("failed to read dir {}: {e}", dir.display()))?
    {
        let entry = entry.map_err(|e| format!("failed to read entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_dir(&path, files)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn scan_files(
    files: &[PathBuf],
) -> (
    Vec<vampiro_seam_analysis::Finding>,
    Vec<vampiro_seam_analysis::Diagnostic>,
) {
    let mut scanned_findings: Vec<vampiro_seam_analysis::Finding> = Vec::new();
    let mut scanned_diagnostics: Vec<vampiro_seam_analysis::Diagnostic> = Vec::new();

    for file in files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("vampiro: failed to read {}: {e}", file.display());
                continue;
            }
        };

        let out = match RustFrontend.extract_full(&source, file) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("vampiro: extraction failed for {}: {e}", file.display());
                continue;
            }
        };

        let vis = to_visibility_facts(&out);
        let (findings, diagnostics) = analyze_with_visibility(&out.graph, &vis);

        scanned_findings.extend(findings);
        scanned_diagnostics.extend(diagnostics);
    }

    (scanned_findings, scanned_diagnostics)
}

fn run_check(args: &CheckArgs) -> ExitCode {
    // Resolve scan scope.
    let files: Vec<PathBuf> = if !args.path.is_empty() {
        // Explicit --path: use as-is
        match collect_rs_files(&args.path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("vampiro: {e}");
                return ExitCode::UsageError;
            }
        }
    } else if args.full {
        // Full scope via Git
        match GitContext::open_from_cwd() {
            Ok(ctx) => match ctx.full_scope() {
                Ok(scope) => scope.files().to_vec(),
                Err(e) => {
                    eprintln!("vampiro: failed to resolve full scope: {e}");
                    return ExitCode::InternalError;
                }
            },
            Err(_) => {
                eprintln!("vampiro: not a Git repository. Use --path to specify files.");
                return ExitCode::UsageError;
            }
        }
    } else if let Some(target) = &args.target {
        // Explicit diff scope
        let base = args.base.as_deref().unwrap_or("HEAD~1");
        match GitContext::open_from_cwd() {
            Ok(ctx) => match ctx.diff_between(base, target) {
                Ok(scope) => scope.files().to_vec(),
                Err(e) => {
                    eprintln!("vampiro: diff scope error: {e}");
                    return ExitCode::InternalError;
                }
            },
            Err(_) => {
                eprintln!("vampiro: not a Git repository. Use --path to specify files.");
                return ExitCode::UsageError;
            }
        }
    } else {
        // Default: local diff (HEAD vs worktree)
        match GitContext::open_from_cwd() {
            Ok(ctx) => match ctx.local_diff() {
                Ok(scope) => scope.files().to_vec(),
                Err(e) => {
                    eprintln!("vampiro: local diff error: {e}");
                    return ExitCode::InternalError;
                }
            },
            Err(_) => {
                eprintln!("vampiro: not a Git repository, and no --path or --target given");
                return ExitCode::UsageError;
            }
        }
    };

    if files.is_empty() {
        eprintln!("vampiro: no files to scan");
        return ExitCode::Success;
    }

    let (scanned_findings, scanned_diagnostics) = scan_files(&files);

    let metadata = ScanResultMetadata {
        scope: if args.full || args.path.is_empty() {
            ScopeKind::Full
        } else {
            ScopeKind::Diff
        },
        base_commit: args.base.clone(),
        target_commit: args.target.clone(),
        scanned_files: files.len(),
    };

    let result = ScanResult::new(
        "vampiro check".to_string(),
        scanned_findings,
        scanned_diagnostics,
        vec![],
        metadata,
    );

    if args.json {
        if let Ok(json) = crate::output::render_json(&result) {
            println!("{json}");
        }
    } else {
        let human = crate::output::render_human(&result);
        print!("{human}");
    }

    // Apply policy
    let mode: ScanMode = args.mode.parse().unwrap_or(ScanMode::Guidance);
    let policy = ScanPolicy {
        mode,
        severity_threshold: args
            .severity_threshold
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(crate::finding::Severity::Medium),
        ..Default::default()
    };

    policy.evaluate(&result.findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_rs_files_single_file() {
        let paths = vec![PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/minimal.rs"
        ))];
        let files = collect_rs_files(&paths).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("minimal.rs"));
    }

    #[test]
    fn collect_rs_files_directory() {
        let paths = vec![PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures"
        ))];
        let files = collect_rs_files(&paths).unwrap();
        assert!(!files.is_empty());
        assert!(files.iter().all(|f| f.extension().unwrap() == "rs"));
    }

    #[test]
    fn collect_rs_files_non_existent_returns_error() {
        let paths = vec![PathBuf::from("/nonexistent/path.rs")];
        assert!(collect_rs_files(&paths).is_err());
    }

    #[test]
    fn collect_rs_files_non_rs_file_returns_error() {
        let paths = vec![PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/Cargo.toml"
        ))];
        assert!(collect_rs_files(&paths).is_err());
    }

    #[test]
    fn collect_rs_files_multiple_paths() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/minimal.rs");
        let paths = vec![PathBuf::from(dir), PathBuf::from(file)];
        let files = collect_rs_files(&paths).unwrap();
        assert!(!files.is_empty());
        // Dedup should handle the duplicate
        assert!(!files.is_empty());
    }
}
