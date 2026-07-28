# Upgrade genesis to v0.2.0 — Design

## Overview

This change upgrades the genesis dependency from v0.1.0 to v0.2.0 and
adopts the new `genesis::config` and `genesis::guide` modules.

The existing adoption of `genesis::envelope`, `genesis::suggestions`,
`genesis::managed_block`, and `genesis::aix` (shipped in depend-on-genesis)
remains unchanged.

## Architecture

### Config Adoption (`genesis::config`)

- `Config` struct kept as-is; the `ConfigFile` trait replaces bespoke
  `load_config`/`load_config_with_xdg` functions.
- `vampiro_config_store()` registers `Config` under the `"vampiro"` tool
  name with `.vampiro/config.toml` marker.
- Missing config is treated as defaults (not an error); parse errors
  propagate through `ErrorSink`.

### Guide Adoption (`genesis::guide`)

- `main.rs` uses `Guide::builder("vampiro", ...)` to obtain a shared
  `CommandRegistry` and `ErrorSink`.
- The `ErrorSink` fulfills the feedback scratch contract: errors are
  persisted for `vampiro feedback bug --from-last-error`.

## Key Decisions

1. **Missing config is optional.** Unlike `dont` where config is required,
   vampiro has minimal config needs. Missing `.vampiro/config.toml` yields
   defaults.
2. **No `ConfigStore` managed block integration yet.** A future change can
   add a `doctor` command that uses `store.managed_block()`.
3. **Retained `ExitCode` enum.** The Guide's `run()` method returns i32,
   but vampiro's existing `ExitCode` provides richer semantics. `main()`
   still returns `ExitCode`; the Guide is used for registry/error-sink
   scaffolding only.