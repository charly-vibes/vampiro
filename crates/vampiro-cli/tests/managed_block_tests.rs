use genesis::managed_block::InjectResult;

#[test]
fn managed_block_insert_vampiro() {
    // Insert a VAMPIRO block into a new file
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AGENTS.md");
    let injector = vampiro_cli::managed::vampiro_injector();

    let result = injector.inject(&path, "VAMPIRO", "\n# Vampiro\n").unwrap();
    assert_eq!(result, InjectResult::Created);

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<!-- VAMPIRO:START -->"));
    assert!(content.contains("<!-- VAMPIRO:END -->"));
    assert!(content.contains("Vampiro"));
}

#[test]
fn managed_block_update_existing() {
    // Update an existing VAMPIRO block in-place
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AGENTS.md");
    let injector = vampiro_cli::managed::vampiro_injector();

    injector
        .inject(&path, "VAMPIRO", "\n# Old content\n")
        .unwrap();

    let result = injector
        .inject(&path, "VAMPIRO", "\n# New content\n")
        .unwrap();
    assert_eq!(result, InjectResult::Updated);

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("New content"));
    assert!(!content.contains("Old content"));
    assert_eq!(
        content.matches("<!-- VAMPIRO:START -->").count(),
        1,
        "should not duplicate markers"
    );
}

#[test]
fn managed_block_idempotent_replay() {
    // Replaying the same content should produce the same result
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AGENTS.md");
    let injector = vampiro_cli::managed::vampiro_injector();

    injector.inject(&path, "VAMPIRO", "\n# Content\n").unwrap();
    let content1 = std::fs::read_to_string(&path).unwrap();

    injector.inject(&path, "VAMPIRO", "\n# Content\n").unwrap();
    let content2 = std::fs::read_to_string(&path).unwrap();

    assert_eq!(
        content1, content2,
        "idempotent replay should produce identical output"
    );
    assert_eq!(
        content2.matches("<!-- VAMPIRO:START -->").count(),
        1,
        "no duplicate markers on replay"
    );
}

#[test]
fn managed_block_preserves_surrounding_content() {
    // User content outside the block should be preserved on update
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AGENTS.md");
    let injector = vampiro_cli::managed::vampiro_injector();

    std::fs::write(&path, "# My Project\n\nSome user content here.\n\n").unwrap();

    injector
        .inject(&path, "VAMPIRO", "\n# Agent instructions\n")
        .unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("My Project"));
    assert!(content.contains("Some user content here."));
    assert!(content.contains("<!-- VAMPIRO:START -->"));

    injector
        .inject(&path, "VAMPIRO", "\n# Updated instructions\n")
        .unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("My Project"), "user content preserved");
    assert!(
        content.contains("Some user content here."),
        "user content preserved"
    );
    assert!(content.contains("Updated instructions"), "block updated");
    assert!(!content.contains("Agent instructions"), "old content gone");
}

#[test]
fn managed_block_registry_has_vampiro() {
    // Verify the VAMPIRO block is registered
    let injector = vampiro_cli::managed::vampiro_injector();
    let reg = injector.registry();
    assert!(reg.has("VAMPIRO"), "VAMPIRO block must be registered");
    assert_eq!(reg.names().len(), 1, "only one block");
}
