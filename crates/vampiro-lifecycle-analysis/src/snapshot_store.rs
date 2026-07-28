//! On-disk snapshot persistence for facade evolution history (REQ-T1).
//!
//! Stores and retrieves [`FacadeSnapshot`] files under
//! `.vampiro/snapshots/v0.1.0/<commit-sha>.json` following the approved
//! storage contract from `docs/decisions/lifecycle-storage.md`.

use std::path::{Path, PathBuf};
use std::fs;

use crate::facade_history::{FacadeHistoryError, FacadeSnapshot};

/// Current snapshot schema version — matches `FACADE_SNAPSHOT_SCHEMA_VERSION`.
pub const SNAPSHOT_DIR_VERSION: &str = "0.1.0";

/// Manages reading and writing facade snapshots on disk.
#[derive(Debug, Clone)]
pub struct SnapshotStore {
    /// Root directory for snapshots (e.g. `.vampiro/snapshots`).
    root: PathBuf,
}

impl SnapshotStore {
    /// Create a new snapshot store rooted at `base_path/.vampiro/snapshots`.
    ///
    /// Creates the full directory tree (including the version subdirectory)
    /// if it doesn't exist.
    pub fn new(base_path: &Path) -> Result<Self, FacadeHistoryError> {
        let root = base_path.join(".vampiro").join("snapshots");
        let version_dir = root.join(SNAPSHOT_DIR_VERSION);
        if !version_dir.exists() {
            fs::create_dir_all(&version_dir)
                .map_err(|e| FacadeHistoryError::IoError(
                    std::io::Error::new(e.kind(), format!("failed to create {version_dir:?}: {e}"))
                ))?;
        }
        Ok(SnapshotStore { root })
    }

    /// Get the directory for the current schema version.
    fn version_dir(&self) -> PathBuf {
        self.root.join(SNAPSHOT_DIR_VERSION)
    }

    /// Get the file path for a given commit SHA.
    fn snapshot_path(&self, commit_sha: &str) -> PathBuf {
        self.version_dir().join(format!("{commit_sha}.json"))
    }

    /// Write a snapshot for a commit. Creates the version subdirectory if
    /// needed. Overwrites any existing snapshot for the same commit.
    pub fn write_snapshot(&self, snapshot: &FacadeSnapshot) -> Result<(), FacadeHistoryError> {
        let version_dir = self.version_dir();
        if !version_dir.exists() {
            fs::create_dir_all(&version_dir)
                .map_err(|e| FacadeHistoryError::IoError(
                    std::io::Error::new(e.kind(), format!("failed to create {version_dir:?}: {e}"))
                ))?;
        }

        let path = self.snapshot_path(&snapshot.commit_sha);
        let json = serde_json::to_string_pretty(snapshot)
            .map_err(|e| FacadeHistoryError::IoError(
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            ))?;
        fs::write(&path, json.as_bytes())?;
        Ok(())
    }

    /// Read a snapshot for the given commit SHA.
    ///
    /// Returns `NoSnapshot` if no file exists for that commit.
    pub fn read_snapshot(&self, commit_sha: &str) -> Result<FacadeSnapshot, FacadeHistoryError> {
        let path = self.snapshot_path(commit_sha);
        if !path.exists() {
            return Err(FacadeHistoryError::NoSnapshot(commit_sha.to_string()));
        }
        let data = fs::read_to_string(&path)?;
        let mut snapshot: FacadeSnapshot = serde_json::from_str(&data)
            .map_err(|e| FacadeHistoryError::IoError(
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            ))?;
        snapshot.rebuild_index();
        Ok(snapshot)
    }

    /// Check if a snapshot exists for the given commit SHA.
    pub fn has_snapshot(&self, commit_sha: &str) -> bool {
        self.snapshot_path(commit_sha).exists()
    }

    /// List all commit SHAs that have snapshots stored (sorted).
    pub fn list_snapshots(&self) -> Result<Vec<String>, FacadeHistoryError> {
        let version_dir = self.version_dir();
        if !version_dir.exists() {
            return Ok(Vec::new());
        }
        let mut shas = Vec::new();
        for entry in fs::read_dir(&version_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    shas.push(stem.to_string());
                }
            }
        }
        shas.sort();
        Ok(shas)
    }

    /// Delete a snapshot for the given commit SHA.
    pub fn delete_snapshot(&self, commit_sha: &str) -> Result<(), FacadeHistoryError> {
        let path = self.snapshot_path(commit_sha);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facade_history::{FacadeItem, hash_shape};

    #[test]
    fn store_creates_directory() {
        let dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(dir.path()).unwrap();
        assert!(store.version_dir().exists());
    }

    #[test]
    fn write_and_read_snapshot_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(dir.path()).unwrap();

        let mut snap = FacadeSnapshot::new("abc123def456");
        snap.add_item(FacadeItem::new(
            "my_crate::foo",
            hash_shape("() -> u32"),
            "() -> u32",
        ).with_source("src/lib.rs", 42));

        store.write_snapshot(&snap).unwrap();

        let loaded = store.read_snapshot("abc123def456").unwrap();
        assert_eq!(loaded.commit_sha, "abc123def456");
        assert_eq!(loaded.items.len(), 1);
        assert_eq!(loaded.get("my_crate::foo").unwrap().qualified_name, "my_crate::foo");
    }

    #[test]
    fn read_missing_snapshot_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(dir.path()).unwrap();
        let err = store.read_snapshot("nonexistent").unwrap_err();
        match err {
            FacadeHistoryError::NoSnapshot(sha) => assert_eq!(sha, "nonexistent"),
            other => panic!("expected NoSnapshot, got: {other:?}"),
        }
    }

    #[test]
    fn has_snapshot_detects_presence() {
        let dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(dir.path()).unwrap();

        assert!(!store.has_snapshot("abc"));
        let snap = FacadeSnapshot::new("abc");
        store.write_snapshot(&snap).unwrap();
        assert!(store.has_snapshot("abc"));
    }

    #[test]
    fn overwrite_existing_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(dir.path()).unwrap();

        let snap1 = FacadeSnapshot::new("abc");
        store.write_snapshot(&snap1).unwrap();

        let mut snap2 = FacadeSnapshot::new("abc");
        snap2.add_item(FacadeItem::new(
            "x",
            hash_shape("() -> u32"),
            "() -> u32",
        ));
        store.write_snapshot(&snap2).unwrap();

        let loaded = store.read_snapshot("abc").unwrap();
        assert_eq!(loaded.items.len(), 1); // overwritten
    }

    #[test]
    fn list_snapshots_returns_sorted_shas() {
        let dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(dir.path()).unwrap();

        store.write_snapshot(&FacadeSnapshot::new("ccc")).unwrap();
        store.write_snapshot(&FacadeSnapshot::new("aaa")).unwrap();
        store.write_snapshot(&FacadeSnapshot::new("bbb")).unwrap();

        let shas = store.list_snapshots().unwrap();
        assert_eq!(shas, vec!["aaa", "bbb", "ccc"]);
    }

    #[test]
    fn list_empty_store_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(dir.path()).unwrap();
        assert!(store.list_snapshots().unwrap().is_empty());
    }

    #[test]
    fn delete_snapshot_removes_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = SnapshotStore::new(dir.path()).unwrap();

        store.write_snapshot(&FacadeSnapshot::new("abc")).unwrap();
        assert!(store.has_snapshot("abc"));

        store.delete_snapshot("abc").unwrap();
        assert!(!store.has_snapshot("abc"));
    }
}