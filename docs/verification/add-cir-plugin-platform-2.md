# Verification: Category and Filtration Tracer

> OpenSpec change: `add-cir-plugin-platform`, task section 2
> Ticket: `vampiro-0vb.2.3`
> Date: 2026-07-28

## Test results

### Category tests (8 tests in `category::tests`)

| Test | Status |
|------|--------|
| `missing_identity_is_rejected` | ✅ |
| `non_closed_composition_is_rejected` | ✅ |
| `invalid_wide_subcategory_is_rejected` | ✅ |
| `non_nesting_is_rejected` | ✅ |
| `arbitrary_filtration_depth_accepted` | ✅ (supports L0–L9, beyond default L0–L4) |
| `filtration_level_computed_correctly` | ✅ |
| `valid_category_with_identity_and_composition` | ✅ |
| `resource_limit_exceeded_is_rejected` | ✅ |

### Workspace quality gates

| Gate | Status |
|------|--------|
| `cargo test --workspace` | ✅ (93 tests, 0 failures) |
| `cargo fmt --check` | ✅ |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ |

## Fixture paths

Fixture files at `tests/fixtures/add-cir-plugin-platform/2/`:

- `valid-category.json` — a valid category declaration with objects A, B, C, identities, and composition
- `nested-filtration.json` — a valid nested filtration with 3 levels

## Implemented API

Module: `vampiro-cir::category`

| Function | Description |
|----------|-------------|
| `validate_category()` | Validates category declaration → `ValidatedCategory` |
| `validate_filtration()` | Validates filtration against a closure |
| `filtration_level()` | Computes the least containing filtration level for a morphism |

### Types

| Type | Description |
|------|-------------|
| `CategoryDecl` | Category declaration with objects, morphisms, composition |
| `FiltrationDecl` | Filtration declaration with nested levels |
| `FiltrationLevel` | A single filtration level (wide subcategory) |
| `MorphismDecl` | A morphism declaration (identity or generator) |
| `MorphismId` | Stable morphism identifier |
| `CompositionRule` | A composition rule `first ∘ second = result` |
| `ValidatedCategory` | Validated declaration with morphisms, composition table, and ID set |
| `ValidationError` | Error type with 8 variants |

### Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `MAX_CLOSURE_SIZE` | 4096 | Maximum morphisms in a finite closure |
| `MAX_FILTRATION_LEVELS` | 16 | Maximum filtration levels |

## Commands

```bash
cargo test --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

All pass. See `crates/vampiro-cir/src/category.rs` for the implementation.