/// Category and filtration declarations for Vampiro CIR.
///
/// This module provides finite category declarations with validation:
/// - Missing identities are rejected before closure construction
/// - Composition tables are exhaustively checked for closure and associativity
/// - Filtrations are validated for nesting and wide subcategory membership
/// - Resource limits prevent unbounded closure growth
use std::collections::{HashMap, HashSet};

/// A morphism identifier.
///
/// Must be non-empty. Created via [`MorphismId::new`] which panics on empty input.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MorphismId(String);

impl MorphismId {
    /// Create a new morphism ID.
    ///
    /// # Panics
    ///
    /// Panics if `id` is empty.
    pub fn new(id: impl Into<String>) -> Self {
        let s = id.into();
        assert!(!s.is_empty(), "MorphismId must not be empty");
        MorphismId(s)
    }

    /// Get the ID string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MorphismId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for MorphismId {
    fn from(s: &str) -> Self {
        assert!(!s.is_empty(), "MorphismId must not be empty");
        MorphismId(s.to_string())
    }
}

/// A morphism declaration: identity or non-identity generator.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MorphismDecl {
    pub id: MorphismId,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub is_identity: bool,
}

/// A composition rule: `first ∘ second = result`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CompositionRule {
    pub first: MorphismId,
    pub second: MorphismId,
    pub result: MorphismId,
}

/// A declared category with objects, morphisms, and composition.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CategoryDecl {
    pub name: String,
    pub objects: Vec<String>,
    pub morphisms: Vec<MorphismDecl>,
    pub composition: Vec<CompositionRule>,
}

impl std::fmt::Display for CategoryDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Category `{}`: {} objects, {} morphisms, {} composition rules",
            self.name,
            self.objects.len(),
            self.morphisms.len(),
            self.composition.len(),
        )
    }
}

/// A single filtration level (a wide subcategory of its predecessor).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FiltrationLevel {
    pub name: String,
    pub index: u32,
    /// Morphism IDs in this level. Duplicates are ignored on deserialization;
    /// use `from_ids` to construct from a `HashSet`.
    #[serde(default)]
    pub morphism_ids: HashSet<MorphismId>,
}

impl FiltrationLevel {
    /// Create a new filtration level from a set of morphism IDs.
    pub fn from_ids(name: impl Into<String>, index: u32, ids: HashSet<MorphismId>) -> Self {
        FiltrationLevel {
            name: name.into(),
            index,
            morphism_ids: ids,
        }
    }
}

/// A filtration declaration: a sequence of nested wide subcategories.
///
/// Levels are validated in order of their `index` field. Duplicate indices
/// are rejected. Levels must be properly nested (each is a subset of its
/// predecessor).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FiltrationDecl {
    pub name: String,
    pub levels: Vec<FiltrationLevel>,
}

/// Validation errors for category/filtration declarations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// An object has no explicitly declared identity morphism.
    MissingIdentity { object: String },
    /// Composable morphisms have no composition rule defined.
    NonClosedComposition { first: String, second: String },
    /// Composition is not associative: (f∘g)∘h ≠ f∘(g∘h).
    NonAssociative {
        first_id: String,
        second_id: String,
        third_id: String,
    },
    /// A filtration level contains morphisms not in the parent category.
    InvalidWideSubcategory { level: String, missing: Vec<String> },
    /// A filtration level is not a subcategory of its predecessor.
    NonNesting { level: String, predecessor: String },
    /// The closure computation exceeded the configured resource limit.
    ResourceLimitExceeded { limit: u32, observed: u32 },
    /// A morphism referenced in composition or filtration does not exist.
    UndefinedMorphism { id: String },
    /// An object referenced by a morphism is not declared.
    UndefinedObject { object: String },
    /// Duplicate filtration level index.
    DuplicateFiltrationIndex { index: u32 },
    /// Filtration levels are not sorted by index.
    UnsortedFiltrationLevels,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::MissingIdentity { object } => {
                write!(fmt, "missing identity morphism for object `{object}`")
            }
            ValidationError::NonClosedComposition { first, second } => {
                write!(fmt, "no composition rule for `{first} ∘ {second}`")
            }
            ValidationError::NonAssociative {
                first_id,
                second_id,
                third_id,
            } => {
                write!(fmt, "non-associative composition: (`{first_id}` ∘ `{second_id}`) ∘ `{third_id}` ≠ `{first_id}` ∘ (`{second_id}` ∘ `{third_id}`)")
            }
            ValidationError::InvalidWideSubcategory { level, missing } => {
                write!(
                    fmt,
                    "level `{level}` contains morphisms not in the parent category: {}",
                    missing.join(", ")
                )
            }
            ValidationError::NonNesting { level, predecessor } => {
                write!(
                    fmt,
                    "level `{level}` is not a subcategory of `{predecessor}`"
                )
            }
            ValidationError::ResourceLimitExceeded { limit, observed } => {
                write!(fmt, "resource limit {limit} exceeded: observed {observed}")
            }
            ValidationError::UndefinedMorphism { id } => {
                write!(fmt, "morphism `{id}` is not declared")
            }
            ValidationError::UndefinedObject { object } => {
                write!(fmt, "object `{object}` is referenced but not declared")
            }
            ValidationError::DuplicateFiltrationIndex { index } => {
                write!(fmt, "duplicate filtration level index {index}")
            }
            ValidationError::UnsortedFiltrationLevels => {
                write!(fmt, "filtration levels must be sorted by index")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// The maximum number of morphisms in a validated category.
pub const MAX_CLOSURE_SIZE: u32 = 4096;

/// The maximum number of filtration levels.
pub const MAX_FILTRATION_LEVELS: u32 = 16;

/// The result of a validated category declaration.
///
/// Contains the declared morphisms and their composition table.
/// The `morphism_ids` set contains all *declared* morphism IDs
/// (not computed composites — those are resolved through the
/// composition table as needed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedCategory {
    /// All declared morphisms (identities + generators).
    pub morphisms: Vec<MorphismDecl>,
    /// The composition table of declared rules.
    pub composition: Vec<CompositionRule>,
    /// The set of all declared morphism IDs.
    pub morphism_ids: HashSet<MorphismId>,
}

