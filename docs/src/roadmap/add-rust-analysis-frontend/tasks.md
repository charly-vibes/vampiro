> **Status:** Active OpenSpec proposal; not implemented or deployed. The source under `openspec/changes/` is authoritative.

> Implementation is blocked until the roadmap approval gate records approval of the EARS source and this proposal.

## 0. Rust Parser and Idiom Decision Gate (HITL)
- [ ] 0.1 Evaluate tree-sitter-rust, `syn`, and rust-analyzer-assisted options against representative macros, modules, provenance, unsupported syntax, and the 50-seam performance profile; record the choice in `docs/decisions/rust-frontend.md`.
- [ ] 0.2 Record supported Rust edition/version range, macro-expansion boundary, initial effect/write/acquire/release idioms, exclusions, approver, and immutable review reference.

## 1. Base Rust CIR Tracer
- [ ] 1.1 Add failing Rust fixtures for callable nodes, structural/opaque shapes, recursive effects, unknowns, exact source spans, and direct/within-bound/over-bound argument provenance.
- [ ] 1.2 Implement syntax-to-CIR extraction under the approved parser boundary without executing source.
- [ ] 1.3 Run deterministic CIR conformance and negative fixtures; publish the frontend/schema version, fixture path, and consumer test proving the CIR milestone for core.

## 2. Visibility and Facade Tracer
- [ ] 2.1 Add failing fixtures for module ancestry, every `pub` form, `pub use`, crate-root facades, hidden/internal conventions, macros beyond coverage, and unsupported constructs (REQ-V1–V2).
- [ ] 2.2 Implement independently versioned Rust visibility/effect idiom tables and facade metadata, emitting explicit unknowns rather than guesses.
- [ ] 2.3 Run focused visibility/facade conformance and core-consumer compatibility tests; publish table versions and fixture evidence.

## 3. Law Runner-Input Tracer
- [ ] 3.1 Add failing extraction fixtures for implementation clusters, proof/law tags, serializable values, generator references, and construct-specific unsupported evidence.
- [ ] 3.2 Implement runner-input extraction only; keep runner execution owned by law verification.
- [ ] 3.3 Run deterministic round-trip and law-consumer compatibility tests; publish the runner-input schema/version and fixture path.

## 4. Lifecycle Fact Tracer
- [ ] 4.1 Add failing extraction fixtures for write/retry facts, resource identity, acquisition/release/transfer, normal/early/error/panic exit paths, aliases, and explicit unknowns.
- [ ] 4.2 Implement lifecycle extraction hooks without lifecycle classification or findings.
- [ ] 4.3 Run deterministic lifecycle-consumer compatibility tests; publish the lifecycle-fact schema/version and fixture path.

## 5. Rust Frontend Acceptance
- [ ] 5.1 Run all Rust CIR, visibility, runner-input, lifecycle, deterministic, and negative suites plus workspace formatting and Clippy.
- [ ] 5.2 Verify each published milestone independently satisfies its named platform consumer contract and generic IDs remain conformance references.
- [ ] 5.3 Run `openspec validate add-rust-analysis-frontend --strict` and record parser decision, schema/table versions, commands, and passing evidence location.
