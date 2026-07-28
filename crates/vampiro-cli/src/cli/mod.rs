use std::path::{Path, PathBuf};

use clap::Parser;
use genesis::envelope::{Envelope, EnvelopeKind, Warning};
use vampiro_rust_frontend::visibility_adapter::to_visibility_facts;
use vampiro_rust_frontend::RustFrontend;
use vampiro_seam_analysis::analyze_with_visibility;

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
    /// Reserved for proof commands
    Prove {
        #[command(subcommand)]
        command: Option<ProveCommands>,
    },
}

#[derive(Parser, Debug)]
pub struct CheckArgs {
    /// Path(s) to Rust source file(s) or directories to scan
    #[arg(long, short)]
    pub path: Vec<PathBuf>,

    /// Output findings as JSON
    #[arg(long, short)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub enum ProveCommands {}

impl Cli {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.command {
            Some(Commands::Check(args)) => run_check(args),
            _ => Ok(()),
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
    for entry in std::fs::read_dir(dir).map_err(|e| format!("failed to read dir {}: {e}", dir.display()))? {
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

/// A single-file scan result.
struct FileResult {
    path: PathBuf,
    findings: Vec<vampiro_seam_analysis::Finding>,
    diagnostics: Vec<vampiro_seam_analysis::Diagnostic>,
}

fn run_check(args: &CheckArgs) -> Result<(), Box<dyn std::error::Error>> {
    let files = collect_rs_files(&args.path)?;

    let mut all_results: Vec<FileResult> = Vec::new();
    for file in &files {
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

        // Filter findings to only those from this file (analysis may reference
        // other files via test fixture paths, etc.)
        let own_findings: Vec<_> = findings
            .into_iter()
            .filter(|f| f.path == *file)
            .collect();
        let own_diagnostics: Vec<_> = diagnostics
            .into_iter()
            .filter(|d| d.path == *file)
            .collect();

        all_results.push(FileResult {
            path: file.clone(),
            findings: own_findings,
            diagnostics: own_diagnostics,
        });
    }

    if args.json {
        output_json_all(&all_results)?;
    } else {
        output_human_all(&all_results);
    }

    Ok(())
}

fn output_human_all(results: &[FileResult]) {
    let mut total_findings = 0;
    let mut total_diagnostics = 0;
    let mut scanned = 0;

    for r in results {
        scanned += 1;
        if r.findings.is_empty() && r.diagnostics.is_empty() {
            continue;
        }
        for f in &r.findings {
            println!(
                "{}:{}-{}  {} [{}]  {}  ({})",
                f.path.display(),
                f.line_range.start,
                f.line_range.end,
                f.rule,
                f.severity,
                f.classification,
                f.axis,
            );
        }
        for d in &r.diagnostics {
            println!(
                "{}:{}-{}  {}  {}",
                d.path.display(),
                d.line_range.start,
                d.line_range.end,
                d.diagnostic,
                d.detail,
            );
        }
        let ft = r.findings.len();
        let dt = r.diagnostics.len();
        println!(
            "\n{} finding(s), {} diagnostic(s) in {}",
            ft,
            dt,
            r.path.display()
        );
        total_findings += ft;
        total_diagnostics += dt;
    }

    if total_findings == 0 && total_diagnostics == 0 {
        println!("vampiro: no findings in {} file(s)", scanned);
    }

    let _ = total_findings;
    let _ = total_diagnostics;
}

fn output_json_all(results: &[FileResult]) -> Result<(), Box<dyn std::error::Error>> {
    let mut all_findings: Vec<serde_json::Value> = Vec::new();
    let mut all_warnings: Vec<Warning> = Vec::new();

    for r in results {
        for f in &r.findings {
            all_findings.push(serde_json::to_value(f)?);
        }
        for d in &r.diagnostics {
            all_warnings.push(Warning {
                rule_name: d.diagnostic.clone(),
                entity_id: Some(format!("{}:{}", d.path.display(), d.line_range.start)),
                message: d.detail.clone(),
                suggested_remediation: None,
            });
        }
    }

    let env = Envelope::success(EnvelopeKind::Check, all_findings, all_warnings, vec![]);
    println!("{}", serde_json::to_string_pretty(&env)?);
    Ok(())
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
        let file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/minimal.rs"
        );
        let paths = vec![PathBuf::from(dir), PathBuf::from(file)];
        let files = collect_rs_files(&paths).unwrap();
        assert!(!files.is_empty());
        // Dedup should handle the duplicate
        assert!(files.len() >= 1);
    }
}