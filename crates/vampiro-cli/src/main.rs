use clap::Parser;
use genesis::guide::Guide;
use vampiro_cli::exit_code::ExitCode;
use vampiro_cli::Cli;

const VAMPIRO_COMMANDS: &[&str] = &["check", "prove", "doctor", "help"];

fn main() -> ExitCode {
    // CLI scaffold from genesis::guide. The Guide bundles vampiro's name/version,
    // a CommandRegistry (for typo detection), and an ErrorSink (self-healing
    // error footer + feedback scratch).
    let guide = Guide::builder("vampiro", env!("CARGO_PKG_VERSION"))
        .commands(VAMPIRO_COMMANDS)
        .build();

    // ConfigStore with vampiro's registered config type.
    let store = vampiro_cli::config::vampiro_config_store();

    // Load config — missing config is fine (use defaults), but parse errors aren't.
    let repo_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    if let Err(e) = store.get::<vampiro_cli::config::Config>("vampiro", &repo_root) {
        if !matches!(e, genesis::config::ConfigError::MissingFile { .. }) {
            let sink = guide.error_sink();
            sink.handle(&e, &mut std::io::stderr());
            return ExitCode::InvalidConfig;
        }
    }

    let cli = Cli::parse();
    cli.run()
}
