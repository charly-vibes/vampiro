# Verification: Section 2 — Normalized Result and Rendering Tracer

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.5.3

## Summary

Delivered the normalized `ScanResult` type with three renderers (human, JSON via genesis envelope, SARIF 2.1.0) and stable deduplication IDs (REQ-24). Unanalyzed files are explicitly reported (REQ-15).

## Implementation

| Module | Contents |
|--------|----------|
| `output.rs` | `ScanResult`, `FlatFinding`, `FlatDiagnostic`, `ScanResultMetadata`, `ScopeKind`; `render_human`, `render_json`, `render_sarif`; `hash_string`, `ScanResult::stable_id_for_finding` |

## Passing commands

```
$ cargo test -p vampiro --test output_tests
test result: ok. 9 passed; 0 failed

$ cargo test --workspace
(298+ passed, all crates, 0 failed)

$ cargo fmt --all --check
(clean)

$ cargo clippy --workspace --all-targets -- -D warnings
(no warnings)
```

## Contract

| Artifact | Version |
|----------|---------|
| `output.rs::ScanResult` | v1 (published) |
| Genesis envelope | `genesis-vibes = "0.2"` |
| SARIF output | 2.1.0 |

## Traceability

| REQ | Test |
|-----|------|
| REQ-15 (unanalyzed) | `unanalyzed_files_appear_in_all_formats` |
| REQ-19 (three renderers) | `render_json_produces_valid_output`, `render_human_produces_output`, `render_sarif_produces_valid_output`, `renderers_are_semantically_equivalent` |
| REQ-24 (stable dedup) | `stable_dedup_id_is_deterministic`, `stable_dedup_id_differs_for_different_findings`, `finding_has_stable_id_in_output` |
| REQ-C2 (filtration distance) | `render_human_produces_output` (filtration_distance field in format) |