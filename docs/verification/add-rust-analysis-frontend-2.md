# Verification: Section 2 — Visibility and Facade Tracer

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.3.3
**Spec:** `openspec/changes/add-rust-analysis-frontend/tasks.md#2`

## Summary

Delivered Rust visibility extraction and facade metadata. Independently versioned visibility idiom table (v0.1.0). All 48 tests pass.

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 2.1 | ✓ | Added fixtures for module ancestry, every `pub` form, `pub use`, crate-root facades, and unsupported constructs. |
| 2.2 | ✓ | Implemented independently versioned visibility idiom table v0.1.0 and facade metadata. Emits explicit unknowns rather than guesses. |
| 2.3 | ✓ | 28 visibility/facade tests pass. Table version and fixture evidence published. |

## Implementation

**Visibility module:** `crates/vampiro-rust-frontend/src/visibility.rs`

| Type | Description |
|------|-------------|
| `Visibility` enum | `Public`, `Crate`, `Super`, `Restricted(String)`, `Private` — with `TABLE_VERSION = "0.1.0"` |
| `FacadeEntry` | A re-exported item: name, original path, wildcard flag, visibility, span, doc_hidden |
| `FacadeDecl` | All re-exports at a module level: version, module_path, entries |

**Visibility extraction** (in `extract.rs`):
- `extract_function()` records `Visibility::from(&func.vis)` per node
- `extract_use_item()` collects `pub use` items as `FacadeEntry` entries
- `visit_item_mod()` tracks module hierarchy for nested re-exports

## Fixtures

Located at `tests/fixtures/add-rust-analysis-frontend/2/`:

| Fixture | Content |
|---------|---------|
| `visibility-all-forms.json` | Public, crate, and private function visibility entries |
| `facade-re-exports.json` | `pub use` and `pub use as` re-export entries |

## Test Suite

### Visibility tests (9 new)
| Test | What it verifies |
|------|-----------------|
| `extract_public_function_visibility` | `pub fn` → `Visibility::Public` |
| `extract_private_function_visibility` | `fn` (no pub) → `Visibility::Private` |
| `extract_crate_visibility` | `pub(crate) fn` → `Visibility::Crate` |
| `extract_super_visibility` | `pub(super) fn` → `Visibility::Super` |
| `extract_restricted_visibility` | `pub(in foo::bar) fn` → `Visibility::Restricted` |
| `extract_mixed_public_and_private` | Both pub and private in same file |
| `visibility_from_syn_*` (5 tests) | `syn::Visibility` → `Visibility` conversion |

### Facade tests (10 new)
| Test | What it verifies |
|------|-----------------|
| `extract_simple_pub_use` | `pub use helper;` → name, visibility |
| `extract_private_use` | `use helper;` → private visibility |
| `extract_wildcard_use` | `pub use module::*;` → wildcard flag |
| `extract_renamed_use` | `pub use old as new;` → renamed entry |
| `extract_use_group` | `pub use {foo, bar};` → multiple entries |
| `extract_use_path` | `pub use foo::bar::baz;` → final name |
| `extract_module_use` | `mod inner { pub use helper; }` → module scoping |
| `extract_multiple_uses` | Multiple `pub use` lines |
| `extract_doc_hidden_use` | `#[doc(hidden)] pub use` → doc_hidden flag |
| `extract_crate_use` | `pub(crate) use` → crate visibility |

### Visibility type tests (9 existing)
| Test | What it verifies |
|------|-----------------|
| `visibility_is_public`, `is_at_least_crate` | Boolean predicates |
| `visibility_display` | Display formatting |
| `facade_decl_*` | FacadeDecl construction |
| `facade_entry_serialization` | JSON serialization |
| `visibility_table_version` | `TABLE_VERSION = "0.1.0"` |

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo test --workspace` | 146 tests pass (48 Rust frontend, 67 CIR, 14 conformance, 9 consumer, 4+4 fixture) |
| `cargo fmt --check` | Clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clean |
| `openspec validate add-rust-analysis-frontend --strict` | Valid |

## Published Contracts

- **Visibility idiom table:** v0.1.0 — `vampiro_rust_frontend::visibility::Visibility`
- **Facade metadata schema:** v0.1.0 — `vampiro_rust_frontend::visibility::FacadeDecl`
- **Visibility extraction:** Integrated into `RustFrontend::extract()` via `ExtractionResult::visibility`
- **Fixture path:** `tests/fixtures/add-rust-analysis-frontend/2/`