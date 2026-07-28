# Verification: Managed-Block Tracer

## Evidence

**Date:** 2026-07-28
**Change:** `depend-on-genesis`, task 4

### What was done

- Created `vampiro_cli::managed` module with `vampiro_registry()` and `vampiro_injector()`
- Registered WAI, OPENSPEC, DONT blocks with `genesis::managed_block::BlockRegistry`
- Added DONT block to `AGENTS.md` for `wai status` detection
- Source injector mechanics from `genesis::managed_block` (no local implementation)

### Commands executed

```bash
# Managed block tests
cargo test managed_block
# → 7 passed, 0 failed

# Full workspace
cargo test --workspace
# → 49 passed, 0 failed

# wai status integration check
wai status
# → Projects: vampiro [implement] — detected

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
| Insert WAI block | Creates file with markers | ✅ |
| Insert OPENSPEC block | Creates file with markers | ✅ |
| Insert DONT block | Creates file with markers | ✅ |
| Update existing block | Replaces content, no duplicate markers | ✅ |
| Idempotent replay | Same content → same output | ✅ |
| Preserve surrounding content | User content survives update | ✅ |
| All three blocks registered | WAI, OPENSPEC, DONT present | ✅ |

### Artifacts

- Managed block module: `crates/vampiro-cli/src/managed.rs`
- Managed block tests: `crates/vampiro-cli/tests/managed_block_tests.rs`
- Project AGENTS.md: now carries DONT block
- Genesis managed_block API version: v0.1.0 (tagged)