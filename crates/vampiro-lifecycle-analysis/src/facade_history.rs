//! Facade snapshot types and history analysis (REQ-T1, REQ-T4, REQ-T8).
//!
//! A facade snapshot records the `L4`-level declarations at one commit.
//! The history analyzer compares two snapshots and emits findings for
//! breaking changes, identity ambiguity, and migration-authorized changes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The current facade snapshot schema version.
pub const FACADE_SNAPSHOT_SCHEMA_VERSION: &str = "0.1.0";

// ---------------------------------------------------------------------------
// FacadeItem
// ---------------------------------------------------------------------------

/// One L4 facade declaration in a snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FacadeItem {
    /// Fully qualified name (e.g. `"my_crate::pricing::Tier::charge"`).
    pub qualified_name: String,
    /// SHA-256 hash of the normalized domain+codomain shapes (used for
    /// fast comparison instead of storing full shapes).
    pub shape_hash: String,
    /// Known previous names (renamed from) for identity resolution (REQ-T8).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Human-readable shape description for diagnostic output.
    pub shape_description: String,
    /// Source file path relative to workspace root.
    pub source_file: Option<String>,
    /// Source line range.
    pub source_line: Option<usize>,
}

impl FacadeItem {
    pub fn new(
        qualified_name: impl Into<String>,
        shape_hash: impl Into<String>,
        shape_description: impl Into<String>,
    ) -> Self {
        FacadeItem {
            qualified_name: qualified_name.into(),
            shape_hash: shape_hash.into(),
            aliases: Vec::new(),
            shape_description: shape_description.into(),
            source_file: None,
            source_line: None,
        }
    }

    /// Set the source location for this item.
    pub fn with_source(mut self, file: impl Into<String>, line: usize) -> Self {
        self.source_file = Some(file.into());
        self.source_line = Some(line);
        self
    }

    /// Add an alias (previous name) for identity resolution.
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.push(alias.into());
        self
    }
}

// ---------------------------------------------------------------------------
// FacadeSnapshot
// ---------------------------------------------------------------------------

/// A persisted facade snapshot for one analyzed commit (REQ-T1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FacadeSnapshot {
    /// Snapshot schema version.
    pub schema_version: String,
    /// The commit SHA this snapshot was computed from.
    pub commit_sha: String,
    /// All L4 facade items.
    pub items: Vec<FacadeItem>,
    /// Map from qualified name to index in items (for fast lookup).
    #[serde(skip)]
    name_index: HashMap<String, usize>,
}

impl FacadeSnapshot {
    /// Create a new empty snapshot for the given commit.
    pub fn new(commit_sha: impl Into<String>) -> Self {
        FacadeSnapshot {
            schema_version: FACADE_SNAPSHOT_SCHEMA_VERSION.to_string(),
            commit_sha: commit_sha.into(),
            items: Vec::new(),
            name_index: HashMap::new(),
        }
    }

    /// Add a facade item.
    pub fn add_item(&mut self, item: FacadeItem) {
        let name = item.qualified_name.clone();
        self.name_index.insert(name, self.items.len());
        self.items.push(item);
    }

    /// Look up an item by qualified name.
    pub fn get(&self, name: &str) -> Option<&FacadeItem> {
        self.name_index
            .get(name)
            .and_then(|&i| self.items.get(i))
    }

    /// Rebuild the name index (call after deserialization).
    pub fn rebuild_index(&mut self) {
        self.name_index.clear();
        for (i, item) in self.items.iter().enumerate() {
            self.name_index
                .insert(item.qualified_name.clone(), i);
        }
    }

    /// Find an item by any known name (qualified name or alias).
    pub fn resolve_name(&self, name: &str) -> Option<&FacadeItem> {
        if let Some(item) = self.get(name) {
            return Some(item);
        }
        // Search aliases
        self.items.iter().find(|item| item.aliases.contains(&name.to_string()))
    }

    /// Return the set of all qualified names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.items.iter().map(|item| item.qualified_name.as_str())
    }
}

// ---------------------------------------------------------------------------
// BreakingChange — a finding from comparing two snapshots
// ---------------------------------------------------------------------------

/// The result of comparing two facade snapshots for a single item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComparisonResult {
    /// Item persisted identically — no finding.
    Unchanged,
    /// Item changed shape (REQ-T4) but is authorized by migration — no finding.
    Migrated { migration_id: String },
    /// Item changed shape — breaking change finding.
    BreakingChange {
        qualified_name: String,
        old_shape_hash: String,
        new_shape_hash: String,
    },
    /// Item was removed — no finding (we only detect shape changes, not
    /// removal, per REQ-T4).
    Removed,
    /// Item was added — no finding (first snapshot establishes baseline).
    Added,
}

