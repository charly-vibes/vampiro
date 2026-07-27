> **Status:** Active OpenSpec proposal; not implemented or deployed. The source under `openspec/changes/` is authoritative.

> Implementation is blocked until the roadmap approval gate records approval of the EARS source and this proposal.

## 0. Plugin Boundary Decision Gate (HITL)
- [ ] 0.1 Compare packaging/ABI and serialization/schema-version alternatives with cross-version load, unknown-version rejection, and canonical-byte fixtures; record the choice in `docs/decisions/plugin-boundary.md`.
- [ ] 0.2 Record compatibility window, migration policy, resource limits, rejected alternatives, approver, and immutable review reference before platform implementation.

## 1. CIR and Recursive Effect Tracer
- [ ] 1.1 Add failing round-trip fixtures for nodes, shapes/opaque, recursive built-in/custom effects, source spans, stable identities, exact discard evidence, and direct/within-`H`/over-`H` provenance (REQ-1–3, REQ-21).
- [ ] 1.2 Implement the versioned CIR/schema, project effect definitions, ordinary/force unwrap evidence, totality, unknown sentinels, and bounded provenance required by the fixtures.
- [ ] 1.3 Run focused round-trip and canonical UTF-8 byte tests plus workspace format/Clippy; publish the schema/version and fixture path as the CIR consumer contract.

## 2. Category and Filtration Tracer
- [ ] 2.1 Add failing exhaustive tests for missing identities, closure/table laws, invalid wide subcategories, non-nesting, arbitrary depth, and `filtration_distance = sev(e)` independent of severity (REQ-C1–C3, REQ-C8–C9).
- [ ] 2.2 Implement finite declaration validation, configured resource-limit rejection, closure/composition tables, filtrations, and distance evidence without allowing fixtures to replace declaration validation.
- [ ] 2.3 Run focused property/model tests and publish the accepted declaration schema, resource-limit behavior, and passing canonical fixtures.

## 3. Plugin Load and Conformance Tracer
- [ ] 3.1 Add failing load fixtures for frontend/resolver traits, unknown idioms, independent effect/visibility tables, deterministic functoriality/naturality, reproducibility, version mismatch, and two-plugin conflicts (REQ-6, REQ-22, REQ-29, REQ-C10, REQ-V1–V2).
- [ ] 3.2 Implement plugin loading under the approved boundary, conformance gating, explicit unknown behavior, canonical result/load-manifest serialization, and rejection of both conflicting plugins.
- [ ] 3.3 Run failed-load, negative-fixture, byte-reproducibility, and plugin integration tests; publish the load-manifest version and consumer fixture path.

## 4. Platform Acceptance
- [ ] 4.1 Run declaration, model, load, reproducibility, and integration suites plus `cargo test --workspace`, formatting, and Clippy with warnings denied.
- [ ] 4.2 Verify exact requirement traceability and a consumer compatibility test against the CLI finding/configuration contracts.
- [ ] 4.3 Run `openspec validate add-cir-plugin-platform --strict` and record CIR schema, plugin boundary, fixture versions, commands, and passing evidence location.
