# Verification: Executable Workspace Tracer

## Evidence

**Date:** 2026-07-27
**Change:** `add-rust-cli-foundation`, task 1

### Commands executed

```bash
# Build
cargo build
# → Finished dev profile

# Tests
cargo test --workspace
# → 1 passed (cli_snapshots)
# → 0 failed

# Formatting
cargo fmt --check
# → no diff

# Clippy (warnings denied)
cargo clippy -- -D warnings
# → Finished, no warnings
```

### Snapshot tests

| Test | Expected | Actual | Status |
|------|----------|--------|--------|
| `vampiro --help` | Shows usage, commands, options | Matches | ✅ |
| `vampiro --version` | `vampiro 0.1.0` | Matches | ✅ |
| `vampiro check --help` | "Reserved for analysis commands" | Matches | ✅ |
| `vampiro prove --help` | "Reserved for proof commands" | Matches | ✅ |

### Artifacts

- CLI snapshots: `crates/vampiro-cli/tests/cmd/`
- Workspace: `Cargo.toml` (workspace root), `crates/vampiro-cli/`
- CLI contract: `docs/decisions/cli-contract.md`