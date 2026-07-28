## Context

Vampiro has no Rust implementation yet, while Genesis owns reusable suite-level
CLI infrastructure. Integrating the pinned dependency with the first Rust
workspace avoids local implementations that would later require migration.
Genesis tag `v0.1.0` and its public API are external preconditions; the global
roadmap approval gate remains the human authorization boundary.

## Goals / Non-Goals

- Reuse Genesis for envelopes, unknown-command suggestions, managed blocks,
  and AIX artifact rendering from the first implementation of each concern.
- Verify the exact tag and resolved commit before consumer work starts.
- Keep CIR, findings, analysis, policy, and all other Vampiro domain behavior
  inside Vampiro.
- Do not extract Vampiro front ends or CIR types into Genesis.
- Do not implement any proposal task before approval.

## Decisions

### Pin the Git tag and record the resolved commit

`Cargo.toml` will use the Genesis repository at tag `v0.1.0`. The API tracer
must compile all four required modules and record Cargo's resolved immutable
commit. Consumer tracers remain blocked if the tag is absent or incompatible;
Vampiro will not temporarily copy or vendor the missing behavior.

Alternatives rejected:

- A branch or unpinned revision is not reproducible.
- A path dependency does not work for independent clones or CI.
- Local temporary implementations create two migrations and violate the
  from-day-one integration goal.

### Adapt at the serialization and presentation boundaries

Vampiro constructs its own normalized findings and command vocabulary, then
passes those values to Genesis at the JSON, error-footer, managed-file, and AIX
rendering boundaries. Genesis does not receive composition-analysis
responsibility or define Vampiro's domain model.

### Prove each integration with a focused tracer

Each consumer starts with a failing conformance or golden fixture, adds the
smallest integration, and records API/version and command evidence in a
dedicated verification document. After the API tracer, the four consumers may
proceed independently except where the CLI-foundation graph names a milestone.

## Risks / Trade-offs

- **The tag is unavailable or lacks an API.** Keep affected tickets blocked;
  do not substitute an unpinned source or duplicate implementation.
- **Genesis API changes after the tag.** The exact tag and lockfile resolution
  isolate Vampiro; upgrades require an explicit dependency change and rerun of
  all five tracers.
- **Domain logic leaks into Genesis adapters.** Tests construct normalized
  Vampiro values before calling Genesis, and review rejects analysis logic in
  the integration layer.
- **Generated files drift.** Golden and idempotency checks compare committed
  artifacts and managed blocks with fresh Genesis output.

## Migration Plan

1. Land the approved Rust workspace skeleton.
2. Verify and pin Genesis `v0.1.0` through the API tracer.
3. Integrate the envelope and suggestions into CLI-foundation milestones.
4. Integrate managed blocks and AIX rendering independently.
5. Run all focused and workspace gates before closing the proposal.

Rollback removes a consumer integration and its dependency use together. It
must not introduce a fallback local renderer or engine; the affected feature
remains unavailable until the pinned integration is restored.
