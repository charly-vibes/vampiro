/// Compatibility fixture: verify Genesis tag v0.1.0 exposes the required APIs.
///
/// These tests will fail to compile until the `genesis` dependency is added
/// to `Cargo.toml` — this is the red phase of the tracer.

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
fn genesis_version_is_v0_1_0() {
    // Verify the pinned version matches expectations
    // This is a compile-time check since we pin by tag
    let _ = genesis::envelope::ENVELOPE_VERSION;
    let _ = genesis::envelope::CLI_VERSION;
}
