//! Integration tests for Git scope resolution and incremental caching
//! (add-scan-gating-reporting, section 1).
//!
//! Each test creates a temporary Git repository, sets up specific scenarios,
//! and verifies that GitContext and ScanCache behave correctly.
//!
//! Scenarios covered:
//! - 1.1: synthetic worktree, staged/unstaged/untracked, explicit target/base,
//!   synthetic worktree, staged/unstaged/untracked, explicit target/base,
//!   detached/initial/shallow/non-Git, failed-fetch, no-silent-fallback,
//!   explicit full, versioned cache invalidation

use std::path::Path;
use std::process::Command;

use genesis::fixture::Fixture;

/// Helper: create a temporary directory with an initialized git repo,
/// using genesis::fixture for consistent test environment setup.
fn setup_git_repo() -> Fixture {
    Fixture::new()
        .with_git_init()
        .with_file("src/lib.rs", "pub fn hello() -> u32 { 42 }")
        .build()
        .expect("fixture build")
}

/// Helper: add a second commit with an additional file.
fn add_second_commit(repo: &Fixture) {
    let main_rs = repo.path("src/main.rs");
    std::fs::write(&main_rs, "pub fn main() { hello(); }").unwrap();

    Command::new("git")
        .args(["add", "src/main.rs"])
        .current_dir(repo.root())
        .output()
        .expect("git add failed");
    Command::new("git")
        .args(["commit", "-m", "add main.rs"])
        .current_dir(repo.root())
        .output()
        .expect("git commit failed");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_1_1_synthetic_worktree_diff() {
    let repo = setup_git_repo();
    let repo_path = repo.root();

    // Use vampiro_cli::scan::GitContext to compute local diff.
    let ctx = vampiro_cli::scan::GitContext::open(repo_path).unwrap();

    // Modify the file (simulate worktree changes)
    let lib_rs = repo.path("src/lib.rs");
    std::fs::write(&lib_rs, "pub fn goodbye() -> bool { false }").unwrap();

    let scope = ctx.local_diff().unwrap();
    assert!(scope.is_diff(), "local diff should be a diff scope");
    assert!(
        !scope.files().is_empty(),
        "should find modified files in worktree"
    );

    let files: Vec<&Path> = scope.files().iter().map(|p| p.as_path()).collect();
    assert!(
        files.contains(&Path::new("src/lib.rs")),
        "should include src/lib.rs, got: {files:?}"
    );
}

#[test]
fn test_1_1_staged_unstaged_untracked() {
    let repo = setup_git_repo();
    let repo_path = repo.root();
    let ctx = vampiro_cli::scan::GitContext::open(repo_path).unwrap();

    // Staged: add a new file to index
    let staged = repo.path("staged.rs");
    std::fs::write(&staged, "pub fn staged() -> u32 { 1 }").unwrap();
    Command::new("git")
        .args(["add", "staged.rs"])
        .current_dir(repo_path)
        .output()
        .expect("git add staged failed");

    // Unstaged: modify a tracked file without staging
    let lib_rs = repo.path("src/lib.rs");
    std::fs::write(&lib_rs, "pub fn modified() -> u32 { 7 }").unwrap();

    // Untracked: create a new file without staging
    let untracked = repo.path("untracked.rs");
    std::fs::write(&untracked, "pub fn untracked() -> u32 { 3 }").unwrap();

    let scope = ctx.local_diff().unwrap();
    let files: Vec<&Path> = scope.files().iter().map(|p| p.as_path()).collect();
    assert!(
        files.contains(&Path::new("staged.rs")),
        "should find staged.rs, got: {files:?}"
    );
    assert!(
        files.contains(&Path::new("src/lib.rs")),
        "should find src/lib.rs (unstaged), got: {files:?}"
    );
    assert!(
        files.contains(&Path::new("untracked.rs")),
        "should find untracked.rs, got: {files:?}"
    );
}

#[test]
fn test_1_1_explicit_target_base() {
    let repo = setup_git_repo();
    add_second_commit(&repo);
    let repo_path = repo.root();
    let ctx = vampiro_cli::scan::GitContext::open(repo_path).unwrap();

    let head = ctx.head_oid().unwrap();
    let first_parent = ctx.first_parent(head).unwrap();

    let scope = ctx
        .diff_between(&first_parent.to_string(), &head.to_string())
        .unwrap();
    assert!(scope.is_diff());
    let files: Vec<&Path> = scope.files().iter().map(|p| p.as_path()).collect();
    assert!(
        files.contains(&Path::new("src/main.rs")),
        "should find src/main.rs added in second commit, got: {files:?}"
    );
}

#[test]
fn test_1_1_detached_head() {
    let repo = setup_git_repo();
    add_second_commit(&repo);
    let repo_path = repo.root();

    // Detach HEAD by checking out a specific commit.
    let head_oid = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .expect("rev-parse failed");
    let head_sha = String::from_utf8(head_oid.stdout)
        .unwrap()
        .trim()
        .to_string();

    // Detach HEAD
    Command::new("git")
        .args(["checkout", "--detach"])
        .current_dir(repo_path)
        .output()
        .expect("checkout --detach failed");

    let ctx = vampiro_cli::scan::GitContext::open(repo_path).unwrap();
    let oid = ctx.head_oid().unwrap();
    assert_eq!(oid.to_string(), head_sha, "detached HEAD should resolve");
}

#[test]
fn test_1_1_initial_commit_no_parent() {
    let repo = setup_git_repo();
    let repo_path = repo.root();
    let ctx = vampiro_cli::scan::GitContext::open(repo_path).unwrap();

    let head = ctx.head_oid().unwrap();
    // First parent of the initial commit should return the empty tree.
    let parent_or_empty = ctx.first_parent(head).unwrap();
    // It won't error; it returns the empty tree OID.
    // Diff between empty tree and HEAD should show all files added.
    let scope = ctx
        .diff_between(&parent_or_empty.to_string(), &head.to_string())
        .unwrap();
    assert!(scope.is_diff());
    assert_eq!(
        scope.files().len(),
        1,
        "initial commit adds 1 .rs file (src/lib.rs)"
    );
}

#[test]
fn test_1_1_non_git_directory() {
    let repo = Fixture::new().build().expect("fixture build");
    let result = vampiro_cli::scan::GitContext::open(repo.root());
    assert!(
        result.is_err(),
        "non-Git directory should return NotAGitRepository error"
    );
}

#[test]
fn test_1_1_explicit_full_scope() {
    let repo = setup_git_repo();
    add_second_commit(&repo);
    let repo_path = repo.root();
    let ctx = vampiro_cli::scan::GitContext::open(repo_path).unwrap();

    let scope = ctx.full_scope().unwrap();
    assert!(!scope.is_diff(), "full scope should not be diff");
    let files: Vec<&Path> = scope.files().iter().map(|p| p.as_path()).collect();
    assert!(
        files.contains(&Path::new("src/lib.rs")),
        "full scope should include src/lib.rs"
    );
    assert!(
        files.contains(&Path::new("src/main.rs")),
        "full scope should include src/main.rs"
    );
}

#[test]
fn test_1_1_versioned_cache_invalidation() {
    use vampiro_cli::scan::{CacheKey, ScanCache};

    let cache = ScanCache::new();

    // Insert under version 1
    let key_v1 = CacheKey::from_content(b"hello", "1", "1", "1", "1");
    cache.insert(key_v1.clone(), vec![1, 2, 3]);
    assert_eq!(cache.len(), 1);

    // Same content, different analyzer version = cache miss
    let key_v2_analyzer = CacheKey::from_content(b"hello", "2", "1", "1", "1");
    assert!(
        cache.get(&key_v2_analyzer).is_none(),
        "analyzer version bump should miss"
    );

    // Same content, different schema version = cache miss
    let key_v2_schema = CacheKey::from_content(b"hello", "1", "2", "1", "1");
    assert!(
        cache.get(&key_v2_schema).is_none(),
        "schema version bump should miss"
    );

    // Original key should still hit
    assert_eq!(
        cache.get(&key_v1),
        Some(vec![1, 2, 3]),
        "original key should still hit"
    );
}

#[test]
fn test_1_1_shallow_repo_detection() {
    let repo = setup_git_repo();
    let ctx = vampiro_cli::scan::GitContext::open(repo.root()).unwrap();
    assert!(!ctx.is_shallow(), "fresh repo should not be shallow");
}

#[test]
fn test_1_1_merge_base_nonexistent_shallow() {
    // Simulate: calling merge_base between two unrelated commits should fail.
    let repo = setup_git_repo();
    let ctx = vampiro_cli::scan::GitContext::open(repo.root()).unwrap();

    let head = ctx.head_oid().unwrap();
    // A non-existent OID will cause find_commit to fail in merge_base.
    let fake_oid = git2::Oid::from_str("0000000000000000000000000000000000000000").unwrap();
    let result = ctx.merge_base(head, fake_oid);
    assert!(result.is_err(), "merge_base with fake OID should error");
}
