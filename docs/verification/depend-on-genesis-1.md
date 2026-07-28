# Verification: Genesis Dependency and API Tracer

## Evidence

**Date:** 2026-07-28
**Change:** `depend-on-genesis`, task 1

### Pinned dependency

- **Repository:** `ssh://git@cv/charly-vibes/genesis.git`
- **Tag:** `v0.1.0`
- **Resolved commit:** `613dc00041839e6a4dff1bbf2cb1f6b55e93ab5d`
- **Cargo config:** `.cargo/config.toml` with `net.git-fetch-with-cli = true`

### API modules verified

| Module | Status |
|--------|--------|
| `genesis::envelope` | ✅ `Envelope<T>`, `EnvelopeKind`, `success()`, `CLI_VERSION`, `ENVELOPE_VERSION` |
| `genesis::suggestions` | ✅ `SuggestionEngine`, `CommandRegistry`, `suggest_typo()` |
| `genesis::managed_block` | ✅ `BlockInjector`, `BlockRegistry`, `BlockDef` |
| `genesis::aix` | ✅ `agents_block()` |

### Commands executed

```bash
# Genesis compatibility tests
cargo test genesis_api
# → 5 passed, 0 failed

# Full workspace
cargo test --workspace
# → 37 passed, 0 failed

# Formatting
cargo fmt --check
# → no diff

# Clippy (warnings denied)
cargo clippy -- -D warnings
# → Finished, no warnings
```

### Artifacts

- Dependency: `crates/vampiro-cli/Cargo.toml` (genesis entry)
- Cargo config: `.cargo/config.toml`
- Compatibility tests: `crates/vampiro-cli/tests/genesis_compatibility.rs`
- Lockfile: `Cargo.lock` (resolves genesis to `613dc000`)