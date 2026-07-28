# Verification: Git scope and incremental caching (Section 1)

**Date:** 2026-07-28
**Tickets:** vampiro-0vb.5.2
**Contracts:**
- ScanScope — diff and full scope resolution (vampiro-cli/src/scan/mod.rs)
- ScanCache — versioned in-memory cache with content+version keys
- GitContext — Git repository wrapper using git2

## Test results

```bash
$ cargo test --test scan_gating_reporting_1
# 10 tests, 10 passed

$ cargo test --lib -p vampiro -- scan::tests
# 15 tests, 15 passed

$ cargo test --workspace
# 258+ tests, all pass

$ cargo fmt --check
# clean

$ cargo clippy --workspace --all-targets -- -D warnings
# clean
```

## Evidence

### Test 1.1 scenarios covered

| Scenario | Test | Status |
|----------|------|--------|
| synthetic worktree | `test_1_1_synthetic_worktree_diff` | ✅ |
| staged + unstaged + untracked | `test_1_1_staged_unstaged_untracked` | ✅ |
| explicit target/base | `test_1_1_explicit_target_base` | ✅ |
| detached HEAD | `test_1_1_detached_head` | ✅ |
| initial commit (no parent) | `test_1_1_initial_commit_no_parent` | ✅ |
| non-Git context | `test_1_1_non_git_directory` | ✅ |
| full scope | `test_1_1_explicit_full_scope` | ✅ |
| versioned cache invalidation | `test_1_1_versioned_cache_invalidation` | ✅ |
| merge base unavailable | `test_1_1_merge_base_nonexistent_shallow` | ✅ |
| shallow repo detection | `test_1_1_shallow_repo_detection` | ✅ |

### Cache contract

- CacheKey is computed from content SHA-256 + analyzer/schema/plugin/config versions
- Same content + same versions = deterministic same key
- Different content or any version bump = different key (cache miss)
- In-memory ScanCache with get/insert/clear operations
- Insert with same content hash evicts prior entry (version change)

### Scope contract

- `GitContext::open()` — opens repo or returns NotAGitRepository error
- `GitContext::local_diff()` — HEAD vs worktree (staged + unstaged + untracked .rs files)
- `GitContext::diff_between(base, target)` — explicit base/target diff
- `GitContext::full_scope()` — all .rs files from HEAD tree + untracked
- Initial commit returns empty tree as base (diff shows all files as new)
- `is_shallow()` checks for `.git/shallow` marker file
- No silent full fallback — merge base error propagates as ScopeError