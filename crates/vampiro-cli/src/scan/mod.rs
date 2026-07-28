//! Git scope resolution and incremental cache for Vampiro scans.
//!
//! Determines which files to scan based on Git context, and caches extracted
//! CIR results to avoid redundant work in full-scope scans.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// ScanScope
// ---------------------------------------------------------------------------

/// What to scan and how to find the relevant files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanScope {
    /// Diff between two Git commits (or a commit and the worktree).
    Diff {
        /// Base commit (left side of the diff).
        base: String,
        /// Target commit (right side of the diff).
        target: String,
        /// Files changed between base and target.
        files: Vec<PathBuf>,
    },
    /// Scan every supported file in the repository.
    Full {
        /// All files found in the repository.
        files: Vec<PathBuf>,
    },
}

impl ScanScope {
    /// Return the list of files to scan.
    pub fn files(&self) -> &[PathBuf] {
        match self {
            ScanScope::Diff { files, .. } => files.as_slice(),
            ScanScope::Full { files } => files.as_slice(),
        }
    }

    /// Return true if this is a diff scope.
    pub fn is_diff(&self) -> bool {
        matches!(self, ScanScope::Diff { .. })
    }

    /// Base commit ID, if diff scope.
    pub fn base_commit(&self) -> Option<&str> {
        match self {
            ScanScope::Diff { base, .. } => Some(base.as_str()),
            ScanScope::Full { .. } => None,
        }
    }

    /// Target commit ID, if diff scope.
    pub fn target_commit(&self) -> Option<&str> {
        match self {
            ScanScope::Diff { target, .. } => Some(target.as_str()),
            ScanScope::Full { .. } => None,
        }
    }

    /// Number of files in scope.
    pub fn len(&self) -> usize {
        self.files().len()
    }

    /// True if no files in scope.
    pub fn is_empty(&self) -> bool {
        self.files().is_empty()
    }
}

// ---------------------------------------------------------------------------
// GitContext — resolve scan scope from a Git repository
// ---------------------------------------------------------------------------

/// Represents a Git repository context for resolving scan scopes.
pub struct GitContext {
    repo: git2::Repository,
}

/// Errors that can occur when resolving a scan scope.
#[derive(Debug)]
pub enum ScopeError {
    /// Not a Git repository.
    NotAGitRepository(String),
    /// Failed to open the repository.
    RepoOpenError(String),
    /// Failed to resolve a revision.
    RevisionError(String),
    /// Failed to fetch or access a remote reference.
    FetchError(String),
    /// The merge base between two commits could not be found (e.g. shallow clone).
    NoMergeBase(String),
    /// No diff is available (initial commit, no parent).
    NoParent(String),
    /// I/O error reading the worktree.
    IoError(String),
}

