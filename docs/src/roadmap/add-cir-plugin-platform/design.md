> **Status:** Active OpenSpec proposal; not implemented or deployed. The source under `openspec/changes/` is authoritative.

## Context
CIR is the sole boundary between syntax plugins and language-neutral analysis.

## Goals / Non-Goals
- Goals: loss-aware recursive models, deterministic plugins, declarative extensibility, validated conformance.
- Non-goals: language parsing, source findings, scan orchestration, or law execution.

## Decisions
- Model built-in IDs (`plain`, `result`, `option`, `throws`, `async`, `stream`) and project-declared effect/functor IDs as recursive coproduct terms; reserve `unknown` for undeclared/unrecognized wrappers.
- Represent shapes structurally and permit `opaque`; source spans and stable identities accompany nodes/edges.
- Record exact discard spans and callee-to-caller argument provenance through configured `H` local-binding hops, terminating over-bound chains explicitly.
- Record ordinary wrapper removal as `resolution=unwrapped` with independently computed totality. Record panic/force removal as separate wrapper-removal evidence with `resolution=swallowed, totality=partial` unless every summand has an intentional branch.
- Load plugins only after versioned fixtures verify deterministic output, identity/composition preservation, and shared-diagram naturality.
- Require an explicit identity morphism for every declared object and reject missing identities before closure; closure never repairs the declaration. Construct finite closure from the validated identities and non-identity generators, then exhaustively validate composition tables, laws, wide subcategories, filtration nesting, and `sev(e)`.

## Risks / Trade-offs
- Finite closure can still grow; reject declarations exceeding configured resource limits as configuration errors, never as partial validation. Fixtures test the exhaustive algorithm but never substitute for validating each declaration.
- Plugin ABI stability is difficult in Rust; packaging/ABI and serialization/versioning are pre-implementation decisions covered by compatibility fixtures.

## Decision Gate
- A HITL decision ticket SHALL select and document plugin packaging/ABI,
  serialization/schema-version policy, compatibility window, migration policy,
  resource limits, approver, and immutable review reference before platform
  implementation.
