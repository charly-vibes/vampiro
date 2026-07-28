use clap::Parser;
use vampiro_cli::exit_code::ExitCode;
use vampiro_cli::{load_config, Cli};

fn main() -> ExitCode {
    // Load config first — errors here are fatal
    match load_config(None) {
        Ok(_config) => {
            // Config loaded successfully (or no config found, using defaults)
        }
        Err(e) => {
            eprintln!("vampiro: error loading config: {e}");
            return ExitCode::InvalidConfig;
        }
    }

    let cli = Cli::parse();
    match cli.run() {
        Ok(()) => ExitCode::Success,
        Err(e) => {
            eprintln!("vampiro: error: {e}");
            ExitCode::InternalError
        }
    }
}
