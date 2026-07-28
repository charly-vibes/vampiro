# Change: Depend on genesis

## Why

vampiro is spec-stage with no `src/` yet — build on `genesis` from day one.
vampiro's `add-rust-cli-foundation` change establishes the CLI skeleton; this
change composes with it by sourcing cross-cutting CLI/AIX/self-healing infra
from genesis (tool-craft playbook §8, Appendix A.4).

## What Changes

- Add `genesis` git dependency (pinned by tag `v0.1.0`) to `Cargo.toml`.
- Source the JSON envelope from `genesis::envelope` (composition findings
  under `data`).
- Source self-healing errors from `genesis::suggestions`.
- Source the managed-block injector from `genesis::managed_block` so vampiro
  carries WAI/OPENSPEC/DONT blocks and is detectable by `wai status`
  (`wai-bdqw.9`).
- Generate `llms.txt`/`llm.txt` through `genesis::aix`.
- Keep all vampiro domain logic (CIR graph, seam analysis, law/proof,
  lifecycle safety, trust-boundary analysis, front ends). The genesis
  boundary rule protects this.

## Impact

- Affected specs: `cli-foundation` (MODIFIED — envelope; ADDED — suggestions,
  managed blocks, and AIX artifacts sourced from genesis). Composes with, does not replace,
  `add-rust-cli-foundation`.
- Blocked by: genesis tagging `v0.1.0`.
- Coordinates with vampiro's EARS spec (vampiro-ears-spec.md) — no REQ
  changes; this is CLI infrastructure.

## Dependencies and Order

Implementation is approval-gated and begins only after the Rust workspace
tracer exists and the pinned Genesis tag exposes compatible `envelope`,
`suggestions`, `managed_block`, and `aix` modules. The dependency/API tracer
unlocks four independent consumer tracers; their acceptance tests prove Vampiro
retains domain ownership while reusing only cross-cutting infrastructure.
