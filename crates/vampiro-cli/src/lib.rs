//! Vampiro CLI foundation
//!
//! This crate provides the top-level CLI entry point for Vampiro, a program
//! analysis tool for verifying compliance with laws and policies.
//! The binary is a thin wrapper; library boundaries exist here for consumption
//! by CIR, analysis, and scan crates.

pub mod aix;
pub mod cli;
pub mod config;
pub mod exit_code;
pub mod finding;

pub mod managed;

pub mod output;
pub mod policy;
pub mod scan;
pub use cli::Cli;
pub use config::{vampiro_config_store, Config};
pub use exit_code::ExitCode;
