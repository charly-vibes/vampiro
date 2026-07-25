## Context
Property tests and external proofs are different evidence channels over declared theories.

## Goals / Non-Goals
- Goals: arbitrary theories, repeatable cluster testing, honest proof statuses, combined evidence.
- Non-goals: proving untagged code, bundling provers, or making proof required for default checks.

## Decisions
- Represent interfaces as operation signatures plus equations; semigroup, monoid, and functor are built-in templates, not limits.
- Frontends extract cluster membership, tags, and serializable values/generator references; project suites explicitly replace or augment built-ins and registered language runners return explicit unsupported/execution results.
- Lower all equations to a versioned backend-neutral obligation IR; deliver one end-to-end runner before staging Lean, Dafny, then TLA+ process adapters with `Proved`, `Disproved`, `Timeout`, or `ProverUnavailable`.
- Execute analyzed source only for an explicitly requested property/law path, never static `check`.
- Aggregate property/proof evidence by obligation into one finding.
- Expose a stable idempotency-evidence contract consumed by lifecycle analysis without requiring either whole change to serialize behind scan.

## Risks / Trade-offs
- Generated values may not cover a model meaningfully; expose generator configuration/evidence and never silently skip a member.
- Prover translations can be unsound; version adapters and golden-test generated obligations.

## Decision Gate
- A HITL decision ticket SHALL confirm the Rust property-testing crate, initial
  prover input/process formats, supported versions, and timeout/resource policy
  before runner or adapter implementation.
