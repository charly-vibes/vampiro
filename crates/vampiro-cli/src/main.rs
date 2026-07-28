use clap::Parser;
use genesis::suggestions::{CommandRegistry, SuggestionEngine};
use vampiro_cli::exit_code::ExitCode;
use vampiro_cli::{load_config, Cli};

fn main() -> ExitCode {
    // Load config first — errors here are fatal
    match load_config(None) {
        Ok(_config) => {}
        Err(e) => {
            eprintln!("vampiro: error loading config: {e}");
            return ExitCode::InvalidConfig;
        }
    }

    // Set up the Genesis suggestion engine with Vampiro's command list
    let mut registry = CommandRegistry::new();
    registry.register(
        "vampiro",
        vec!["check".into(), "prove".into(), "help".into()],
    );
    let _engine: SuggestionEngine = SuggestionEngine::new();

    let cli = Cli::parse();
    match cli.run() {
        Ok(()) => ExitCode::Success,
        Err(e) => {
            eprintln!("vampiro: error: {e}");
            ExitCode::InternalError
        }
    }
}
