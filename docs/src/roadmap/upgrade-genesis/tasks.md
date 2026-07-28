> **Status:** Active OpenSpec proposal; not implemented or deployed. The source under `openspec/changes/` is authoritative.

## 1. Bump dependency
- [x] Already on `genesis-vibes = "0.2"` in `crates/vampiro-cli/Cargo.toml`.

## 2. Adopt genesis::config
- [x] Rewrote `config.rs`: kept `Config` struct, added `impl ConfigFile for Config`, removed bespoke `ConfigError`/`load_config`/`load_config_with_xdg`.
- [x] Added `vampiro_config_store()` — registers `Config` with `ConfigRegistry` under `"vampiro"` with `.vampiro/config.toml` marker.
- [x] Wired `ConfigStore` in `main.rs` — missing config is treated as defaults, parse errors go through `ErrorSink`.
- [x] `cargo test` passes: 258 tests, all green.

## 3. Adopt genesis::guide
- [x] `main.rs` uses `Guide::builder("vampiro", ...)` for CommandRegistry + ErrorSink.
- [x] Error handling uses `guide.error_sink().handle()` for config and runtime errors (ErrorSink contract with scratch persistence).
- [x] Removed dead `CommandRegistry::new()`, `SuggestionEngine::new()` manual setup.
- [x] `cargo test` passes: all CLI snapshots and acceptance tests green.

## 4. Clean up
- [x] `cargo test` — all 258 tests pass.
- [x] `cargo clippy --all-targets -- -D warnings` — clean.
- [x] `cargo fmt --check` — clean.
- [x] Updated test files: `config_tests.rs`, `cir_consumer_tests.rs`, `genesis_compatibility.rs`.
- [x] Updated `lib.rs` exports.