// ---------------------------------------------------------------------------
// FacadeHistoryAnalyzer
// ---------------------------------------------------------------------------

/// Compares two facade snapshots and produces findings for breaking changes.
///
/// Operates on already-loaded snapshots. The store handles persistence.
#[derive(Debug, Clone, Default)]
pub struct FacadeHistoryAnalyzer {
    /// Migrations: qualified name -> migration ID.
    migrations: HashMap<String, String>,
}

impl FacadeHistoryAnalyzer {
    /// Create a new analyzer with optional migration declarations.
    pub fn new(migrations: HashMap<String, String>) -> Self {
        FacadeHistoryAnalyzer { migrations }
    }

    /// Compare two snapshots and return a comparison result for each item in
    /// `newer` that also exists in `older`.
    pub fn compare(&self, older: &FacadeSnapshot, newer: &FacadeSnapshot) -> Vec<ComparisonResult> {
        let mut results = Vec::new();

        for new_item in &newer.items {
            // Try to find the item in the old snapshot:
            // 1. By qualified name directly
            // 2. By any of the new item's aliases (it was renamed)
            let old_item = older
                .resolve_name(&new_item.qualified_name)
                .or_else(|| {
                    new_item
                        .aliases
                        .iter()
                        .find_map(|alias| older.resolve_name(alias))
                });

            let old_item = match old_item {
                Some(item) => item,
                None => continue, // Added item — no finding
            };

            if old_item.shape_hash == new_item.shape_hash {
                results.push(ComparisonResult::Unchanged);
            } else if let Some(migration_id) = self.migrations.get(&new_item.qualified_name) {
                results.push(ComparisonResult::Migrated {
                    migration_id: migration_id.clone(),
                });
            } else {
                results.push(ComparisonResult::BreakingChange {
                    qualified_name: new_item.qualified_name.clone(),
                    old_shape_hash: old_item.shape_hash.clone(),
                    new_shape_hash: new_item.shape_hash.clone(),
                });
            }
        }

        results
    }

    /// Check for ambiguous identity: items in `older` not found in `newer`
    /// by qualified name or alias (REQ-T8).
    pub fn find_ambiguous(&self, older: &FacadeSnapshot, newer: &FacadeSnapshot) -> Vec<String> {
        let mut ambiguous = Vec::new();
        for old_item in &older.items {
            let found = newer.resolve_name(&old_item.qualified_name).is_some()
                || old_item
                    .aliases
                    .iter()
                    .any(|alias| newer.resolve_name(alias).is_some());

            if !found {
                ambiguous.push(old_item.qualified_name.clone());
            }
        }
        ambiguous
    }
}