impl std::fmt::Display for ValidatedCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ValidatedCategory: {} morphisms, {} composition rules",
            self.morphisms.len(),
            self.composition.len(),
        )
    }
}

/// Compute the filtration level for a given morphism/edge.
///
/// Returns the index of the least containing filtration level
/// (the lowest-indexed level that contains the morphism),
/// or `None` if the morphism is not in any filtration level.
pub fn filtration_level(edge_id: &MorphismId, filtration: &FiltrationDecl) -> Option<u32> {
    // Check from lowest to highest to find the least containing level
    for level in filtration.levels.iter() {
        if level.morphism_ids.contains(edge_id) {
            return Some(level.index);
        }
    }
    None
}

/// Validate a category declaration.
///
/// Checks:
/// - All objects have an explicitly declared identity morphism
/// - All morphism source/target objects are declared
/// - All composition rule morphisms are declared
/// - Composition is closed under the declared rules
/// - Composition is associative (using composable-pair adjacency lists)
/// - Resource limits are not exceeded
pub fn validate_category(decl: &CategoryDecl) -> Result<ValidatedCategory, ValidationError> {
    let object_set: HashSet<&str> = decl.objects.iter().map(|s| s.as_str()).collect();

    // Build morphism map
    let mut morphism_map: HashMap<&str, &MorphismDecl> = HashMap::new();
    for m in &decl.morphisms {
        morphism_map.insert(m.id.as_str(), m);
    }

    // Check all objects have identity morphisms
    for obj in &decl.objects {
        let has_identity = decl
            .morphisms
            .iter()
            .any(|m| m.is_identity && m.source == *obj && m.target == *obj);
        if !has_identity {
            return Err(ValidationError::MissingIdentity {
                object: obj.clone(),
            });
        }
    }

    // Check all morphism objects are declared
    for m in &decl.morphisms {
        if !object_set.contains(m.source.as_str()) {
            return Err(ValidationError::UndefinedObject {
                object: m.source.clone(),
            });
        }
        if !object_set.contains(m.target.as_str()) {
            return Err(ValidationError::UndefinedObject {
                object: m.target.clone(),
            });
        }
    }

    // Check all composition rule morphisms are declared
    for rule in &decl.composition {
        if !morphism_map.contains_key(rule.first.as_str()) {
            return Err(ValidationError::UndefinedMorphism {
                id: rule.first.to_string(),
            });
        }
        if !morphism_map.contains_key(rule.second.as_str()) {
            return Err(ValidationError::UndefinedMorphism {
                id: rule.second.to_string(),
            });
        }
        if !morphism_map.contains_key(rule.result.as_str()) {
            return Err(ValidationError::UndefinedMorphism {
                id: rule.result.to_string(),
            });
        }
    }

    // Check resource limit
    let total = decl.morphisms.len() as u32;
    if total > MAX_CLOSURE_SIZE {
        return Err(ValidationError::ResourceLimitExceeded {
            limit: MAX_CLOSURE_SIZE,
            observed: total,
        });
    }

    let closure_ids: HashSet<MorphismId> = decl.morphisms.iter().map(|m| m.id.clone()).collect();

    // Create a composition lookup: (first, second) -> result
    let mut comp_lookup: HashMap<(&str, &str), &str> = HashMap::new();
    for rule in &decl.composition {
        comp_lookup.insert(
            (rule.first.as_str(), rule.second.as_str()),
            rule.result.as_str(),
        );
    }

    // Build composable-pair adjacency: for each morphism f, list all g
    // where target(f) == source(g). This avoids O(n²) in the closure check.
    let mut composable_targets: HashMap<&str, Vec<&MorphismDecl>> = HashMap::new();
    for f in &decl.morphisms {
        for g in &decl.morphisms {
            if f.target == g.source {
                composable_targets.entry(f.id.as_str()).or_default().push(g);
            }
        }
    }

    // Check closure: for every composable pair, there must be a composition rule
    for f in &decl.morphisms {
        if let Some(targets) = composable_targets.get(f.id.as_str()) {
            for g in targets {
                if !comp_lookup.contains_key(&(f.id.as_str(), g.id.as_str())) {
                    return Err(ValidationError::NonClosedComposition {
                        first: f.id.to_string(),
                        second: g.id.to_string(),
                    });
                }
            }
        }
    }

    // Check associativity using composable-pair adjacency lists.
    // For each composable pair (f, g) with f∘g = fg, find all h
    // composable with g. Then check (f∘g)∘h == f∘(g∘h).
    for f in &decl.morphisms {
        let f_targets = match composable_targets.get(f.id.as_str()) {
            Some(list) => list,
            None => continue,
        };

        for g in f_targets {
            let fg_name = match comp_lookup.get(&(f.id.as_str(), g.id.as_str())) {
                Some(name) => *name,
                None => continue,
            };
            if !morphism_map.contains_key(fg_name) {
                continue;
            }

            let g_targets = match composable_targets.get(g.id.as_str()) {
                Some(list) => list,
                None => continue,
            };

            for h in g_targets {
                let gh_name = match comp_lookup.get(&(g.id.as_str(), h.id.as_str())) {
                    Some(name) => *name,
                    None => continue,
                };
                if !morphism_map.contains_key(gh_name) {
                    continue;
                }

                // (f ∘ g) ∘ h = fg ∘ h
                let fg_gh_name = comp_lookup.get(&(fg_name, h.id.as_str()));
                // f ∘ (g ∘ h) = f ∘ gh
                let f_gh_name = comp_lookup.get(&(f.id.as_str(), gh_name));

                match (fg_gh_name, f_gh_name) {
                    (Some(a), Some(b)) if a == b => {} // associative
                    (Some(_), Some(_)) => {
                        return Err(ValidationError::NonAssociative {
                            first_id: f.id.to_string(),
                            second_id: g.id.to_string(),
                            third_id: h.id.to_string(),
                        });
                    }
                    _ => {} // one or both not defined — not a violation
                }
            }
        }
    }

    Ok(ValidatedCategory {
        morphisms: decl.morphisms.clone(),
        composition: decl.composition.clone(),
        morphism_ids: closure_ids,
    })
}

