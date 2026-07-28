# Verification: Shared Envelope Tracer

## Evidence

**Date:** 2026-07-28
**Change:** `depend-on-genesis`, task 2

### What was done

- Added `--json` flag to `vampiro check` subcommand
- Routed output through `genesis::envelope::Envelope` with `EnvelopeKind::Check`
- Findings (empty for now) are placed under `data` as an array
- Vampiro constructs its own findings; Genesis only provides the envelope

### Commands executed

```bash
# Envelope test
cargo test rust_cli_foundation_2_envelope_json_top_level_keys
# → 1 passed, 0 failed

# Full workspace
cargo test --workspace
# → 38 passed, 0 failed

# Formatting
cargo fmt --check
# → no diff

# Clippy (warnings denied)
cargo clippy -- -D warnings
# → Finished, no warnings
```

### Envelope structure

```json
{
  "ok": true,
  "envelope_version": "0.1",
  "cli_version": "0.1.0",
  "envelope_kind": "check",
  "data": [],
  "warnings": [],
  "meta": {
    "duration_ms": 0,
    "tx": null,
    "request_id": null,
    "author": null
  }
}
```

### Artifacts

- CLI module: `crates/vampiro-cli/src/cli/mod.rs` (CheckArgs with --json flag)
- Envelope test: `crates/vampiro-cli/tests/envelope_tests.rs`
- Genesis envelope version: `0.1`