impl std::fmt::Display for ScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopeError::NotAGitRepository(p) => write!(f, "not a Git repository: {p}"),
            ScopeError::RepoOpenError(e) => write!(f, "failed to open repository: {e}"),
            ScopeError::RevisionError(e) => write!(f, "revision error: {e}"),
            ScopeError::FetchError(e) => write!(f, "fetch error: {e}"),
            ScopeError::NoMergeBase(s) => {
                write!(f, "merge base unavailable (shallow clone?): {s}")
            }
            ScopeError::NoParent(s) => write!(f, "no parent commit: {s}"),
            ScopeError::IoError(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for ScopeError {}

impl GitContext {
    /// Open a Git repository from the given path.
    ///
    /// Returns `NotAGitRepository` if the path is not inside a Git repository.
    pub fn open(path: &Path) -> Result<Self, ScopeError> {
        let repo = git2::Repository::open(path).map_err(|e| match e.code() {
            git2::ErrorCode::NotFound => ScopeError::NotAGitRepository(path.display().to_string()),
            _ => ScopeError::RepoOpenError(e.message().to_string()),
        })?;
        Ok(GitContext { repo })
    }

    /// Open a Git repository by discovering it from the current directory.
    pub fn open_from_cwd() -> Result<Self, ScopeError> {
        let cwd = std::env::current_dir().map_err(|e| ScopeError::IoError(e.to_string()))?;
        Self::open(&cwd)
    }

    /// Resolve a revision string (branch name, tag, commit SHA, etc.) to an
    /// OID. Returns `RevisionError` if the revision cannot be found.
    pub fn resolve_rev(&self, rev: &str) -> Result<git2::Oid, ScopeError> {
        let obj = self
            .repo
            .revparse_single(rev)
            .map_err(|e| ScopeError::RevisionError(format!("cannot resolve {rev}: {e}")))?;
        Ok(obj.id())
    }

    /// Resolve HEAD to its commit OID.
    pub fn head_oid(&self) -> Result<git2::Oid, ScopeError> {
        let head = self
            .repo
            .head()
            .map_err(|e| ScopeError::RevisionError(format!("no HEAD: {e}")))?;
        head.target()
            .ok_or_else(|| ScopeError::RevisionError("HEAD is not a direct reference".to_string()))
    }

    /// Get the OID of the first parent of a commit.
    /// Returns `NoParent` for the initial commit (no parent).
    pub fn first_parent(&self, oid: git2::Oid) -> Result<git2::Oid, ScopeError> {
        let commit = self
            .repo
            .find_commit(oid)
            .map_err(|e| ScopeError::RevisionError(format!("commit {oid}: {e}")))?;
        if commit.parent_count() == 0 {
            return self.empty_tree_oid();
        }
        Ok(commit.parent_id(0).unwrap())
    }

    /// Compute the merge base between two commits.
    /// Returns `NoMergeBase` if the merge base cannot be found (e.g. shallow
    /// clone with missing history).
    pub fn merge_base(&self, one: git2::Oid, two: git2::Oid) -> Result<git2::Oid, ScopeError> {
        self.repo
            .merge_base(one, two)
            .map_err(|e| ScopeError::NoMergeBase(format!("{one}..{two}: {e}")))
    }

    /// Return the OID of the empty tree.
    pub fn empty_tree_oid(&self) -> Result<git2::Oid, ScopeError> {
        let tb = self
            .repo
            .treebuilder(None)
            .map_err(|e| ScopeError::RevisionError(format!("treebuilder: {e}")))?;
        tb.write()
            .map_err(|e| ScopeError::RevisionError(format!("write empty tree: {e}")))
    }

    /// Get the diff between two trees, returning the list of changed (and
    /// newly added) file paths that end with ".rs".
    fn diff_paths(
        &self,
        old_tree: Option<&git2::Tree>,
        new_tree: Option<&git2::Tree>,
    ) -> Result<Vec<PathBuf>, ScopeError> {
        let mut diff_opts = git2::DiffOptions::new();
        // No rename detection for simplicity in v1 — exact match only.
        let diff = self
            .repo
            .diff_tree_to_tree(old_tree, new_tree, Some(&mut diff_opts))
            .map_err(|e| ScopeError::IoError(format!("diff failed: {e}")))?;

        let mut paths = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                if let Some(file) = delta.new_file().path() {
                    if file.extension().is_some_and(|e| e == "rs") {
                        paths.push(file.to_path_buf());
                    }
                }
                true
            },
            None,
            None,
            None,
        )
        .map_err(|e| ScopeError::IoError(format!("diff walk failed: {e}")))?;

        paths.sort();
        paths.dedup();
        Ok(paths)
    }

    /// Resolve the diff scope between two revisions.
    ///
    /// The `target` is the newer revision (HEAD of PR), and `base` is the
    /// older revision (merge-base or base branch).
    pub fn diff_between(&self, base_rev: &str, target_rev: &str) -> Result<ScanScope, ScopeError> {
        let base_oid = self.resolve_rev(base_rev)?;
        let target_oid = self.resolve_rev(target_rev)?;

        // Resolve base tree: try commit first, then fall back to raw tree
        // (for the empty tree sentinel which is a tree, not a commit).
        let base_tree = match self.repo.find_commit(base_oid) {
            Ok(c) => c.tree().ok(),
            Err(_) => self.repo.find_tree(base_oid).ok(),
        };

        let target_commit = self
            .repo
            .find_commit(target_oid)
            .map_err(|e| ScopeError::RevisionError(format!("target commit {target_oid}: {e}")))?;
        let target_tree = target_commit.tree().ok();

        let files = self.diff_paths(base_tree.as_ref(), target_tree.as_ref())?;

        Ok(ScanScope::Diff {
            base: base_oid.to_string(),
            target: target_oid.to_string(),
            files,
        })
    }

    /// Resolve the default local diff scope: HEAD vs worktree (including
    /// staged, unstaged, and untracked .rs files).
    pub fn local_diff(&self) -> Result<ScanScope, ScopeError> {
        let head_oid = self.head_oid()?;
        let head_commit = self
            .repo
            .find_commit(head_oid)
            .map_err(|e| ScopeError::RevisionError(format!("HEAD {head_oid}: {e}")))?;

        let head_tree = head_commit
            .tree()
            .map_err(|e| ScopeError::RevisionError(format!("HEAD tree: {e}")))?;

        // Diff HEAD tree vs worktree + index (staged + unstaged changes).
        let mut paths = Vec::new();

        // 1. Staged changes (diff HEAD vs index)
        {
            let mut opts = git2::DiffOptions::new();
            let diff = self
                .repo
                .diff_tree_to_index(Some(&head_tree), None, Some(&mut opts))
                .map_err(|e| ScopeError::IoError(format!("staged diff: {e}")))?;
            diff.foreach(
                &mut |delta, _| {
                    if let Some(file) = delta.new_file().path() {
                        if file.extension().is_some_and(|e| e == "rs") {
                            paths.push(file.to_path_buf());
                        }
                    }
                    true
                },
                None,
                None,
                None,
            )
            .map_err(|e| ScopeError::IoError(format!("staged walk: {e}")))?;
        }

        // 2. Unstaged changes (diff index vs worktree)
        {
            let mut opts = git2::DiffOptions::new();
            // Include untracked files.
            let diff = self
                .repo
                .diff_index_to_workdir(None, Some(&mut opts))
                .map_err(|e| ScopeError::IoError(format!("unstaged diff: {e}")))?;
            diff.foreach(
                &mut |delta, _| {
                    if let Some(file) = delta.new_file().path() {
                        if file.extension().is_some_and(|e| e == "rs") {
                            paths.push(file.to_path_buf());
                        }
                    }
                    true
                },
                None,
                None,
                None,
            )
            .map_err(|e| ScopeError::IoError(format!("unstaged walk: {e}")))?;
        }

        // 3. Untracked files (not in index at all)
        {
            let mut opts = git2::StatusOptions::new();
            opts.include_untracked(true);
            let statuses = self
                .repo
                .statuses(Some(&mut opts))
                .map_err(|e| ScopeError::IoError(format!("status: {e}")))?;
            for entry in statuses.iter() {
                if let Some(path) = entry.path() {
                    let p = PathBuf::from(path);
                    if p.extension().is_some_and(|e| e == "rs") {
                        paths.push(p);
                    }
                }
            }
        }

        paths.sort();
        paths.dedup();

        let base = head_oid.to_string();
        let target = format!("worktree:{}", base);

        Ok(ScanScope::Diff {
            base,
            target,
            files: paths,
        })
    }

    /// Resolve the full scope: every .rs file in the repository.
    pub fn full_scope(&self) -> Result<ScanScope, ScopeError> {
        let mut files = Vec::new();
        let _workdir = self
            .repo
            .workdir()
            .ok_or_else(|| ScopeError::NotAGitRepository("no workdir".to_string()))?;

        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true);
        let statuses = self
            .repo
            .statuses(Some(&mut opts))
            .map_err(|e| ScopeError::IoError(format!("status: {e}")))?;

        // Collect tracked .rs files from HEAD tree.
        let head_oid = self.head_oid().ok();
        if let Some(oid) = head_oid {
            if let Ok(commit) = self.repo.find_commit(oid) {
                if let Ok(tree) = commit.tree() {
                    self.collect_rs_from_tree(&tree, PathBuf::new(), &mut files);
                }
            }
        }

        // Also include untracked .rs files.
        for entry in statuses.iter() {
            if entry.status() == git2::Status::WT_NEW {
                if let Some(path) = entry.path() {
                    let p = PathBuf::from(path);
                    if p.extension().is_some_and(|e| e == "rs") {
                        files.push(p);
                    }
                }
            }
        }

        files.sort();
        files.dedup();
        Ok(ScanScope::Full { files })
    }

    fn collect_rs_from_tree(&self, tree: &git2::Tree, prefix: PathBuf, files: &mut Vec<PathBuf>) {
        for entry in tree.iter() {
            let name = entry.name().unwrap_or("");
            let path = prefix.join(name);
            if let Ok(obj) = entry.to_object(&self.repo) {
                if obj.kind() == Some(git2::ObjectType::Tree) {
                    if let Ok(subtree) = obj.peel_to_tree() {
                        self.collect_rs_from_tree(&subtree, path, files);
                    }
                } else if path.extension().is_some_and(|e| e == "rs") {
                    files.push(path);
                }
            }
        }
    }

    /// Check whether the repository is shallow.
    pub fn is_shallow(&self) -> bool {
        self.repo.path().join("shallow").exists()
    }
}

