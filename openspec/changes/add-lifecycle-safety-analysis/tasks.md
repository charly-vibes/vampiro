> Implementation is blocked until the roadmap approval gate records approval of the EARS source and this proposal.

## 0. Snapshot and Lifecycle Idiom Decision Gate (HITL)
- [x] 0.1 Compare snapshot storage/schema/version/retention/baseline/migration alternatives and initial write/resource idiom sets against history, migration, unknown, and portability examples; record the choice in `docs/decisions/lifecycle-storage.md`.
- [x] 0.2 Record migration-reader policy, supported idioms, exclusions, approver, and immutable review reference before persistence or classifier implementation.

## 1. Facade History Tracer
- [x] 1.1 Add failing nearest-first-parent, explicit override, first snapshot, missing/non-ancestor baseline, deterministic multiple-snapshot, breaking shape, migration, and ambiguous-identity tests (REQ-T1, REQ-T4, REQ-T8).
- [x] 1.2 Implement versioned snapshot persistence, deterministic identity matching, aliases, and migration authorization under the approved storage contract.
- [x] 1.3 Run focused historical and core-result compatibility tests; publish snapshot schema/version, migration fixture path, and passing evidence.

## 2. Retry Idempotency Tracer
- [x] 2.1 Add failing versioned write-table conformance, unknown, unsafe-retry, idempotency-equation, and robustness/optionality cross-reference tests (REQ-T2, REQ-T5–T6, REQ-T9).
- [x] 2.2 Implement idempotency classification and retry findings against the published law idempotency-evidence contract without depending on prover adapters.
- [x] 2.3 Run focused table/property/contract tests; record table version, law evidence reference, and passing commands.

## 3. Resource Linearity Tracer
- [ ] 3.1 Add failing normal/early/error/panic fixtures for unique identity, one-to-one release, duplicate release leaving another obligation pending, transfer, mismatch, and exact `identity:unknown` diagnostics (REQ-T3, REQ-T7).
- [ ] 3.2 Implement identity-based exit-path matching, transfer, duplicate-release non-discharge, resource-leak findings, and unknown coverage evidence.
- [ ] 3.3 Run focused control-flow and Rust lifecycle-fact compatibility tests; record expected finding/diagnostic evidence and passing commands.

## 4. Lifecycle Acceptance
- [ ] 4.1 Run history, retry, resource, negative-table, contract, and relevant frontend E2E suites without requiring completed scan workflows.
- [ ] 4.2 Verify REQ-T1–T9 traceability and compatibility only with published CIR, core-result, law-evidence, and Rust lifecycle-fact contracts.
- [ ] 4.3 Run workspace formatting/Clippy and `openspec validate add-lifecycle-safety-analysis --strict`; record contract versions, commands, and evidence location.