/// Convenience: compute a SHA-256 hash from shape description bytes.
pub fn hash_shape(description: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(description.as_bytes());
    hex::encode(hasher.finalize())
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors from the facade history analyzer.
#[derive(Debug)]
pub enum FacadeHistoryError {
    /// No snapshot found for the requested baseline.
    NoSnapshot(String),
    /// The requested baseline is not an ancestor of the target.
    NotAncestor(String),
    /// I/O error reading/writing snapshots.
    IoError(std::io::Error),
}

impl std::fmt::Display for FacadeHistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FacadeHistoryError::NoSnapshot(sha) => {
                write!(f, "no snapshot found for baseline {sha}")
            }
            FacadeHistoryError::NotAncestor(msg) => write!(f, "{msg}"),
            FacadeHistoryError::IoError(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for FacadeHistoryError {}

impl From<std::io::Error> for FacadeHistoryError {
    fn from(e: std::io::Error) -> Self {
        FacadeHistoryError::IoError(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(
        commit: &str,
        items: Vec<(&str, &str, &str)>,
    ) -> FacadeSnapshot {
        // items: (qualified_name, shape_description, alias?)
        let mut snap = FacadeSnapshot::new(commit);
        for (name, desc, alias) in items {
            let hash = hash_shape(desc);
            let mut item = FacadeItem::new(name, hash, desc);
            if !alias.is_empty() {
                item = item.with_alias(alias);
            }
            snap.add_item(item);
        }
        snap
    }

    fn analyzer() -> FacadeHistoryAnalyzer {
        FacadeHistoryAnalyzer::new(HashMap::new())
    }

    // -----------------------------------------------------------------------
    // FacadeSnapshot basics
    // -----------------------------------------------------------------------

    #[test]
    fn empty_snapshot_has_schema_version() {
        let snap = FacadeSnapshot::new("abc123");
        assert_eq!(snap.schema_version, "0.1.0");
        assert_eq!(snap.commit_sha, "abc123");
        assert!(snap.items.is_empty());
    }

    #[test]
    fn snapshot_add_and_get_item() {
        let mut snap = FacadeSnapshot::new("abc");
        snap.add_item(FacadeItem::new(
            "my_crate::foo",
            hash_shape("() -> u32"),
            "() -> u32",
        ));
        let item = snap.get("my_crate::foo").unwrap();
        assert_eq!(item.qualified_name, "my_crate::foo");
        assert!(snap.get("nonexistent").is_none());
    }

    #[test]
    fn snapshot_rebuild_index_after_deserialization() {
        let mut snap = FacadeSnapshot::new("abc");
        snap.add_item(FacadeItem::new("a", "h1", "desc1"));
        let json = serde_json::to_string(&snap).unwrap();
        let mut deserialized: FacadeSnapshot = serde_json::from_str(&json).unwrap();
        // Index is not serialized, so get() returns None before rebuild.
        assert!(deserialized.get("a").is_none());
        deserialized.rebuild_index();
        assert_eq!(deserialized.get("a").unwrap().qualified_name, "a");
    }

    // -----------------------------------------------------------------------
    // Comparison: first snapshot (no baseline)
    // -----------------------------------------------------------------------

    #[test]
    fn first_snapshot_establishes_baseline_no_findings() {
        // REQ-T1: "On the first snapshot it SHALL persist the baseline and
        // emit no breaking findings."
        let snap = make_snapshot("abc123", vec![("a::foo", "() -> u32", "")]);
        let analyzer = analyzer();

        // No older snapshot to compare — no findings.
        // This is handled at orchestration level, not at comparison level.
        // The comparison with an empty snapshot represents the first-analysis case.
        let older = FacadeSnapshot::new("empty");
        let results = analyzer.compare(&older, &snap);
        assert!(results.is_empty());
    }

    // -----------------------------------------------------------------------
    // Comparison: breaking shape change (REQ-T4)
    // -----------------------------------------------------------------------

    #[test]
    fn breaking_shape_change_produces_finding() {
        // REQ-T4: shape change without migration raises breaking-change finding.
        let older = make_snapshot(
            "abc",
            vec![("a::foo", "() -> u32", "")],
        );
        let mut newer = FacadeSnapshot::new("def");
        newer.add_item(FacadeItem::new(
            "a::foo",
            hash_shape("(&str) -> u64"),
            "(&str) -> u64",
        ));

        let results = analyzer().compare(&older, &newer);
        assert_eq!(results.len(), 1);
        match &results[0] {
            ComparisonResult::BreakingChange {
                qualified_name,
                old_shape_hash,
                new_shape_hash: _,
            } => {
                assert_eq!(qualified_name, "a::foo");
                assert_eq!(*old_shape_hash, hash_shape("() -> u32"));
            }
            other => panic!("expected breaking change, got: {other:?}"),
        }
    }

    #[test]
    fn unchanged_item_produces_no_finding() {
        let older = make_snapshot("abc", vec![("a::foo", "() -> u32", "")]);
        let newer = make_snapshot("def", vec![("a::foo", "() -> u32", "")]);

        let results = analyzer().compare(&older, &newer);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], ComparisonResult::Unchanged);
    }

    #[test]
    fn added_item_produces_no_finding() {
        let older = make_snapshot("abc", vec![("a::foo", "() -> u32", "")]);
        let newer = make_snapshot(
            "def",
            vec![
                ("a::foo", "() -> u32", ""),
                ("a::bar", "() -> bool", ""),
            ],
        );

        let results = analyzer().compare(&older, &newer);
        // Only "a::foo" is compared; "a::bar" is added, which is not a breaking change
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], ComparisonResult::Unchanged);
    }

    // -----------------------------------------------------------------------
    // Comparison: migration authorization
    // -----------------------------------------------------------------------

    #[test]
    fn breaking_change_with_migration_produces_migrated_result() {
        let mut migrations = HashMap::new();
        migrations.insert(
            "a::foo".to_string(),
            "breaking-change-v0.2.0".to_string(),
        );
        let analyzer = FacadeHistoryAnalyzer::new(migrations);

        let older = make_snapshot("abc", vec![("a::foo", "() -> u32", "")]);
        let newer = make_snapshot(
            "def",
            vec![("a::foo", "(&str) -> u64", "")],
        );

        let results = analyzer.compare(&older, &newer);
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            ComparisonResult::Migrated {
                migration_id: "breaking-change-v0.2.0".to_string()
            }
        );
    }

    // -----------------------------------------------------------------------
    // Alias-based identity resolution (REQ-T8)
    // -----------------------------------------------------------------------

    #[test]
    fn renamed_item_resolved_via_alias() {
        let older = make_snapshot("abc", vec![("a::foo", "() -> u32", "")]);
        let mut newer = FacadeSnapshot::new("def");
        // Item was renamed from "a::foo" to "a::bar" with an alias.
        let mut item = FacadeItem::new("a::bar", hash_shape("() -> u32"), "() -> u32");
        item = item.with_alias("a::foo");
        newer.add_item(item);

        // Should resolve via alias — no breaking change since shape is same.
        let results = analyzer().compare(&older, &newer);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], ComparisonResult::Unchanged);
    }

    #[test]
    fn ambiguous_identity_when_renamed_without_alias() {
        // REQ-T8: renamed/moved without declared alias -> identity:ambiguous
        let older = make_snapshot("abc", vec![("a::foo", "() -> u32", "")]);
        let newer = make_snapshot(
            "def",
            vec![("a::bar", "() -> u32", "")], // no alias to "a::foo"
        );

        let ambiguous = analyzer().find_ambiguous(&older, &newer);
        assert_eq!(ambiguous, vec!["a::foo"]);
    }

    #[test]
    fn no_ambiguous_identity_when_alias_present() {
        let older = make_snapshot("abc", vec![("a::foo", "() -> u32", "")]);
        let mut newer = FacadeSnapshot::new("def");
        let mut item = FacadeItem::new("a::bar", hash_shape("() -> u32"), "() -> u32");
        item = item.with_alias("a::foo");
        newer.add_item(item);

        let ambiguous = analyzer().find_ambiguous(&older, &newer);
        assert!(ambiguous.is_empty(), "expected no ambiguous, got: {ambiguous:?}");
    }

    // -----------------------------------------------------------------------
    // Deterministic snapshot identity
    // -----------------------------------------------------------------------

    #[test]
    fn multiple_items_all_comparison() {
        let older = make_snapshot(
            "abc",
            vec![
                ("a::foo", "() -> u32", ""),
                ("a::bar", "(&str) -> String", ""),
                ("a::baz", "(u32) -> bool", ""),
            ],
        );
        let newer = make_snapshot(
            "def",
            vec![
                ("a::foo", "() -> u32", ""),           // unchanged
                ("a::bar", "(&str) -> u64", ""),        // breaking change
                ("a::baz", "(u32) -> bool", ""),        // unchanged
            ],
        );

        let results = analyzer().compare(&older, &newer);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], ComparisonResult::Unchanged);

        match &results[1] {
            ComparisonResult::BreakingChange { qualified_name, .. } => {
                assert_eq!(qualified_name, "a::bar");
            }
            other => panic!("expected breaking change, got: {other:?}"),
        }

        assert_eq!(results[2], ComparisonResult::Unchanged);
    }

    // -----------------------------------------------------------------------
    // Serialization round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn snapshot_serialization_roundtrip() {
        let mut snap = FacadeSnapshot::new("abc123");
        snap.add_item(FacadeItem::new(
            "my_crate::foo",
            hash_shape("() -> u32"),
            "() -> u32",
        ));
        snap.add_item(FacadeItem::new(
            "my_crate::bar",
            hash_shape("(&str) -> String"),
            "(&str) -> String",
        ));

        let json = serde_json::to_string_pretty(&snap).unwrap();
        let mut deserialized: FacadeSnapshot = serde_json::from_str(&json).unwrap();
        deserialized.rebuild_index();

        assert_eq!(deserialized.commit_sha, "abc123");
        assert_eq!(deserialized.items.len(), 2);
        assert_eq!(deserialized.get("my_crate::foo").unwrap().qualified_name, "my_crate::foo");
        assert_eq!(deserialized.get("my_crate::bar").unwrap().qualified_name, "my_crate::bar");
    }

    // -----------------------------------------------------------------------
    // Hash deterministic
    // -----------------------------------------------------------------------

    #[test]
    fn hash_is_deterministic() {
        let h1 = hash_shape("() -> u32");
        let h2 = hash_shape("() -> u32");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_differs_for_different_shapes() {
        let h1 = hash_shape("() -> u32");
        let h2 = hash_shape("() -> u64");
        assert_ne!(h1, h2);
    }

    // -----------------------------------------------------------------------
    // Snapshot with source info
    // -----------------------------------------------------------------------

    #[test]
    fn item_with_source_location() {
        let mut snap = FacadeSnapshot::new("abc");
        snap.add_item(
            FacadeItem::new("a::foo", hash_shape("() -> u32"), "() -> u32")
                .with_source("src/lib.rs", 42),
        );
        let item = snap.get("a::foo").unwrap();
        assert_eq!(item.source_file.as_deref(), Some("src/lib.rs"));
        assert_eq!(item.source_line, Some(42));
    }
}