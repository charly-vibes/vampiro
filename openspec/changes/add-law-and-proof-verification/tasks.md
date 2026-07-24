## 1. Verification Tests
- [ ] 1.1 Add failing every-member tests for replacement/augmentation suites, registered runners, extracted runner inputs, and explicit unsupported execution (REQ-10, REQ-18, REQ-C6).
- [ ] 1.2 Add adapter contract tests for all four statuses, timeouts, malformed/unavailable tools, and no property-as-proof substitution (REQ-12, REQ-16–17).
- [ ] 1.3 Add combined-evidence finding tests (REQ-26).

## 2. Verification Engine
- [ ] 2.1 Implement versioned backend-neutral obligation IR plus theory, cluster, runner-input, and evidence contracts.
- [ ] 2.2 Implement project suite replacement/augmentation and one registered language runner E2E; prove static `check` executes no source.
- [ ] 2.3 After the runner, implement optional versioned Lean/Dafny/TLA+ adapters and `prove` integration.
- [ ] 2.4 Aggregate property and proof evidence into one finding per failed obligation.

## 3. Verification
- [ ] 3.1 Run runner/unsupported, no-static-execution, obligation-IR, all-four-status, golden translation, timeout, and CLI/core tests.
- [ ] 3.2 Run rustfmt and Clippy.
- [ ] 3.3 Verify requirement traceability, CLI/CIR/Rust-runner contract compatibility, lifecycle idempotency-evidence compatibility, and that default `check` has no prover dependency or scan dependency.
