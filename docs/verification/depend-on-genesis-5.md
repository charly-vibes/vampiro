# Verification: AIX Artifact Tracer

## Evidence

**Date:** 2026-07-28
**Change:** `depend-on-genesis`, task 5

### What was done

- Created `vampiro_cli::aix` module with `generate_llms_txt()`, `generate_llm_txt()`, and `write_aix_artifacts()`
- All generation routes through `genesis::aix::agents_block()` — no second local renderer
- Regenerated `llms.txt` and `llm.txt` from authoritative project metadata
- Added byte-for-byte freshness check (`aix_committed_artifacts_are_current`)

### Commands executed

```bash
# AIX tests
cargo test aix
# → 6 passed, 1 ignored (regeneration helper)

# Full workspace
cargo test --workspace
# → 52 passed, 1 ignored (regeneration helper)

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
| llms.txt generated through genesis | Uses `genesis::aix::agents_block()` | ✅ |
| llm.txt generated through genesis | Uses `genesis::aix::agents_block()` | ✅ |
| Deterministic generation | Same inputs → same outputs | ✅ |
| No second local renderer | Only Genesis used for rendering | ✅ |
| Committed artifacts are current | Byte-for-byte match with generated | ✅ |

### Artifacts

- AIX module: `crates/vampiro-cli/src/aix.rs`
- Generated `llms.txt` (regenerated from metadata)
- Generated `llm.txt` (regenerated from metadata)
- Genesis AIX API version: v0.1.0 (tagged; agents_block is functional)