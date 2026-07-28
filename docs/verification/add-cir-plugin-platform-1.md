# Verification: CIR and Recursive Effect Tracer

> OpenSpec change: `add-cir-plugin-platform`, task section 1
> Ticket: `vampiro-0vb.2.2`
> Date: 2026-07-28

## Test results

### Unit tests (50 tests in `vampiro-cir`)

| Test | Status |
|------|--------|
| `cir::tests::cir_graph_round_trip` | ✅ |
| `cir::tests::cir_graph_with_effects` | ✅ |
| `cir::tests::cir_graph_empty` | ✅ |
| `cir::tests::cir_graph_custom_effect` | ✅ |
| `cir::tests::cir_graph_validate_missing_node` | ✅ |
| `cir::tests::cir_graph_validate_effect_depth` | ✅ |
| `cir::tests::cir_graph_validate_shape_depth` | ✅ |
| `cir::tests::cir_graph_from_json_valid` | ✅ |
| `cir::tests::cir_graph_from_json_invalid_missing_node` | ✅ |
| `effect::tests::effect_channel_plain` | ✅ |
| `effect::tests::effect_channel_recursive` | ✅ |
| `effect::tests::effect_channel_custom` | ✅ |
| `effect::tests::effect_channel_unknown` | ✅ |
| `effect::tests::effect_resolution_propagated` | ✅ |
| `effect::tests::effect_resolution_unwrapped` | ✅ |
| `effect::tests::unwrap_evidence_serialization` | ✅ |
| `effect::tests::effect_depth_plain` | ✅ |
| `effect::tests::effect_depth_recursive` | ✅ |
| `effect::tests::effect_depth_deeply_nested` | ✅ |
| `effect::tests::effect_depth_within_limit` | ✅ |
| `effect::tests::effect_depth_beyond_limit` | ✅ |
| `provenance::tests::provenance_direct` | ✅ |
| `provenance::tests::provenance_within_h` | ✅ |
| `provenance::tests::provenance_over_bound` | ✅ |
| `provenance::tests::provenance_over_bound_no_traced_hops` | ✅ |
| `provenance::tests::source_span_serialization` | ✅ |
| `provenance::tests::discard_span_serialization` | ✅ |
| `provenance::tests::stable_id_construction` | ✅ |
| `provenance::tests::traced_hop_serialization` | ✅ |
| `shape::tests::shape_scalar` | ✅ |
| `shape::tests::shape_opaque` | ✅ |
| `shape::tests::shape_record` | ✅ |
| `shape::tests::shape_function` | ✅ |
| `shape::tests::shape_parameterized` | ✅ |
| `shape::tests::shape_depth_scalar` | ✅ |
| `shape::tests::shape_depth_opaque` | ✅ |
| `shape::tests::shape_depth_record` | ✅ |
| `shape::tests::shape_depth_nested_record` | ✅ |
| `shape::tests::shape_depth_parameterized_nested` | ✅ |
| `shape::tests::shape_depth_within_limit` | ✅ |
| `shape::tests::shape_depth_beyond_limit` | ✅ |
| `error::tests::cir_error_missing_node_display` | ✅ |
| `error::tests::cir_error_target_node_display` | ✅ |
| `error::tests::cir_error_depth_display` | ✅ |
| `error::tests::cir_error_error_trait` | ✅ |
| `error::tests::cir_error_from_serde_json` | ✅ |
| `error::tests::cir_error_from_str` | ✅ |
| `frontend::tests::test_frontend_implements_trait` | ✅ |
| `frontend::tests::null_frontend_returns_empty_graph` | ✅ |
| `frontend::tests::frontend_language_is_static` | ✅ |

### Fixture round-trip tests (4 tests in `fixture_round_trips.rs`)

| Test | Status |
|------|--------|
| `fixture_simple_call_round_trip` | ✅ |
| `fixture_recursive_effect_round_trip` | ✅ |
| `fixture_custom_effect_round_trip` | ✅ |
| `fixture_canonical_utf8_byte_reproducibility` | ✅ |

### Workspace quality gates

| Gate | Status |
|------|--------|
| `cargo test --workspace` | ✅ (93 tests, 0 failures) |
| `cargo fmt --check` | ✅ |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ |

## Fixture paths

Fixture files live at `tests/fixtures/add-cir-plugin-platform/1/`:

- `simple-call.json` — basic node→edge graph with plain effect
- `recursive-effect.json` — recursive effect channel with unwrap evidence and discard spans
- `custom-effect.json` — project-declared custom effect and resolution

## CIR schema version

- **Version:** `0.1.0`
- **Crate:** `vampiro-cir` (`crates/vampiro-cir/`)
- **Contract:** `CirGraph` with nodes, edges, shapes, effects, provenance, stable identities, source spans, discard spans, unwrap evidence, `Frontend` trait, `CirError` type, and depth-limit validation

## Commands

```bash
cargo test --workspace -p vampiro-cir
cargo test --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

All pass. See `crates/vampiro-cir/src/` for the implementation and `tests/fixtures/add-cir-plugin-platform/1/` for the fixture files.