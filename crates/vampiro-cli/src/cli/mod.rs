use clap::Parser;
use genesis::envelope::{Envelope, EnvelopeKind};

/// A program analysis tool for verifying compliance with laws and policies.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Parser, Debug)]
pub enum Commands {
    /// Reserved for analysis commands
    Check(CheckArgs),
    /// Reserved for proof commands
    Prove {
        #[command(subcommand)]
        command: Option<ProveCommands>,
    },
}

#[derive(Parser, Debug)]
pub struct CheckArgs {
    /// Output findings as JSON
    #[arg(long, short)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub enum ProveCommands {}

impl Cli {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.command {
            Some(Commands::Check(args)) => {
                if args.json {
                    run_check_json()?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

fn run_check_json() -> Result<(), Box<dyn std::error::Error>> {
    // Vampiro constructs its own normalized findings, then passes them
    // through Genesis at the serialization boundary.
    let findings: Vec<serde_json::Value> = Vec::new();

    let env = Envelope::success(EnvelopeKind::Check, findings, vec![], vec![]);

    let json = serde_json::to_string_pretty(&env)?;
    println!("{json}");
    Ok(())
}