// ---------------------------------------------------------------------------
// ScanCache — versioned cache of extracted CIR results
// ---------------------------------------------------------------------------

/// A key that uniquely identifies a cached extraction result.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// SHA-256 of file content.
    pub content_hash: String,
    /// Analyzer version.
    pub analyzer_version: String,
    /// Schema version.
    pub schema_version: String,
    /// Plugin version.
    pub plugin_version: String,
    /// Configuration version.
    pub config_version: String,
}

impl CacheKey {
    /// Compute a cache key from file content and the current versions.
    pub fn from_content(
        content: &[u8],
        analyzer_version: &str,
        schema_version: &str,
        plugin_version: &str,
        config_version: &str,
    ) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let content_hash = hex::encode(hasher.finalize());

        CacheKey {
            content_hash,
            analyzer_version: analyzer_version.to_string(),
            schema_version: schema_version.to_string(),
            plugin_version: plugin_version.to_string(),
            config_version: config_version.to_string(),
        }
    }

    /// A human-readable summary of the key (for telemetry / invalidation).
    pub fn summary(&self) -> String {
        format!(
            "content:{} analyzer:{} schema:{} plugin:{} config:{}",
            &self.content_hash[..12],
            self.analyzer_version,
            self.schema_version,
            self.plugin_version,
            self.config_version,
        )
    }
}

