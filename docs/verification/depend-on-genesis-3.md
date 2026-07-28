# Verification: Shared Suggestions Tracer

## Evidence

**Date:** 2026-07-28
**Change:** `depend-on-genesis`, task 3

### What was done

- Registered Vampiro's command list (`check`, `prove`, `help`) with `genesis::suggestions::CommandRegistry`
- Initialized `genesis::suggestions::SuggestionEngine` in startup
- Verified Genesis engine produces correct suggestions for common typos

### Commands executed

```bash
# Suggestion tests
cargo test suggest
# → 4 passed, 0 failed

# Full workspace
cargo test --workspace
# → 42 passed, 0 failed

# Formatting
cargo fmt --check
# → no diff

# Clippy (warnings denied)
cargo clippy -- -D warnings
# → Finished, no warnings
```

### Test results

| Test | What it proves | Status |
|------|----------------|--------|
| Typo suggestion | `chek` → suggests `check` | ✅ |
| Unrelated token | `xyzzy` → exits 2, no false positive | ✅ |
| Deterministic | Same typo → same output | ✅ |
| No local engine | Only Genesis's engine used | ✅ |

### Artifacts

- Suggestion engine setup: `crates/vampiro-cli/src/main.rs` (CommandRegistry + SuggestionEngine)
- Suggestion tests: `crates/vampiro-cli/tests/suggestions_tests.rs`
- Genesis suggestions API version: v0.1.0 (tagged)