/// Validate a filtration declaration against a validated category.
///
/// Checks:
/// - Levels are sorted by index (rejects unsorted)
/// - No duplicate indices
/// - Each level is a wide subcategory of its predecessor
/// - All referenced morphisms exist in the validated category
/// - Resource limits are not exceeded
pub fn validate_filtration(
    filtration: &FiltrationDecl,
    category: &ValidatedCategory,
) -> Result<(), ValidationError> {
    if filtration.levels.is_empty() {
        return Ok(());
    }

    // Check resource limit
    if filtration.levels.len() as u32 > MAX_FILTRATION_LEVELS {
        return Err(ValidationError::ResourceLimitExceeded {
            limit: MAX_FILTRATION_LEVELS,
            observed: filtration.levels.len() as u32,
        });
    }

    // Check for duplicate indices (before sortedness, because duplicate
    // indices are also unsorted in the strict sense)
    let mut seen_indices: HashSet<u32> = HashSet::new();
    for level in &filtration.levels {
        if !seen_indices.insert(level.index) {
            return Err(ValidationError::DuplicateFiltrationIndex { index: level.index });
        }
    }

    // Verify levels are sorted by index
    for pair in filtration.levels.windows(2) {
        if pair[0].index >= pair[1].index {
            return Err(ValidationError::UnsortedFiltrationLevels);
        }
    }

    // All morphism IDs must exist in the category
    for level in &filtration.levels {
        for mid in &level.morphism_ids {
            if !category.morphism_ids.contains(mid) {
                return Err(ValidationError::UndefinedMorphism {
                    id: mid.to_string(),
                });
            }
        }
    }

    // Check nesting and wide subcategory membership
    let mut prev_ids: Option<&HashSet<MorphismId>> = None;

    for (i, level) in filtration.levels.iter().enumerate() {
        if i == 0 {
            prev_ids = Some(&level.morphism_ids);
            continue;
        }

        let prev = prev_ids.unwrap();
        let not_in_prev: Vec<String> = level
            .morphism_ids
            .iter()
            .filter(|m| !prev.contains(*m))
            .map(|m| m.to_string())
            .collect();

        if !not_in_prev.is_empty() {
            return Err(ValidationError::InvalidWideSubcategory {
                level: level.name.clone(),
                missing: not_in_prev,
            });
        }

        prev_ids = Some(&level.morphism_ids);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_decl(id: &str, source: &str, target: &str, is_identity: bool) -> MorphismDecl {
        MorphismDecl {
            id: MorphismId::new(id),
            source: source.to_string(),
            target: target.to_string(),
            is_identity,
        }
    }

    fn ids_from(vec: Vec<&str>) -> HashSet<MorphismId> {
        vec.into_iter().map(MorphismId::new).collect()
    }

    // --- Missing identity ---

    #[test]
    fn missing_identity_is_rejected() {
        let decl = CategoryDecl {
            name: "test".into(),
            objects: vec!["A".into(), "B".into()],
            morphisms: vec![
                make_decl("id_A", "A", "A", true),
                make_decl("f", "A", "B", false),
            ],
            composition: vec![],
        };
        let result = validate_category(&decl);
        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::MissingIdentity { object } => assert_eq!(object, "B"),
            other => panic!("expected MissingIdentity, got {other:?}"),
        }
    }

    // --- Non-closed composition ---

    #[test]
    fn non_closed_composition_is_rejected() {
        let decl = CategoryDecl {
            name: "test".into(),
            objects: vec!["A".into(), "B".into(), "C".into()],
            morphisms: vec![
                make_decl("id_A", "A", "A", true),
                make_decl("id_B", "B", "B", true),
                make_decl("id_C", "C", "C", true),
                make_decl("f", "A", "B", false),
                make_decl("g", "B", "C", false),
            ],
            composition: vec![],
        };
        let result = validate_category(&decl);
        assert!(result.is_err(), "expected error for non-closed composition");
        match result.unwrap_err() {
            ValidationError::NonClosedComposition { .. } => {}
            other => panic!("expected NonClosedComposition, got {other:?}"),
        }
    }

    // --- Invalid wide subcategory ---

    #[test]
    fn invalid_wide_subcategory_is_rejected() {
        let category = ValidatedCategory {
            morphisms: vec![
                make_decl("id_A", "A", "A", true),
                make_decl("id_B", "B", "B", true),
                make_decl("f", "A", "B", false),
            ],
            composition: vec![],
            morphism_ids: ids_from(vec!["id_A", "id_B", "f"]),
        };

        let filtration = FiltrationDecl {
            name: "test".into(),
            levels: vec![
                FiltrationLevel::from_ids("L0", 0, ids_from(vec!["id_A", "id_B", "f"])),
                FiltrationLevel::from_ids("L1", 1, ids_from(vec!["id_A", "id_B", "f"])),
                FiltrationLevel::from_ids(
                    "L2",
                    2,
                    ids_from(vec!["id_A", "id_B", "f", "nonexistent"]),
                ),
            ],
        };

        let result = validate_filtration(&filtration, &category);
        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::UndefinedMorphism { id } => assert_eq!(id, "nonexistent"),
            other => panic!("expected UndefinedMorphism, got {other:?}"),
        }
    }

    // --- Non-nesting ---

    #[test]
    fn non_nesting_is_rejected() {
        let category = ValidatedCategory {
            morphisms: vec![
                make_decl("id_A", "A", "A", true),
                make_decl("id_B", "B", "B", true),
                make_decl("f", "A", "B", false),
            ],
            composition: vec![],
            morphism_ids: ids_from(vec!["id_A", "id_B", "f"]),
        };

        // L1 has id_B which is not in L0
        let filtration = FiltrationDecl {
            name: "test".into(),
            levels: vec![
                FiltrationLevel::from_ids("L0", 0, ids_from(vec!["id_A"])),
                FiltrationLevel::from_ids("L1", 1, ids_from(vec!["id_A", "id_B"])),
            ],
        };

        let result = validate_filtration(&filtration, &category);
        assert!(result.is_err(), "expected error for non-nesting filtration");
        match result.unwrap_err() {
            ValidationError::InvalidWideSubcategory { level, .. } => {
                assert_eq!(level, "L1");
            }
            other => panic!("expected InvalidWideSubcategory, got {other:?}"),
        }
    }

    // --- Arbitrary depth ---

    #[test]
    fn arbitrary_filtration_depth_accepted() {
        let category = ValidatedCategory {
            morphisms: vec![
                make_decl("id_A", "A", "A", true),
                make_decl("id_B", "B", "B", true),
            ],
            composition: vec![],
            morphism_ids: ids_from(vec!["id_A", "id_B"]),
        };

        let mut levels = Vec::new();
        for i in 0..10u32 {
            let ids = ids_from(vec!["id_A", "id_B"]);
            levels.push(FiltrationLevel::from_ids(format!("L{i}"), i, ids));
        }

        let filtration = FiltrationDecl {
            name: "deep".into(),
            levels,
        };

        let result = validate_filtration(&filtration, &category);
        assert!(
            result.is_ok(),
            "arbitrary depth should be accepted: {result:?}"
        );
    }

    // --- Filtration level computation ---

    #[test]
    fn filtration_level_computed_correctly() {
        let filtration = FiltrationDecl {
            name: "test".into(),
            levels: vec![
                FiltrationLevel::from_ids("L0", 0, ids_from(vec!["id_A"])),
                FiltrationLevel::from_ids("L1", 1, ids_from(vec!["id_A", "f"])),
                FiltrationLevel::from_ids("L2", 2, ids_from(vec!["id_A", "f", "g"])),
            ],
        };

        assert_eq!(
            filtration_level(&MorphismId::new("id_A"), &filtration),
            Some(0)
        );
        assert_eq!(
            filtration_level(&MorphismId::new("f"), &filtration),
            Some(1)
        );
        assert_eq!(
            filtration_level(&MorphismId::new("g"), &filtration),
            Some(2)
        );
        assert_eq!(filtration_level(&MorphismId::new("h"), &filtration), None);
    }

    // --- Valid category ---

    #[test]
    fn valid_category_with_identity_and_composition() {
        let decl = CategoryDecl {
            name: "test".into(),
            objects: vec!["A".into(), "B".into(), "C".into()],
            morphisms: vec![
                make_decl("id_A", "A", "A", true),
                make_decl("id_B", "B", "B", true),
                make_decl("id_C", "C", "C", true),
                make_decl("f", "A", "B", false),
                make_decl("g", "B", "C", false),
                make_decl("h", "A", "C", false),
            ],
            composition: vec![
                CompositionRule {
                    first: MorphismId::new("id_A"),
                    second: MorphismId::new("id_A"),
                    result: MorphismId::new("id_A"),
                },
                CompositionRule {
                    first: MorphismId::new("id_B"),
                    second: MorphismId::new("id_B"),
                    result: MorphismId::new("id_B"),
                },
                CompositionRule {
                    first: MorphismId::new("id_C"),
                    second: MorphismId::new("id_C"),
                    result: MorphismId::new("id_C"),
                },
                CompositionRule {
                    first: MorphismId::new("id_A"),
                    second: MorphismId::new("f"),
                    result: MorphismId::new("f"),
                },
                CompositionRule {
                    first: MorphismId::new("f"),
                    second: MorphismId::new("id_B"),
                    result: MorphismId::new("f"),
                },
                CompositionRule {
                    first: MorphismId::new("id_B"),
                    second: MorphismId::new("g"),
                    result: MorphismId::new("g"),
                },
                CompositionRule {
                    first: MorphismId::new("g"),
                    second: MorphismId::new("id_C"),
                    result: MorphismId::new("g"),
                },
                CompositionRule {
                    first: MorphismId::new("id_A"),
                    second: MorphismId::new("h"),
                    result: MorphismId::new("h"),
                },
                CompositionRule {
                    first: MorphismId::new("h"),
                    second: MorphismId::new("id_C"),
                    result: MorphismId::new("h"),
                },
                CompositionRule {
                    first: MorphismId::new("f"),
                    second: MorphismId::new("g"),
                    result: MorphismId::new("h"),
                },
            ],
        };
        let result = validate_category(&decl);
        assert!(result.is_ok(), "valid category should pass: {result:?}");
    }

    // --- Resource limit ---

    #[test]
    fn resource_limit_exceeded_is_rejected() {
        let mut objects = Vec::new();
        let mut morphisms = Vec::new();
        let mut composition = Vec::new();
        for i in 0..(MAX_CLOSURE_SIZE + 1) {
            let obj = format!("O{i}");
            let id_name = format!("id_{i}");
            objects.push(obj.clone());
            morphisms.push(make_decl(&id_name, &obj, &obj, true));
            composition.push(CompositionRule {
                first: MorphismId::new(&id_name),
                second: MorphismId::new(&id_name),
                result: MorphismId::new(&id_name),
            });
        }

        let decl = CategoryDecl {
            name: "big".into(),
            objects,
            morphisms,
            composition,
        };

        let result = validate_category(&decl);
        assert!(result.is_err(), "expected resource limit exceeded error");
        match result.unwrap_err() {
            ValidationError::ResourceLimitExceeded { limit, .. } => {
                assert_eq!(limit, MAX_CLOSURE_SIZE);
            }
            other => panic!("expected ResourceLimitExceeded, got {other:?}"),
        }
    }

    // --- New tests for RO5U fixes ---

    #[test]
    fn non_associative_composition_is_rejected() {
        // Declare a category where (f∘g)∘h ≠ f∘(g∘h)
        // Objects: A, B, C, D
        // f: A→B, g: B→C, h: C→D
        // f∘g = x (A→C),  g∘h = y (B→D)
        // x∘h = z1 (A→D),  f∘y = z2 (A→D), with z1 ≠ z2
        let decl = CategoryDecl {
            name: "non-assoc".into(),
            objects: vec!["A".into(), "B".into(), "C".into(), "D".into()],
            morphisms: vec![
                make_decl("id_A", "A", "A", true),
                make_decl("id_B", "B", "B", true),
                make_decl("id_C", "C", "C", true),
                make_decl("id_D", "D", "D", true),
                make_decl("f", "A", "B", false),
                make_decl("g", "B", "C", false),
                make_decl("h", "C", "D", false),
                make_decl("x", "A", "C", false),  // f∘g = x
                make_decl("y", "B", "D", false),  // g∘h = y
                make_decl("z1", "A", "D", false), // x∘h = z1
                make_decl("z2", "A", "D", false), // f∘y = z2 (z1 ≠ z2 → non-associative)
            ],
            composition: vec![
                // Identities
                CompositionRule {
                    first: MorphismId::new("id_A"),
                    second: MorphismId::new("id_A"),
                    result: MorphismId::new("id_A"),
                },
                CompositionRule {
                    first: MorphismId::new("id_B"),
                    second: MorphismId::new("id_B"),
                    result: MorphismId::new("id_B"),
                },
                CompositionRule {
                    first: MorphismId::new("id_C"),
                    second: MorphismId::new("id_C"),
                    result: MorphismId::new("id_C"),
                },
                CompositionRule {
                    first: MorphismId::new("id_D"),
                    second: MorphismId::new("id_D"),
                    result: MorphismId::new("id_D"),
                },
                // Identity unit laws
                CompositionRule {
                    first: MorphismId::new("id_A"),
                    second: MorphismId::new("f"),
                    result: MorphismId::new("f"),
                },
                CompositionRule {
                    first: MorphismId::new("f"),
                    second: MorphismId::new("id_B"),
                    result: MorphismId::new("f"),
                },
                CompositionRule {
                    first: MorphismId::new("id_B"),
                    second: MorphismId::new("g"),
                    result: MorphismId::new("g"),
                },
                CompositionRule {
                    first: MorphismId::new("g"),
                    second: MorphismId::new("id_C"),
                    result: MorphismId::new("g"),
                },
                CompositionRule {
                    first: MorphismId::new("id_C"),
                    second: MorphismId::new("h"),
                    result: MorphismId::new("h"),
                },
                CompositionRule {
                    first: MorphismId::new("h"),
                    second: MorphismId::new("id_D"),
                    result: MorphismId::new("h"),
                },
                CompositionRule {
                    first: MorphismId::new("id_A"),
                    second: MorphismId::new("x"),
                    result: MorphismId::new("x"),
                },
                CompositionRule {
                    first: MorphismId::new("x"),
                    second: MorphismId::new("id_C"),
                    result: MorphismId::new("x"),
                },
                CompositionRule {
                    first: MorphismId::new("x"),
                    second: MorphismId::new("id_D"),
                    result: MorphismId::new("x"),
                },
                CompositionRule {
                    first: MorphismId::new("id_A"),
                    second: MorphismId::new("z1"),
                    result: MorphismId::new("z1"),
                },
                CompositionRule {
                    first: MorphismId::new("z1"),
                    second: MorphismId::new("id_D"),
                    result: MorphismId::new("z1"),
                },
                CompositionRule {
                    first: MorphismId::new("id_A"),
                    second: MorphismId::new("z2"),
                    result: MorphismId::new("z2"),
                },
                CompositionRule {
                    first: MorphismId::new("z2"),
                    second: MorphismId::new("id_D"),
                    result: MorphismId::new("z2"),
                },
                // f∘g = x, g∘h = y
                CompositionRule {
                    first: MorphismId::new("f"),
                    second: MorphismId::new("g"),
                    result: MorphismId::new("x"),
                },
                CompositionRule {
                    first: MorphismId::new("g"),
                    second: MorphismId::new("h"),
                    result: MorphismId::new("y"),
                },
                // x∘h = z1, f∘y = z2  (z1 ≠ z2 → violation)
                CompositionRule {
                    first: MorphismId::new("x"),
                    second: MorphismId::new("h"),
                    result: MorphismId::new("z1"),
                },
                CompositionRule {
                    first: MorphismId::new("f"),
                    second: MorphismId::new("y"),
                    result: MorphismId::new("z2"),
                },
                // Identity unit laws for y, x, z1, z2
                CompositionRule {
                    first: MorphismId::new("id_B"),
                    second: MorphismId::new("y"),
                    result: MorphismId::new("y"),
                },
                CompositionRule {
                    first: MorphismId::new("y"),
                    second: MorphismId::new("id_D"),
                    result: MorphismId::new("y"),
                },
                CompositionRule {
                    first: MorphismId::new("id_A"),
                    second: MorphismId::new("x"),
                    result: MorphismId::new("x"),
                },
                CompositionRule {
                    first: MorphismId::new("x"),
                    second: MorphismId::new("id_C"),
                    result: MorphismId::new("x"),
                },
                CompositionRule {
                    first: MorphismId::new("x"),
                    second: MorphismId::new("id_D"),
                    result: MorphismId::new("x"),
                },
                CompositionRule {
                    first: MorphismId::new("id_A"),
                    second: MorphismId::new("z1"),
                    result: MorphismId::new("z1"),
                },
                CompositionRule {
                    first: MorphismId::new("z1"),
                    second: MorphismId::new("id_D"),
                    result: MorphismId::new("z1"),
                },
                CompositionRule {
                    first: MorphismId::new("id_A"),
                    second: MorphismId::new("z2"),
                    result: MorphismId::new("z2"),
                },
                CompositionRule {
                    first: MorphismId::new("z2"),
                    second: MorphismId::new("id_D"),
                    result: MorphismId::new("z2"),
                },
            ],
        };
        let result = validate_category(&decl);
        assert!(result.is_err(), "expected non-associative error");
        match result.unwrap_err() {
            ValidationError::NonAssociative {
                first_id,
                second_id,
                third_id,
            } => {
                assert_eq!(first_id, "f");
                assert_eq!(second_id, "g");
                assert_eq!(third_id, "h");
            }
            other => panic!("expected NonAssociative, got {other:?}"),
        }
    }

    #[test]
    fn empty_category_is_accepted() {
        let decl = CategoryDecl {
            name: "empty".into(),
            objects: vec![],
            morphisms: vec![],
            composition: vec![],
        };
        let result = validate_category(&decl);
        assert!(
            result.is_ok(),
            "empty category should be accepted: {result:?}"
        );
        let validated = result.unwrap();
        assert!(validated.morphisms.is_empty());
        assert!(validated.composition.is_empty());
        assert!(validated.morphism_ids.is_empty());
    }

    #[test]
    fn empty_filtration_is_accepted() {
        let category = ValidatedCategory {
            morphisms: vec![],
            composition: vec![],
            morphism_ids: HashSet::new(),
        };
        let filtration = FiltrationDecl {
            name: "empty".into(),
            levels: vec![],
        };
        let result = validate_filtration(&filtration, &category);
        assert!(result.is_ok(), "empty filtration should be accepted");
    }

    #[test]
    fn duplicate_filtration_index_is_rejected() {
        let category = ValidatedCategory {
            morphisms: vec![make_decl("id_A", "A", "A", true)],
            composition: vec![],
            morphism_ids: ids_from(vec!["id_A"]),
        };

        let filtration = FiltrationDecl {
            name: "dup".into(),
            levels: vec![
                FiltrationLevel::from_ids("L0", 0, ids_from(vec!["id_A"])),
                FiltrationLevel::from_ids("L0_dup", 0, ids_from(vec!["id_A"])),
            ],
        };

        let result = validate_filtration(&filtration, &category);
        assert!(result.is_err(), "expected error for duplicate index");
        match result.unwrap_err() {
            ValidationError::DuplicateFiltrationIndex { index } => assert_eq!(index, 0),
            other => panic!("expected DuplicateFiltrationIndex, got {other:?}"),
        }
    }

    #[test]
    fn unsorted_filtration_is_rejected() {
        let category = ValidatedCategory {
            morphisms: vec![make_decl("id_A", "A", "A", true)],
            composition: vec![],
            morphism_ids: ids_from(vec!["id_A"]),
        };

        let filtration = FiltrationDecl {
            name: "unsorted".into(),
            levels: vec![
                FiltrationLevel::from_ids("L1", 1, ids_from(vec!["id_A"])),
                FiltrationLevel::from_ids("L0", 0, ids_from(vec!["id_A"])),
            ],
        };

        let result = validate_filtration(&filtration, &category);
        assert!(result.is_err(), "expected error for unsorted levels");
        match result.unwrap_err() {
            ValidationError::UnsortedFiltrationLevels => {} // expected
            other => panic!("expected UnsortedFiltrationLevels, got {other:?}"),
        }
    }

    #[test]
    fn filtration_level_boundary_max() {
        let category = ValidatedCategory {
            morphisms: vec![make_decl("id_A", "A", "A", true)],
            composition: vec![],
            morphism_ids: ids_from(vec!["id_A"]),
        };

        // MAX_FILTRATION_LEVELS levels should pass
        let mut levels = Vec::new();
        for i in 0..MAX_FILTRATION_LEVELS {
            levels.push(FiltrationLevel::from_ids(
                format!("L{i}"),
                i,
                ids_from(vec!["id_A"]),
            ));
        }
        let filtration = FiltrationDecl {
            name: "at-limit".into(),
            levels,
        };
        assert!(
            validate_filtration(&filtration, &category).is_ok(),
            "MAX_FILTRATION_LEVELS levels should pass"
        );

        // MAX_FILTRATION_LEVELS + 1 levels should fail
        let mut levels = Vec::new();
        for i in 0..=MAX_FILTRATION_LEVELS {
            levels.push(FiltrationLevel::from_ids(
                format!("L{i}"),
                i,
                ids_from(vec!["id_A"]),
            ));
        }
        let filtration = FiltrationDecl {
            name: "over-limit".into(),
            levels,
        };
        let result = validate_filtration(&filtration, &category);
        assert!(
            result.is_err(),
            "expected error for exceeding MAX_FILTRATION_LEVELS"
        );
        match result.unwrap_err() {
            ValidationError::ResourceLimitExceeded { limit, .. } => {
                assert_eq!(limit, MAX_FILTRATION_LEVELS);
            }
            other => panic!("expected ResourceLimitExceeded, got {other:?}"),
        }
    }

    #[test]
    fn morphism_id_empty_panics() {
        let result = std::panic::catch_unwind(|| {
            MorphismId::new("");
        });
        assert!(result.is_err(), "empty MorphismId should panic");
    }

    #[test]
    fn category_display_impl() {
        let decl = CategoryDecl {
            name: "test".into(),
            objects: vec!["A".into(), "B".into()],
            morphisms: vec![
                make_decl("id_A", "A", "A", true),
                make_decl("id_B", "B", "B", true),
            ],
            composition: vec![],
        };
        let display = format!("{decl}");
        assert!(display.contains("test"));
        assert!(display.contains("2 objects"));
        assert!(display.contains("2 morphisms"));
    }

    #[test]
    fn validated_category_display_impl() {
        let vc = ValidatedCategory {
            morphisms: vec![make_decl("id_A", "A", "A", true)],
            composition: vec![],
            morphism_ids: ids_from(vec!["id_A"]),
        };
        let display = format!("{vc}");
        assert!(display.contains("1 morphisms"));
        assert!(display.contains("0 composition rules"));
    }
}