/// A cached extraction result.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: CacheKey,
    pub data: Vec<u8>,
}

/// Thread-safe in-memory cache for extracted CIR results.
///
/// In v1 this is a simple in-memory HashMap. A future version may add
/// on-disk persistence for cross-session caching.
pub struct ScanCache {
    entries: Mutex<Vec<CacheEntry>>,
}

impl ScanCache {
    pub fn new() -> Self {
        ScanCache {
            entries: Mutex::new(Vec::new()),
        }
    }

    /// Look up a cache entry by key.
    /// Returns `Some(data)` on hit, `None` on miss.
    pub fn get(&self, key: &CacheKey) -> Option<Vec<u8>> {
        let entries = self.entries.lock().unwrap();
        entries
            .iter()
            .find(|e| e.key == *key)
            .map(|e| e.data.clone())
    }

    /// Insert a cache entry.
    pub fn insert(&self, key: CacheKey, data: Vec<u8>) {
        let mut entries = self.entries.lock().unwrap();
        // Evict any entry with the same key (content_hash only for simplicity).
        entries.retain(|e| e.key.content_hash != key.content_hash);
        entries.push(CacheEntry { key, data });
    }

    /// Return the number of entries (for telemetry).
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// Return true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all entries.
    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }
}

impl Default for ScanCache {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Metadata helpers
// ---------------------------------------------------------------------------

/// Current version constants for cache key computation.
pub mod versions {
    /// Version of the overall analyzer.
    pub const ANALYZER: &str = "0.1.0";
    /// Version of the CIR schema.
    pub const SCHEMA: &str = "0.1.0";
    /// Version of the Rust frontend plugin.
    pub const PLUGIN: &str = "0.1.0";
    /// Version of the configuration format.
    pub const CONFIG: &str = "0.1.0";
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // CacheKey tests
    // -----------------------------------------------------------------------

