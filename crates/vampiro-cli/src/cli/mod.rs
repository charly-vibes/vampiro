use clap::{Parser, Subcommand};

/// A program analysis tool for verifying compliance with laws and policies.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Reserved for analysis commands
    Check {
        #[command(subcommand)]
        command: Option<CheckCommands>,
    },
    /// Reserved for proof commands
    Prove {
        #[command(subcommand)]
        command: Option<ProveCommands>,
    },
}

#[derive(Subcommand, Debug)]
pub enum CheckCommands {}

#[derive(Subcommand, Debug)]
pub enum ProveCommands {}

impl Cli {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
