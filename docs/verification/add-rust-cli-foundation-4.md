# Verification: Foundation Acceptance

## Evidence

**Date:** 2026-07-28
**Change:** `add-rust-cli-foundation`, task 4

### Commands executed

```bash
# Full workspace tests
cargo test --workspace
# → 32 passed, 0 failed

# Acceptance tests (successor compatibility)
cargo test --workspace rust_cli_foundation_4
# → 4 passed, 0 failed
#   - Contract cli-config-exit/v1 exists and is valid JSON
#   - Contract finding-envelope/v1 exists and is valid JSON
#   - Verification docs exist
#   - No analysis/gating behavior present

# Formatting
cargo fmt --check
# → no diff

# Clippy (warnings denied)
cargo clippy -- -D warnings
# → Finished, no warnings

# OpenSpec validation
openspec validate add-rust-cli-foundation --strict
# → Change 'add-rust-cli-foundation' is valid
```

### Deliverables

| Artifact | Location |
|----------|----------|
| Workspace and binary | `Cargo.toml`, `crates/vampiro-cli/` |
| CLI contract | `docs/decisions/cli-contract.md` |
| Config module | `crates/vampiro-cli/src/config.rs` |
| Exit code type | `crates/vampiro-cli/src/exit_code.rs` |
| Finding envelope | `crates/vampiro-cli/src/finding.rs` |
| CLI snapshots | `crates/vampiro-cli/tests/cmd/` |
| Config contract | `tests/contracts/cli/config-exit-v1.json` |
| Finding contract | `tests/contracts/findings/envelope-v1.json` |

### Scope verification

No analysis, proof, CI-generation, or gating behavior is claimed or implemented.