    #[test]
    fn cache_key_is_deterministic() {
        let k1 = CacheKey::from_content(b"hello", "1", "1", "1", "1");
        let k2 = CacheKey::from_content(b"hello", "1", "1", "1", "1");
        assert_eq!(k1, k2);
    }

    #[test]
    fn cache_key_differs_for_different_content() {
        let k1 = CacheKey::from_content(b"hello", "1", "1", "1", "1");
        let k2 = CacheKey::from_content(b"world", "1", "1", "1", "1");
        assert_ne!(k1, k2);
    }

    #[test]
    fn cache_key_differs_for_different_versions() {
        let k1 = CacheKey::from_content(b"hello", "1", "1", "1", "1");
        let k2 = CacheKey::from_content(b"hello", "2", "1", "1", "1");
        assert_ne!(k1, k2);
        let k3 = CacheKey::from_content(b"hello", "1", "2", "1", "1");
        assert_ne!(k1, k3);
    }

    // -----------------------------------------------------------------------
    // ScanCache tests
    // -----------------------------------------------------------------------

    #[test]
    fn cache_hit_and_miss() {
        let cache = ScanCache::new();
        let key = CacheKey::from_content(b"test", "1", "1", "1", "1");
        assert!(cache.get(&key).is_none());

        cache.insert(key.clone(), vec![1, 2, 3]);
        assert_eq!(cache.get(&key), Some(vec![1, 2, 3]));
    }

    #[test]
    fn cache_evicts_old_content() {
        let cache = ScanCache::new();
        let key1 = CacheKey::from_content(b"old", "1", "1", "1", "1");
        let key2 = CacheKey::from_content(b"new", "1", "1", "1", "1");
        cache.insert(key1.clone(), vec![1]);
        cache.insert(key2.clone(), vec![2]);
        assert_eq!(cache.len(), 2);

        // Re-insert with same content hash but different versions.
        let key3 = CacheKey::from_content(b"old", "2", "1", "1", "1");
        cache.insert(key3, vec![3]);
        // Should have evicted key1 (same content_hash), kept key2
        assert_eq!(cache.len(), 2);
        assert!(cache.get(&key1).is_none());
    }

    #[test]
    fn cache_summary_is_readable() {
        let key = CacheKey::from_content(b"x", "0.1.0", "0.1.0", "0.1.0", "0.1.0");
        let s = key.summary();
        assert!(s.contains("analyzer:0.1.0"));
        assert!(s.contains("schema:0.1.0"));
    }

    // -----------------------------------------------------------------------
    // GitContext tests (run in a temporary git repository)
    // -----------------------------------------------------------------------

