/// Compatibility fixture: verify Genesis-vibes v0.4.0 exposes the required APIs.
///
/// These tests verify that the published `genesis-vibes = "0.4"` crate
/// provides the modules vampiro depends on.

#[test]
fn genesis_api_envelope_importable() {
    // genesis::envelope::Envelope<T> must be accessible
    let _: genesis::envelope::Envelope<&str> = genesis::envelope::Envelope::success(
        genesis::envelope::EnvelopeKind::Ok,
        "test",
        vec![],
        vec![],
    );
}

#[test]
fn genesis_api_suggestions_importable() {
    // genesis::suggestions::SuggestionEngine must be accessible
    let engine = genesis::suggestions::SuggestionEngine::new();
    let mut reg = genesis::suggestions::CommandRegistry::new();
    reg.register("vampiro", vec!["check".into(), "prove".into()]);
    let suggestion = engine.suggest_typo("chek", &reg);
    assert!(suggestion.is_some(), "typo detection should work");
}

#[test]
fn genesis_api_managed_block_importable() {
    // genesis::managed_block::BlockInjector must be accessible
    let mut reg = genesis::managed_block::BlockRegistry::new();
    reg.register(genesis::managed_block::BlockDef::new("WAI"));
    let injector = genesis::managed_block::BlockInjector::new(reg);
    let _ = injector;
}

#[test]
fn genesis_api_aix_importable() {
    // genesis::aix::agents_block must be accessible
    let block = genesis::aix::agents_block("VAMPIRO", "# Content\n");
    assert!(block.contains("<!-- VAMPIRO:START -->"));
    assert!(block.contains("<!-- VAMPIRO:END -->"));
}

#[test]
fn genesis_api_config_importable() {
    // genesis::config::ConfigFile trait must be accessible on a concrete type
    use genesis::config::ConfigFile;
    // The trait is not dyn-compatible (requires Sized), but concrete impls compile
    fn _check<T: ConfigFile>() {}
    _check::<vampiro_cli::config::Config>();
}

#[test]
fn genesis_api_guide_importable() {
    // genesis::guide::Guide, Output, ErrorSink must be accessible
    use genesis::guide::Guide;
    let guide = Guide::builder("test", "0.1").build();
    let _ = guide.registry();
    let _ = guide.error_sink();
}

#[test]
fn genesis_api_cli_importable() {
    // genesis::cli::generate_completions and maybe_print_version_json
    use genesis::cli::maybe_print_version_json;
    let _ = maybe_print_version_json("test", "0.1.0");
}

#[test]
fn genesis_api_feedback_importable() {
    // genesis::feedback::FeedbackArgs and handle_feedback
    use genesis::feedback::FeedbackArgs;
    let args = FeedbackArgs::new("bug", true, false);
    assert_eq!(args.kind, "bug");
    assert!(args.dry_run);
}

#[test]
fn genesis_api_fixture_importable() {
    // genesis::fixture::Fixture must be constructable
    use genesis::fixture::Fixture;
    let f = Fixture::new()
        .with_marker(".vampiro")
        .with_file("test.txt", "hello")
        .build()
        .expect("fixture build");
    assert!(f.path(".vampiro").is_dir());
    assert!(f.path("test.txt").is_file());
}

#[test]
fn genesis_api_scaffold_importable() {
    // genesis::scaffold::Scaffold must be constructable
    use genesis::scaffold::Scaffold;
    let dir = std::env::temp_dir().join(format!("vampiro-test-scaffold-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let result = Scaffold::new(&dir)
        .dir(".test-dir")
        .default_config("test.toml", "key = \"val\"")
        .build()
        .expect("scaffold build");
    assert!(!result.created.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn genesis_api_discovery_importable() {
    // genesis::discovery::scan and register must be accessible
    use genesis::discovery;
    let dir = std::env::temp_dir().join(format!("vampiro-test-discovery-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    discovery::register(&dir, "test-tool", "test", "directory", ".test").unwrap();
    let tools = discovery::scan(&dir);
    assert!(!tools.is_empty());
    assert!(tools.iter().any(|t| t.name == "test-tool"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn genesis_version_is_v0_4() {
    // Verify we're on genesis-vibes 0.4.x
    let _ = genesis::envelope::ENVELOPE_VERSION;
    let _ = genesis::envelope::CLI_VERSION;
}
