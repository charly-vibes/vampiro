//! Write-shape idiom table for retry idempotency classification (REQ-T2, REQ-T9).
//!
//! Classifies write operations as `Idempotent`, `NonIdempotent`, or `Unknown`
//! based on a versioned, conformance-tested idiom table matching the mechanism
//! of REQ-3's effect idiom table.

use serde::{Deserialize, Serialize};

/// The current write-idiom table schema version.
pub const WRITE_IDIOM_SCHEMA_VERSION: &str = "0.1.0";

// ---------------------------------------------------------------------------
// IdempotencyClass
// ---------------------------------------------------------------------------

/// The idempotency class of a write operation (REQ-T2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdempotencyClass {
    /// Applying the operation twice has the same effect as applying it once.
    Idempotent,
    /// Applying the operation twice may produce different state.
    NonIdempotent,
    /// No matching idiom table entry — coverage diagnostic (REQ-T9).
    Unknown,
}

impl std::fmt::Display for IdempotencyClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdempotencyClass::Idempotent => f.write_str("idempotent"),
            IdempotencyClass::NonIdempotent => f.write_str("non-idempotent"),
            IdempotencyClass::Unknown => f.write_str("unknown"),
        }
    }
}

// ---------------------------------------------------------------------------
// WriteIdiomEntry
// ---------------------------------------------------------------------------

/// One entry in the write-shape idiom table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WriteIdiomEntry {
    /// Human-readable pattern description (e.g. "INSERT INTO ... ON CONFLICT").
    pub pattern: String,
    /// The idempotency classification.
    pub classification: IdempotencyClass,
    /// Pattern matching keywords / method names.
    pub keywords: Vec<String>,
    /// Optional notes about this pattern.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl WriteIdiomEntry {
    pub fn new(
        pattern: impl Into<String>,
        classification: IdempotencyClass,
        keywords: Vec<impl Into<String>>,
    ) -> Self {
        WriteIdiomEntry {
            pattern: pattern.into(),
            classification,
            keywords: keywords.into_iter().map(|k| k.into()).collect(),
            notes: None,
        }
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

// ---------------------------------------------------------------------------
// WriteIdiomTable
// ---------------------------------------------------------------------------

/// A versioned write-shape idiom table (REQ-T2, REQ-T9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WriteIdiomTable {
    /// Schema version.
    pub schema_version: String,
    /// Version of this idiom table (incremented on any entry change).
    pub table_version: String,
    /// The entries in this table.
    pub entries: Vec<WriteIdiomEntry>,
}

impl WriteIdiomTable {
    /// Create a new empty table.
    pub fn new(table_version: impl Into<String>) -> Self {
        WriteIdiomTable {
            schema_version: WRITE_IDIOM_SCHEMA_VERSION.to_string(),
            table_version: table_version.into(),
            entries: Vec::new(),
        }
    }

    /// Classify a write operation by its method/function name.
    ///
    /// Returns `Unknown` if no entry matches (REQ-T9).
    pub fn classify(&self, method_name: &str) -> IdempotencyClass {
        for entry in &self.entries {
            if entry.keywords.iter().any(|k| k == method_name) {
                return entry.classification;
            }
        }
        IdempotencyClass::Unknown
    }

    /// Register an entry.
    pub fn add_entry(&mut self, entry: WriteIdiomEntry) {
        self.entries.push(entry);
    }
}

// ---------------------------------------------------------------------------
// Built-in v0.1.0 table
// ---------------------------------------------------------------------------

