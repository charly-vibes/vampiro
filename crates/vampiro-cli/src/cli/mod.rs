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
    /// Path to the Rust source file to analyze
    #[arg(long, short)]
    pub path: PathBuf,

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

fn run_check(args: &CheckArgs) -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(&args.path)
        .map_err(|e| format!("failed to read {}: {e}", args.path.display()))?;

    let out = RustFrontend
        .extract_full(&source, &args.path)
        .map_err(|e| format!("frontend extraction failed: {e}"))?;

    let vis = to_visibility_facts(&out);
    let (findings, diagnostics) = analyze_with_visibility(&out.graph, &vis);

    if args.json {
        output_json(findings, diagnostics)?;
    } else {
        output_human(findings, diagnostics, &args.path);
    }

    Ok(())
}

fn output_json(
    findings: Vec<vampiro_seam_analysis::Finding>,
    diagnostics: Vec<vampiro_seam_analysis::Diagnostic>,
) -> Result<(), Box<dyn std::error::Error>> {
    let finding_values: Vec<serde_json::Value> = findings
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<_, _>>()?;
    let warnings: Vec<Warning> = diagnostics
        .iter()
        .map(|d| Warning {
            rule_name: d.diagnostic.clone(),
            entity_id: Some(format!("{}:{}", d.path.display(), d.line_range.start)),
            message: d.detail.clone(),
            suggested_remediation: None,
        })
        .collect();

    let env = Envelope::success(EnvelopeKind::Check, finding_values, warnings, vec![]);

    let json = serde_json::to_string_pretty(&env)?;
    println!("{json}");
    Ok(())
}

fn output_human(
    findings: Vec<vampiro_seam_analysis::Finding>,
    diagnostics: Vec<vampiro_seam_analysis::Diagnostic>,
    path: &Path,
) {
    if findings.is_empty() && diagnostics.is_empty() {
        println!("vampiro: no findings in {}", path.display());
        return;
    }

    for f in &findings {
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

    for d in &diagnostics {
        println!(
            "{}:{}-{}  {}  {}",
            d.path.display(),
            d.line_range.start,
            d.line_range.end,
            d.diagnostic,
            d.detail,
        );
    }

    let total = findings.len() + diagnostics.len();
    println!(
        "\n{} finding(s), {} diagnostic(s) in {}",
        findings.len(),
        diagnostics.len(),
        path.display()
    );
    let _ = total;
}
