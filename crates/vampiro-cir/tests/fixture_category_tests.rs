/// Fixture tests for category and filtration validation.
///
/// These tests load JSON fixture files from
/// `tests/fixtures/add-cir-plugin-platform/2/`, deserialize them,
/// and validate them.
use std::collections::HashSet;
use std::path::Path;
use vampiro_cir::{
    validate_category, validate_filtration, CategoryDecl, FiltrationDecl, MorphismId,
    ValidatedCategory,
};

/// The relative path from the workspace root to the fixture directory.
const FIXTURE_DIR: &str = "tests/fixtures/add-cir-plugin-platform/2";

fn fixture_path(name: &str) -> String {
    let manifest_dir = std::env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(FIXTURE_DIR)
        .join(name);
    path.to_string_lossy().to_string()
}

#[test]
fn fixture_valid_category_validates() {
    let path = fixture_path("valid-category.json");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
    let decl: CategoryDecl = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to deserialize fixture {path}: {e}"));

    let result = validate_category(&decl);
    assert!(
        result.is_ok(),
        "valid category fixture should validate: {result:?}"
    );

    let validated = result.unwrap();
    assert_eq!(validated.morphisms.len(), 6, "expected 6 morphisms");
    assert_eq!(
        validated.composition.len(),
        10,
        "expected 10 composition rules"
    );
}

#[test]
fn fixture_nested_filtration_validates() {
    let path = fixture_path("nested-filtration.json");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
    let filtration: FiltrationDecl = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to deserialize fixture {path}: {e}"));

    // Build a matching category with the expected morphism IDs
    let ids: HashSet<MorphismId> = vec!["id_A", "id_B", "id_C", "f", "g", "h"]
        .into_iter()
        .map(MorphismId::new)
        .collect();

    let category = ValidatedCategory {
        morphisms: vec![],
        composition: vec![],
        morphism_ids: ids,
    };

    let result = validate_filtration(&filtration, &category);
    assert!(
        result.is_ok(),
        "nested filtration fixture should validate: {result:?}"
    );
}

#[test]
fn fixture_valid_category_round_trip() {
    let path = fixture_path("valid-category.json");
    let content = std::fs::read_to_string(&path).unwrap();
    let decl: CategoryDecl = serde_json::from_str(&content).unwrap();

    let serialized = serde_json::to_string_pretty(&decl).unwrap();
    let re_parsed: CategoryDecl = serde_json::from_str(&serialized).unwrap();
    assert_eq!(decl.name, re_parsed.name);
    assert_eq!(decl.objects.len(), re_parsed.objects.len());
    assert_eq!(decl.morphisms.len(), re_parsed.morphisms.len());
}

#[test]
fn fixture_nested_filtration_round_trip() {
    let path = fixture_path("nested-filtration.json");
    let content = std::fs::read_to_string(&path).unwrap();
    let filtration: FiltrationDecl = serde_json::from_str(&content).unwrap();

    let serialized = serde_json::to_string_pretty(&filtration).unwrap();
    let re_parsed: FiltrationDecl = serde_json::from_str(&serialized).unwrap();
    assert_eq!(filtration.name, re_parsed.name);
    assert_eq!(filtration.levels.len(), re_parsed.levels.len());
}
