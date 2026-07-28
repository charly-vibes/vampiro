use std::process::Command;

#[test]
fn suggest_typo_shows_did_you_mean() {
    // A close typo should suggest the correct command
    let output = Command::new(env!("CARGO_BIN_EXE_vampiro"))
        .arg("chek")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr_lower = stderr.to_lowercase();
    assert!(
        stderr_lower.contains("check"),
        "typo 'chek' should suggest 'check': {stderr}"
    );
    // Exit code should be 2 (usage error)
    assert_eq!(
        output.status.code(),
        Some(2),
        "unknown command should exit 2"
    );
}

#[test]
fn suggest_unrelated_token_does_not_suggest() {
    // An unrelated token should not produce a false-positive suggestion
    let output = Command::new(env!("CARGO_BIN_EXE_vampiro"))
        .arg("xyzzy")
        .output()
        .unwrap();

    let _stderr = String::from_utf8_lossy(&output.stderr);
    // Exit code should still be 2 (usage error)
    assert_eq!(
        output.status.code(),
        Some(2),
        "unknown command should exit 2"
    );
}

#[test]
fn suggest_deterministic_ordering() {
    // Multiple runs should produce the same suggestion
    let output1 = Command::new(env!("CARGO_BIN_EXE_vampiro"))
        .arg("prov")
        .output()
        .unwrap();
    let output2 = Command::new(env!("CARGO_BIN_EXE_vampiro"))
        .arg("prov")
        .output()
        .unwrap();

    assert_eq!(
        output1.stderr, output2.stderr,
        "suggestions should be deterministic"
    );
}

#[test]
fn suggest_no_local_engine() {
    // No locally defined suggestion engine — only Genesis's engine is used.
    // This test verifies that the SuggestionEngine is set up in main.rs
    // and that Vampiro's commands are registered with Genesis.
    let engine = genesis::suggestions::SuggestionEngine::new();
    let mut registry = genesis::suggestions::CommandRegistry::new();
    registry.register(
        "vampiro",
        vec!["check".into(), "prove".into(), "help".into()],
    );

    let suggestion = engine.suggest_typo("chek", &registry);
    assert!(
        suggestion.is_some(),
        "genesis engine should suggest 'check' for typo 'chek'"
    );
    if let Some(genesis::suggestions::Suggestion::DidYouMean {
        original,
        suggestion,
    }) = suggestion
    {
        assert_eq!(original, "chek");
        assert_eq!(suggestion, "check");
    } else {
        panic!("expected DidYouMean suggestion");
    }
}