/// Return the built-in v0.1.0 write-idiom table.
pub fn builtin_write_idiom_table_v0_1_0() -> WriteIdiomTable {
    let mut table = WriteIdiomTable::new("0.1.0");

    // SQL / database patterns
    table.add_entry(WriteIdiomEntry::new(
        "INSERT INTO",
        IdempotencyClass::NonIdempotent,
        vec!["insert", "INSERT"],
    ));
    table.add_entry(WriteIdiomEntry::new(
        "INSERT ... ON CONFLICT (upsert)",
        IdempotencyClass::Idempotent,
        vec!["upsert", "insert_on_conflict", "ON CONFLICT"],
    ));
    table.add_entry(WriteIdiomEntry::new(
        "UPDATE ... WHERE pk = ?",
        IdempotencyClass::Idempotent,
        vec!["update", "UPDATE"],
    ));
    table.add_entry(WriteIdiomEntry::new(
        "DELETE FROM ... WHERE pk = ?",
        IdempotencyClass::Idempotent,
        vec!["delete", "DELETE"],
    ));

    // HTTP / API patterns
    table.add_entry(WriteIdiomEntry::new(
        "PUT (full replacement)",
        IdempotencyClass::Idempotent,
        vec!["put", "PUT"],
    ));
    table.add_entry(WriteIdiomEntry::new(
        "PATCH (merge)",
        IdempotencyClass::NonIdempotent,
        vec!["patch", "PATCH"],
    ));

    // Filesystem patterns
    table.add_entry(WriteIdiomEntry::new(
        "fs::write (plain file write)",
        IdempotencyClass::NonIdempotent,
        vec!["write", "fs::write", "std::fs::write"],
    ));
    table.add_entry(WriteIdiomEntry::new(
        "fs::OpenOptions::append",
        IdempotencyClass::NonIdempotent,
        vec!["append", "OpenOptions::append"],
    ));

    // In-memory data structures
    table.add_entry(WriteIdiomEntry::new(
        "HashMap::insert / BTreeMap::insert",
        IdempotencyClass::Idempotent,
        vec!["HashMap::insert", "BTreeMap::insert", "insert"],
    ));
    table.add_entry(WriteIdiomEntry::new(
        "Vec::push / list append",
        IdempotencyClass::NonIdempotent,
        vec!["Vec::push", "push", "push_back"],
    ));

    // Redis / key-value patterns
    table.add_entry(WriteIdiomEntry::new(
        "SET key value",
        IdempotencyClass::Idempotent,
        vec!["SET", "set"],
    ));

    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_schema_version_is_0_1_0() {
        assert_eq!(WRITE_IDIOM_SCHEMA_VERSION, "0.1.0");
    }

    #[test]
    fn builtin_table_has_entries() {
        let table = builtin_write_idiom_table_v0_1_0();
        assert!(!table.entries.is_empty());
        assert_eq!(table.table_version, "0.1.0");
    }

    #[test]
    fn classify_insert_as_non_idempotent() {
        let table = builtin_write_idiom_table_v0_1_0();
        assert_eq!(table.classify("insert"), IdempotencyClass::NonIdempotent);
    }

    #[test]
    fn classify_upsert_as_idempotent() {
        let table = builtin_write_idiom_table_v0_1_0();
        assert_eq!(table.classify("upsert"), IdempotencyClass::Idempotent);
    }

    #[test]
    fn classify_update_as_idempotent() {
        let table = builtin_write_idiom_table_v0_1_0();
        assert_eq!(table.classify("update"), IdempotencyClass::Idempotent);
    }

    #[test]
    fn classify_delete_as_idempotent() {
        let table = builtin_write_idiom_table_v0_1_0();
        assert_eq!(table.classify("delete"), IdempotencyClass::Idempotent);
    }

    #[test]
    fn classify_put_as_idempotent() {
        let table = builtin_write_idiom_table_v0_1_0();
        assert_eq!(table.classify("put"), IdempotencyClass::Idempotent);
    }

    #[test]
    fn classify_patch_as_non_idempotent() {
        let table = builtin_write_idiom_table_v0_1_0();
        assert_eq!(table.classify("patch"), IdempotencyClass::NonIdempotent);
    }

    #[test]
    fn classify_fs_write_as_non_idempotent() {
        let table = builtin_write_idiom_table_v0_1_0();
        assert_eq!(table.classify("fs::write"), IdempotencyClass::NonIdempotent);
    }

    #[test]
    fn classify_vec_push_as_non_idempotent() {
        let table = builtin_write_idiom_table_v0_1_0();
        assert_eq!(table.classify("push"), IdempotencyClass::NonIdempotent);
    }

    #[test]
    fn classify_hashmap_insert_as_idempotent() {
        let table = builtin_write_idiom_table_v0_1_0();
        assert_eq!(
            table.classify("HashMap::insert"),
            IdempotencyClass::Idempotent
        );
    }

    #[test]
    fn classify_unknown_returns_unknown() {
        let table = builtin_write_idiom_table_v0_1_0();
        assert_eq!(
            table.classify("custom_unknown_fn"),
            IdempotencyClass::Unknown
        );
    }

    #[test]
    fn classify_set_as_idempotent() {
        let table = builtin_write_idiom_table_v0_1_0();
        assert_eq!(table.classify("set"), IdempotencyClass::Idempotent);
    }

    #[test]
    fn table_serialization_roundtrip() {
        let table = builtin_write_idiom_table_v0_1_0();
        let json = serde_json::to_string_pretty(&table).unwrap();
        let deserialized: WriteIdiomTable = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.table_version, "0.1.0");
        assert_eq!(deserialized.entries.len(), table.entries.len());
    }

    #[test]
    fn empty_table_classifies_all_as_unknown() {
        let table = WriteIdiomTable::new("0.1.0");
        assert_eq!(table.classify("insert"), IdempotencyClass::Unknown);
        assert_eq!(table.classify("anything"), IdempotencyClass::Unknown);
    }

    #[test]
    fn idempotency_class_display() {
        assert_eq!(IdempotencyClass::Idempotent.to_string(), "idempotent");
        assert_eq!(
            IdempotencyClass::NonIdempotent.to_string(),
            "non-idempotent"
        );
        assert_eq!(IdempotencyClass::Unknown.to_string(), "unknown");
    }
}
