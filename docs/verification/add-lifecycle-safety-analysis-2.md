# Retry Idempotency Tracer — Implementation Verification (Task 2)

> Verification evidence for `add-lifecycle-safety-analysis`, task section 2.

**Write-idiom table version:** `0.1.0`
**Write-idiom schema version:** `WRITE_IDIOM_SCHEMA_VERSION = "0.1.0"`
**Retry idempotency schema version:** `RETRY_IDEMPOTENCY_SCHEMA_VERSION = "0.1.0"`
**Crate:** `vampiro-lifecycle-analysis` at `crates/vampiro-lifecycle-analysis/`

---

## Module structure

```
crates/vampiro-lifecycle-analysis/src/
  write_idiom_table.rs       — WriteIdiomTable, IdempotencyClass, builtin v0.1.0 table
  retry_idempotency.rs       — RetryIdempotencyAnalyzer, RetryIdempotencyFinding, RetryCoverageDiagnostic
```

---

## Test commands

```bash
# Run all lifecycle tests (includes retry idempotency)
cargo test --workspace -p vampiro-lifecycle-analysis

# Run all workspace tests (no regressions)
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Test evidence

### write_idiom_table module — 13 tests

| Test | Scenario | Status |
|------|----------|--------|
| `table_schema_version_is_0_1_0` | Schema version constant | ✅ |
| `builtin_table_has_entries` | Built-in v0.1.0 table populated | ✅ |
| `classify_insert_as_non_idempotent` | REQ-T2: insert → non-idempotent | ✅ |
| `classify_upsert_as_idempotent` | REQ-T2: upsert → idempotent | ✅ |
| `classify_update_as_idempotent` | REQ-T2: update → idempotent | ✅ |
| `classify_delete_as_idempotent` | REQ-T2: delete → idempotent | ✅ |
| `classify_put_as_idempotent` | REQ-T2: PUT → idempotent | ✅ |
| `classify_patch_as_non_idempotent` | REQ-T2: PATCH → non-idempotent | ✅ |
| `classify_fs_write_as_non_idempotent` | REQ-T2: fs::write → non-idempotent | ✅ |
| `classify_vec_push_as_non_idempotent` | REQ-T2: push → non-idempotent | ✅ |
| `classify_hashmap_insert_as_idempotent` | REQ-T2: HashMap::insert → idempotent | ✅ |
| `classify_unknown_returns_unknown` | REQ-T9: unknown idiom → coverage diagnostic | ✅ |
| `empty_table_classifies_all_as_unknown` | Empty table → all unknown | ✅ |

### retry_idempotency module — 12 tests

| Test | Scenario | Status |
|------|----------|--------|
| `non_idempotent_write_produces_unsafe_retry_finding` | REQ-T5: insert retry → unsafe-retry | ✅ |
| `idempotent_write_produces_no_finding` | REQ-T5: set retry → no finding | ✅ |
| `upsert_is_idempotent_no_finding` | REQ-T5: upsert retry → safe | ✅ |
| `patch_is_non_idempotent` | REQ-T5: PATCH retry → unsafe-retry | ✅ |
| `unknown_write_method_produces_coverage_diagnostic` | REQ-T9: unknown → coverage diagnostic | ✅ |
| `mixed_idempotent_and_non_idempotent` | Multiple facts: correct classification of each | ✅ |
| `empty_facts_produce_no_results` | No facts → empty results | ✅ |
| `finding_contains_sufficient_info_for_law_cross_reference` | REQ-T6: file:line:function forms cross-ref key | ✅ |
| `analyzer_with_custom_table` | Custom idiom table works | ✅ |

---

## Coverage

| Requirement | Coverage | Evidence |
|-------------|----------|----------|
| REQ-T2 | Write-shape idiom table with idempotency classification | `write_idiom_table.rs` |
| REQ-T5 | Unsafe retry finding for non-idempotent writes | `retry_idempotency.rs` |
| REQ-T6 | Cross-reference fields for law evidence correlation | Function/file/line in each finding |
| REQ-T9 | Unknown idiom → coverage diagnostic (not a finding) | `RetryCoverageDiagnostic` |

## Law evidence reference

The law idempotency-evidence contract is in `vampiro-law` (crate version 0.1.0).
The retry findings carry function + source_file + line enabling cross-reference
with `CombinedEvidence.lifecycle_ref`. No prover adapters are required per
task 2.2 spec.