# Change: Add trust-boundary analysis

## Why
External data can cross into internal code without first becoming a refined
shape, while repeated validation can drift from the smart constructor that is
supposed to own the invariant. Vampiro needs one conservative, language-neutral
analysis path that distinguishes trust provenance from existing CIR argument
provenance and never treats missing validation evidence as proof of safety.

## What Changes
- Extend CIR and frontend conformance with trust-boundary sources, propagated
  trust provenance, refined shapes, smart-constructor declarations/idioms, and
  an explicit unknown diagnostic.
- Add robustness `boundary-leak` and modularity `validation-duplication`
  findings with declaration-backed evidence.
- Import versioned boundary-coverage evidence and allow `unreachable` only when
  every declared boundary class has current, correlated passing evidence.

## Impact
- Affected capability: `trust-boundary-analysis`
- Source requirements: REQ-B1–REQ-B6; reuses REQ-C4 totality and the existing
  finding/diagnostic contracts.
- Affected contracts: CIR schema, frontend conformance fixtures, project
  configuration, normalized findings/diagnostics, and optional companion-tool
  evidence import.

## Dependencies and Order
Depends on stable CIR/plugin and shared finding contracts plus the Rust
frontend's base extraction milestone. The trust-provenance tracer establishes
the consumer contract; boundary-leak, duplicate-validation, and coverage-
confirmation tracers can then proceed independently before final acceptance.