    fn init_git_repo() -> (tempfile::TempDir, GitContext) {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();

        // Create an initial file and commit it.
        let initial_rs = dir.path().join("src/lib.rs");
        std::fs::create_dir_all(initial_rs.parent().unwrap()).unwrap();
        std::fs::write(&initial_rs, "pub fn hello() -> u32 { 42 }").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("src/lib.rs")).unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = git2::Signature::now("test", "test@test.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
        drop(tree);

        // Create a second commit with another file.
        let second_rs = dir.path().join("src/main.rs");
        std::fs::write(&second_rs, "pub fn main() { hello(); }").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("src/main.rs")).unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let parent = repo.head().unwrap().target().unwrap();
        let parent_commit = repo.find_commit(parent).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "second", &tree, &[&parent_commit])
            .unwrap();
        drop(parent_commit);
        drop(tree);

        let ctx = GitContext { repo };
        (dir, ctx)
    }

    #[test]
    fn test_open_non_existent_directory() {
        let result = GitContext::open(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_head_oid() {
        let (_dir, ctx) = init_git_repo();
        let oid = ctx.head_oid().unwrap();
        assert!(!oid.is_zero());
    }

    #[test]
    fn test_diff_between_commits() {
        let (_dir, ctx) = init_git_repo();

        let head = ctx.head_oid().unwrap();
        let first_parent = ctx.first_parent(head).unwrap();

        // Diff between first commit and HEAD should show src/main.rs as added.
        let scope = ctx
            .diff_between(&first_parent.to_string(), &head.to_string())
            .unwrap();
        assert!(scope.is_diff());
        assert_eq!(scope.base_commit(), Some(first_parent.to_string().as_str()));
        assert_eq!(scope.target_commit(), Some(head.to_string().as_str()));

        let files: Vec<&Path> = scope.files().iter().map(|p| p.as_path()).collect();
        assert!(
            files.contains(&Path::new("src/main.rs")),
            "expected src/main.rs in diff, got: {files:?}"
        );
    }

    #[test]
    fn test_local_diff_shows_modified_files() {
        let (dir, ctx) = init_git_repo();

        // Modify a tracked file.
        std::fs::write(
            dir.path().join("src/lib.rs"),
            "pub fn goodbye() -> bool { false }",
        )
        .unwrap();

        let scope = ctx.local_diff().unwrap();
        assert!(scope.is_diff());
        let files: Vec<&Path> = scope.files().iter().map(|p| p.as_path()).collect();
        assert!(
            files.contains(&Path::new("src/lib.rs")),
            "expected src/lib.rs in local diff, got: {files:?}"
        );
    }

    #[test]
    fn test_full_scope_includes_all_rs_files() {
        let (_dir, ctx) = init_git_repo();
        let scope = ctx.full_scope().unwrap();
        assert!(!scope.is_diff());
        let files: Vec<&Path> = scope.files().iter().map(|p| p.as_path()).collect();
        assert!(files.contains(&Path::new("src/lib.rs")));
        assert!(files.contains(&Path::new("src/main.rs")));
    }

    #[test]
    fn test_not_a_git_repo() {
        let dir = tempfile::tempdir().unwrap();
        let result = GitContext::open(dir.path());
        assert!(matches!(result, Err(ScopeError::NotAGitRepository(_))));
    }

    #[test]
    fn test_resolve_rev() {
        let (_dir, ctx) = init_git_repo();
        let oid = ctx.resolve_rev("HEAD").unwrap();
        assert!(!oid.is_zero());
    }

    #[test]
    fn test_resolve_nonexistent_rev() {
        let (_dir, ctx) = init_git_repo();
        let result = ctx.resolve_rev("nonexistent-branch-name-12345");
        assert!(matches!(result, Err(ScopeError::RevisionError(_))));
    }

    #[test]
    fn test_is_shallow_default_is_false() {
        let (_dir, ctx) = init_git_repo();
        assert!(!ctx.is_shallow());
    }
}
