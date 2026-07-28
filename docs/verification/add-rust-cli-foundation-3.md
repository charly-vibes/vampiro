# Verification: Finding Envelope Tracer

## Evidence

**Date:** 2026-07-28
**Change:** `add-rust-cli-foundation`, task 3

### Commands executed

```bash
# Build
cargo build
# → Finished dev profile

# Tests (filtered)
cargo test --workspace rust_cli_foundation_3
# → 7 passed, 0 failed

# Full workspace
cargo test --workspace
# → 28 passed, 0 failed

# Formatting
cargo fmt --check
# → no diff

# Clippy (warnings denied)
cargo clippy -- -D warnings
# → Finished, no warnings
```

### Test results

| Test | Status |
|------|--------|
| Finding construction (rule, path, line range, severity, axis, fd) | ✅ |
| Custom filtration distance override | ✅ |
| Filtration distance set to None | ✅ |
| JSON serialization | ✅ |
| JSON deserialization | ✅ |
| sev() function values (Error→3, Warning→2, Note→1) | ✅ |
| Axis display strings | ✅ |

### Artifacts

- Finding envelope module: `crates/vampiro-cli/src/finding.rs`
- Contract artifact: `tests/contracts/findings/envelope-v1.json`
- Unit tests: `crates/vampiro-cli/src/finding.rs` (5 tests)
- Integration tests: `crates/vampiro-cli/tests/finding_tests.rs` (7 tests)

### Source requirements

- REQ-4 (exit code contract): unaffected — no new exit codes added
- REQ-5 (finding envelope): verified through finding construction and serialization tests
- REQ-12 (finding axis): verified through axis enum and display tests