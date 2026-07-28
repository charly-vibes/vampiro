# Verification: Section 3 — Effect Handling Tracer

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.4.4
**Spec:** `openspec/changes/add-core-seam-analysis/specs/seam-analysis/spec.md`
**Canonicalization decision:** `docs/decisions/shape-canonicalization.md` (vampiro-0vb.4.1, approved 2026-07-28)

## Summary

Delivered the effect-handling tracer (REQ-9, REQ-25, REQ-C4) as the third slice
of the `vampiro-seam-analysis` crate. Introduces the `SwallowedEffect` evidence
variant on the `robustness` axis (default severity `MEDIUM` per REQ-4 table).
Handles recursive coproduct resolution, independent totality classification,
and memoized bounded ancestor search for `throws` effects.

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 3.1 | ✓ | 17 focused tests covering direct result/option/throws discard, nested/recursive effect, ancestor-handled/swallowed-throws, ordinary total unwrap (not swallowed), force partial unwrap (swallowed), force total unwrap (not swallowed), custom effect exclusion, and axis-only robustness validation. |
| 3.2 | ✓ | Recursive coproduct resolution (`collect_discard_channels`), independent totality (`classify_edge`), memoized bounded ancestor handling search (`search_ancestors` with depth limit 32). |
| 3.3 | ✓ | 4 E2E tests (hand-constructed CIR graphs + full pipeline) + 17 unit tests; all findings use only the robustness axis and preserve exact discard evidence via `discard_spans`. |

## Implementation

### Effect-handling tracer — `crates/vampiro-seam-analysis/src/effects.rs`

| Feature | Implementation |
|---------|---------------|
| `EffectHandlingAnalyzer::analyze` | Iterates every CIR edge; classifies each as swallowed or not based on resolution + unwrap evidence; collects discard channels from callee's effect (recursive coproduct resolution). |
| `classify_edge` | `Swallowed` resolution → swallowed (totality from `unwrap_evidence` or `partial`). `Unwrapped` + `Force` + `Partial` → swallowed per REQ-C4. `Unwrapped` + `Ordinary` + `Total` → properly handled. |
| `collect_discard_channels` | Resolves recursive effect channels one layer at a time (REQ-C4). Plain/Option/Result/Throws → single channel. `Recursive(inner)` → recurses and includes base channel. |
| `search_ancestors` | Memoized bounded (depth 32) DFS up the CIR call graph from the swallow site. Stops at process-boundary nodes. Returns `true` if any ancestor unwraps the throws effect. |

### Finding contract extension — `crates/vampiro-seam-analysis/src/finding.rs`

| Feature | Implementation |
|---------|---------------|
| `Evidence::SwallowedEffect` | New variant with `discarded_channel` (EffectChannel), `discard_lines` (Vec<DiscardSpan>), `totality` (String), `ancestor_handled` (Option<bool>). |
| `Finding::swallowed_effect` | Builder function with default severity `MEDIUM`, axis `Robustness`, classification `swallowed-effect`, rule `REQ-9`. |

## Fixtures

Located at `tests/fixtures/add-core-seam-analysis/3/`:

| Fixture | Rust source | Purpose |
|---------|-------------|---------|
| `swallowed_effect.rs` | `parse_raw() -> Option<f64>`, `lookup_price() -> Result<f64, String>`, `force_unwrap()`, `total()` with `let _ = parse_raw(raw)` and `let _ = lookup_price(id)` | Negative fixture for swallowed Option and Result effects; force-unwrap via `val.unwrap()`. |

**Note:** The Rust frontend does not yet classify edges as `Swallowed` (discard
detection is a separate enhancement). The E2E tests construct the CIR graph
programmatically to validate the analyzer + evidence + output format. A
frontend-based E2E test can be added once the frontend supports discard
detection.

## Expected finding fields

A swallowed-effect finding produced by the tracer carries (REQ-4, REQ-9):

| Field | Value |
|-------|-------|
| `rule` | `REQ-9` |
| `axis` | `robustness` |
| `severity` | `medium` (default; REQ-4 table) |
| `line-range-start` / `line-range-end` | edge span |
| `evidence.discarded-channel` | the discarded effect channel (e.g. `result`, `option`, `throws`) |
| `evidence.discard-lines` | exact source lines of the discard (e.g. `[{file, start_line, end_line}]`) |
| `evidence.totality` | `total` / `partial` / `unknown` |
| `evidence.ancestor-handled` | present for `throws` channel: `true` if ancestor handles, `false` otherwise |
| `filtration-distance` | absent |
| `classification` | `swallowed-effect` |

## Passing command output

```
$ cargo test -p vampiro-seam-analysis
test result: ok. 39 passed; 0 failed; 0 ignored (lib)
test result: ok. 4 passed; 0 failed; 0 ignored (effects_e2e)
test result: ok. 1 passed; 0 failed; 0 ignored (composition_e2e)
test result: ok. 1 passed; 0 failed; 0 ignored (modularity_e2e)

$ cargo test --workspace
total passed: 285  (all crates, 0 failed)

$ cargo fmt --all --check
(clean)

$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile in 0.28s  (no warnings)

$ openspec validate add-core-seam-analysis --strict
Change 'add-core-seam-analysis' is valid
```

## Contract versions

| Contract | Version | Location |
|----------|---------|----------|
| CIR schema | `0.1.0` | `vampiro-cir` crate; `CirGraph.version` |
| Effect channel model | `0.1.0` | `vampiro-cir::effect` (EffectChannel, EffectResolution, UnwrapEvidence, Totality) |
| Normalized finding contract | `0.1.0` (in-progress; formally published at `0vb.4.6`) | `vampiro-seam-analysis::finding` (now includes `SwallowedEffect` evidence variant) |

## Known limitation / refinement

The effect-handling tracer operates on edges the frontend classifies as
`Swallowed` or as `Unwrapped` with `Force`+`Partial` evidence. The Rust
frontend does not currently produce `Swallowed` edges for `let _ = expr;`
patterns — that is tracked as a separate frontend enhancement. The analyzer
logic is correct; the frontend gap means real Rust source with discard
patterns does not trigger findings today. When the frontend adds discard
detection, this tracer will work end-to-end without modification.

## Owned requirement traceability

| Requirement | Test(s) |
|-------------|---------|
| REQ-9 (swallowed effect, robustness axis) | `swallowed_result_raises_robustness_finding`, `swallowed_option_raises_robustness_finding`, `swallowed_throws_raises_robustness_finding`, `effects_e2e_swallowed_result` |
| REQ-25 (ancestor handling for throws) | `swallowed_throws_finds_no_ancestor`, `swallowed_throws_finds_ancestor_handler`, `effects_e2e_swallowed_throws_no_ancestor`, `effects_e2e_swallowed_throws_with_ancestor` |
| REQ-C4 (recursive coproduct resolution, independent totality) | `recursive_effect_resolves_all_layers`, `ordinary_total_unwrap_does_not_raise_finding`, `force_partial_unwrap_raises_finding`, `force_total_unwrap_does_not_raise_finding`, `effects_e2e_force_partial_unwrap` |
| REQ-4 (closed axis set, default severities) | `all_effect_findings_use_robustness_axis` |
| Non-target effect channels | `swallowed_plain_does_not_raise_finding`, `swallowed_async_does_not_raise_finding`, `swallowed_stream_does_not_raise_finding`, `swallowed_custom_effect_does_not_raise_finding` |