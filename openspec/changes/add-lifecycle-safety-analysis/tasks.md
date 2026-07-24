## 1. Lifecycle Tests
- [ ] 1.1 Select/document snapshot storage, schema/version, retention, baseline override, and migration syntax before persistence; test nearest first-parent baseline, override, first snapshot, missing/non-ancestor baseline, deterministic multiple snapshots, breaking shape, migration, and ambiguous identity (REQ-T1, REQ-T4, REQ-T8).
- [ ] 1.2 Add failing write-table conformance, unknown, unsafe-retry, and law cross-reference tests (REQ-T2, REQ-T5–T6, REQ-T9).
- [ ] 1.3 Add failing normal/early/error/panic tests for unique acquisition identity, one-to-one release, duplicate release leaving another obligation pending, transfer, and exact `identity:unknown` diagnostics (REQ-T3, REQ-T7).

## 2. Analysis
- [ ] 2.1 Implement versioned snapshot persistence, deterministic matching, aliases, and migration authorization.
- [ ] 2.2 Implement versioned idempotency tables and retry findings; integrate idempotency equations with law verification.
- [ ] 2.3 Implement identity-based one-to-one exit-path/resource matching, transfer, duplicate-release non-discharge semantics, REQ-T7 resource-leak findings, and exact `identity:unknown` coverage evidence.

## 3. Verification
- [ ] 3.1 Run historical, negative-table, property/law, control-flow, scan/report, and relevant frontend E2E tests.
- [ ] 3.2 Run rustfmt and Clippy.
- [ ] 3.3 Verify REQ-T1–T9 traceability and only the CIR/platform, core finding, law idempotency-evidence, and Rust lifecycle-extraction contracts.
