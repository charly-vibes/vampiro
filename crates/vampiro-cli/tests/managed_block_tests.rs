use genesis::managed_block::InjectResult;

#[test]
fn managed_block_insert_wai() {
    // Insert a WAI block into a new file
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AGENTS.md");
    let injector = vampiro_cli::managed::vampiro_injector();

    let result = injector
        .inject(&path, "WAI", "\n# Vampiro agent instructions\n")
        .unwrap();
    assert_eq!(result, InjectResult::Created);

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<!-- WAI:START -->"));
    assert!(content.contains("<!-- WAI:END -->"));
    assert!(content.contains("Vampiro agent instructions"));
}

#[test]
fn managed_block_insert_openspec() {
    // Insert an OPENSPEC block into a new file
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("openspec.md");
    let injector = vampiro_cli::managed::vampiro_injector();

    let result = injector
        .inject(&path, "OPENSPEC", "\n# OpenSpec instructions\n")
        .unwrap();
    assert_eq!(result, InjectResult::Created);

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<!-- OPENSPEC:START -->"));
    assert!(content.contains("<!-- OPENSPEC:END -->"));
}

#[test]
fn managed_block_insert_dont() {
    // Insert a DONT block into a new file
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("DONT.md");
    let injector = vampiro_cli::managed::vampiro_injector();

    let result = injector
        .inject(&path, "DONT", "\n# DONT instructions\n")
        .unwrap();
    assert_eq!(result, InjectResult::Created);

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("<!-- DONT:START -->"));
    assert!(content.contains("<!-- DONT:END -->"));
}

#[test]
fn managed_block_update_existing() {
    // Update an existing WAI block in-place
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AGENTS.md");
    let injector = vampiro_cli::managed::vampiro_injector();

    // First insert
    injector.inject(&path, "WAI", "\n# Old content\n").unwrap();

    // Update
    let result = injector.inject(&path, "WAI", "\n# New content\n").unwrap();
    assert_eq!(result, InjectResult::Updated);

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("New content"));
    assert!(!content.contains("Old content"));
    // Only one set of markers
    assert_eq!(
        content.matches("<!-- WAI:START -->").count(),
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

    injector.inject(&path, "WAI", "\n# Content\n").unwrap();
    let content1 = std::fs::read_to_string(&path).unwrap();

    injector.inject(&path, "WAI", "\n# Content\n").unwrap();
    let content2 = std::fs::read_to_string(&path).unwrap();

    assert_eq!(
        content1, content2,
        "idempotent replay should produce identical output"
    );
    assert_eq!(
        content2.matches("<!-- WAI:START -->").count(),
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

    // Create file with user content
    std::fs::write(&path, "# My Project\n\nSome user content here.\n\n").unwrap();

    // Insert block (prepended)
    injector
        .inject(&path, "WAI", "\n# Agent instructions\n")
        .unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("My Project"));
    assert!(content.contains("Some user content here."));
    assert!(content.contains("<!-- WAI:START -->"));

    // Update block
    injector
        .inject(&path, "WAI", "\n# Updated instructions\n")
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
fn managed_block_has_all_three_blocks() {
    // Verify all three blocks needed for wai status detection are registered
    let injector = vampiro_cli::managed::vampiro_injector();
    let reg = injector.registry();
    assert!(reg.has("WAI"), "WAI block must be registered");
    assert!(reg.has("OPENSPEC"), "OPENSPEC block must be registered");
    assert!(reg.has("DONT"), "DONT block must be registered");
    assert_eq!(reg.names().len(), 3, "exactly three blocks");
}
