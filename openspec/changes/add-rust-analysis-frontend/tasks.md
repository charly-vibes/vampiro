> Implementation is blocked until the roadmap approval gate records approval of the EARS source and this proposal.

## 0. Rust Parser and Idiom Decision Gate (HITL)
- [x] 0.1 Evaluate tree-sitter-rust, `syn`, and rust-analyzer-assisted options against representative macros, modules, provenance, unsupported syntax, and the 50-seam performance profile; record the choice in `docs/decisions/rust-frontend.md`.
- [x] 0.2 Record supported Rust edition/version range, macro-expansion boundary, initial effect/write/acquire/release idioms, exclusions, approver, and immutable review reference.

> **Decision (2026-07-28):** `syn` — typed AST, lightweight, no CST→CIR mapping layer. Rust 2021+ minimum. No macro expansion. See `docs/decisions/rust-frontend.md`.

## 1. Base Rust CIR Tracer
- [x] 1.1 Add failing Rust fixtures for callable nodes, structural/opaque shapes, recursive effects, unknowns, exact source spans, and direct/within-bound/over-bound argument provenance.
- [x] 1.2 Implement syntax-to-CIR extraction under the approved parser boundary without executing source.
- [x] 1.3 Run deterministic CIR conformance and negative fixtures; publish the frontend/schema version, fixture path, and consumer test proving the CIR milestone for core.

> **Verification (2026-07-28):** All 20 Rust frontend tests pass. 3 fixtures at `tests/fixtures/add-rust-analysis-frontend/1/`. `RustFrontend` implemented in `crates/vampiro-rust-frontend/`. Full details in `docs/verification/add-rust-analysis-frontend-1.md`.

## 2. Visibility and Facade Tracer
- [x] 2.1 Add failing fixtures for module ancestry, every `pub` form, `pub use`, crate-root facades, hidden/internal conventions, macros beyond coverage, and unsupported constructs (REQ-V1–V2).
- [x] 2.2 Implement independently versioned Rust visibility/effect idiom tables (v0.1.0) and facade metadata, emitting explicit unknowns rather than guesses.
- [x] 2.3 Run focused visibility/facade conformance and core-consumer compatibility tests; publish table versions and fixture evidence.

> **Verification (2026-07-28):** 28 visibility/facade tests pass. Visibility idiom table v0.1.0. Fixtures at `tests/fixtures/add-rust-analysis-frontend/2/`. Full details in `docs/verification/add-rust-analysis-frontend-2.md`.

## 3. Law Runner-Input Tracer
- [x] 3.1 Add failing extraction fixtures for implementation clusters, proof/law tags, serializable values, generator references, and construct-specific unsupported evidence.
- [x] 3.2 Implement runner-input extraction only; keep runner execution owned by law verification.
- [x] 3.3 Run deterministic round-trip and law-consumer compatibility tests; publish the runner-input schema/version and fixture path.

> **Verification (2026-07-28):** 14 law runner-input tests pass. Runner-input schema v0.1.0. Fixtures at `tests/fixtures/add-rust-analysis-frontend/3/`. Full details in `docs/verification/add-rust-analysis-frontend-3.md`.

## 4. Lifecycle Fact Tracer
- [x] 4.1 Add failing extraction fixtures for write/retry facts, resource identity, acquisition/release/transfer, normal/early/error/panic exit paths, aliases, and explicit unknowns.
- [x] 4.2 Implement lifecycle extraction hooks without lifecycle classification or findings.
- [x] 4.3 Run deterministic lifecycle-consumer compatibility tests; publish the lifecycle-fact schema/version and fixture path.

> **Verification (2026-07-28):** 13 lifecycle extraction tests pass. Lifecycle-fact schema v0.1.0. Fixtures at `tests/fixtures/add-rust-analysis-frontend/4/`. Full details in `docs/verification/add-rust-analysis-frontend-4.md`.

## 5. Rust Frontend Acceptance
- [ ] 5.1 Run all Rust CIR, visibility, runner-input, lifecycle, deterministic, and negative suites plus workspace formatting and Clippy.
- [ ] 5.2 Verify each published milestone independently satisfies its named platform consumer contract and generic IDs remain conformance references.
- [ ] 5.3 Run `openspec validate add-rust-analysis-frontend --strict` and record parser decision, schema/table versions, commands, and passing evidence location.
