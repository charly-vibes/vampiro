> **Status:** Active OpenSpec proposal; not implemented or deployed. The source under `openspec/changes/` is authoritative.

> Implementation is blocked until the roadmap approval gate records approval of the EARS source and this proposal. Section 1 may proceed before the property/prover decision; Sections 2–3 require the approved decision.

## 0. Property and Prover Boundary Decision Gate (HITL)
- [ ] 0.1 Compare Rust property-testing crates and initial Lean/Dafny/TLA+ input/process formats against deterministic generation, timeout, malformed/unavailable, and golden-translation examples; record the choice in `docs/decisions/law-backends.md`.
- [ ] 0.2 Record supported versions, process/security boundary, timeout/resource policy, rejected alternatives, approver, and immutable review reference.

## 1. Obligation and Idempotency-Evidence Contract Tracer
- [ ] 1.1 Add failing round-trip/version tests for theories, clusters, runner inputs, backend-neutral obligations, property/proof evidence, all four statuses, and lifecycle idempotency cross-references (REQ-12, REQ-16, REQ-C6, REQ-T6).
- [ ] 1.2 Implement the versioned obligation/evidence contracts without runner or prover execution.
- [ ] 1.3 Run focused canonical contract and lifecycle-consumer compatibility tests; publish schema/version, fixture path, and evidence contract milestone.

## 2. Law Runner Tracer
- [ ] 2.1 Add failing every-member tests for replacement/augmentation suites, Rust runner inputs, explicit construct-level unsupported results, generated values, and proof that static `check` executes no source (REQ-10, REQ-18, REQ-C6).
- [ ] 2.2 Implement suite selection and one registered Rust property runner under the approved property boundary; source execution remains explicit law-only behavior.
- [ ] 2.3 Run runner/unsupported/no-static-execution tests and Rust frontend compatibility fixtures; record generator evidence and passing commands.

## 3. Prover Adapter Tracer
- [ ] 3.1 Add failing golden/process tests for `Proved`, `Disproved`, `Timeout`, `ProverUnavailable`, malformed tools, unavailable tools, and prohibition on property-as-proof substitution (REQ-12, REQ-16–17).
- [ ] 3.2 Implement optional versioned Lean, Dafny, and TLA+ adapters plus `prove` dispatch under the approved process boundary.
- [ ] 3.3 Run golden translation, timeout, unavailable-tool, and CLI tests; record adapter/input versions and passing evidence.

## 4. Combined Evidence and Law Acceptance
- [ ] 4.1 Add failing correlation tests requiring one optionality finding per failed obligation with combined property/proof evidence (REQ-26).
- [ ] 4.2 Implement deterministic aggregation without introducing scan dependency or a default-check prover dependency.
- [ ] 4.3 Run all contract/runner/adapter/aggregation suites, workspace formatting/Clippy, and `openspec validate add-law-and-proof-verification --strict`; record requirement traceability and evidence location.
