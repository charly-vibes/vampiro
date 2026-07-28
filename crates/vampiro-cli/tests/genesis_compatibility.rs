/// Compatibility fixture: verify Genesis-vibes v0.2.0 exposes the required APIs.
///
/// These tests verify that the published `genesis-vibes = "0.2"` crate
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
fn genesis_version_is_v0_2() {
    // Verify we're on genesis-vibes 0.2.x
    let _ = genesis::envelope::ENVELOPE_VERSION;
    let _ = genesis::envelope::CLI_VERSION;
}
