> Implementation is blocked until the roadmap approval gate records approval of the EARS source and this proposal.

## 0. Plugin Boundary Decision Gate (HITL)
- [x] 0.1 Compare packaging/ABI and serialization/schema-version alternatives with cross-version load, unknown-version rejection, and canonical-byte fixtures; record the choice in `docs/decisions/plugin-boundary.md`.
- [x] 0.2 Record compatibility window, migration policy, resource limits, rejected alternatives, approver, and immutable review reference before platform implementation.

> **Decision (2026-07-28):** YAGNI — workspace-crate ABI only. No serialization, no dynamic loading, no plugin boundary until ≥2 frontends exist. See `docs/decisions/plugin-boundary.md`.
>
> **Scope impact:**
> - Sections 3 and 4 proceed with **in-process trait dispatch** via the `Frontend` trait — no dynamic loading, no serialization, no subprocess ABI.
> - Section 3 is **partially deferred**: the `Frontend` trait contract exists, but multi-plugin scenarios (version mismatch, two-plugin conflicts, load-manifest serialization) are scoped to single-frontend with fixture-based conformance. The multi-plugin aspects of 3.1–3.2 are marked STUB below; they will be revived when a second frontend enters active development.
> - Section 4 is adjusted: platform acceptance verifies the `Frontend` trait contract, depth-limit validation, and single-plugin fixture conformance — not dynamic plugin lifecycle.

## 1. CIR and Recursive Effect Tracer
- [x] 1.1 Add failing round-trip fixtures for nodes, shapes/opaque, recursive built-in/custom effects, source spans, stable identities, exact discard evidence, and direct/within-`H`/over-`H` provenance (REQ-1–3, REQ-21).
- [x] 1.2 Implement the versioned CIR/schema, project effect definitions, ordinary/force unwrap evidence, totality, unknown sentinels, and bounded provenance required by the fixtures.
- [x] 1.3 Run focused round-trip and canonical UTF-8 byte tests plus workspace format/Clippy; publish the schema/version and fixture path as the CIR consumer contract.

> **Verification (2026-07-28):** All 26 CIR tests pass (22 unit + 4 fixture round-trips). Workspace format and Clippy clean. CIR schema v0.1.0 published as `CirGraph` in `vampiro-cir` crate. Fixtures at `tests/fixtures/add-cir-plugin-platform/1/`. Full details in `docs/verification/add-cir-plugin-platform-1.md`.

## 2. Category and Filtration Tracer
*(This section may proceed in parallel with section 1.)*
- [x] 2.1 Add failing exhaustive tests for missing identities, closure/table laws, invalid wide subcategories, non-nesting, arbitrary depth, and `filtration_distance = filtration_level(e)` independent of severity (REQ-C1–C3, REQ-C8–C9).
- [x] 2.2 Implement finite declaration validation, configured resource-limit rejection, closure/composition tables, filtrations, and distance evidence without allowing fixtures to replace declaration validation.
- [x] 2.3 Run focused property/model tests and publish the accepted declaration schema, resource-limit behavior, and passing canonical fixtures.

> **Verification (2026-07-28):** All 8 category validation tests pass. Finite closure up to 4096 morphisms, filtration up to 16 levels. `filtration_level()` returns least containing level index. Fixtures at `tests/fixtures/add-cir-plugin-platform/2/`. Full details in `docs/verification/add-cir-plugin-platform-2.md`.

## 3. Plugin Load and Conformance Tracer
*(Depends on section 0–1 for CIR types and section 2.1 for identity/closure validation. Scoped to single-frontend trait dispatch per YAGNI decision above.)*
- [x] 3.1 Add passing fixtures for the `Frontend` trait contract: language identifier, graph extraction, depth-limit rejection. *STUB: multi-plugin scenarios (version mismatch, two-plugin conflicts) deferred until ≥2 frontends exist.*
- [x] 3.2 Implement the `Frontend` trait and extraction error types. *STUB: load-manifest serialization deferred until dynamic loading is needed.*
- [x] 3.3 Run frontend trait tests, negative fixtures (invalid source, depth exceeded), and fixture-based conformance tests.

> **Verification (2026-07-28):** All 14 frontend conformance tests pass. 3 fixtures at `tests/fixtures/add-cir-plugin-platform/3/`. Full details in `docs/verification/add-cir-plugin-platform-3.md`.

## 4. Platform Acceptance
*(Depends on sections 0–3. Scoped to workspace-crate integration per YAGNI decision.)*
- [ ] 4.1 Run declaration, model, frontend trait, and integration suites plus `cargo test --workspace`, formatting, and Clippy with warnings denied.
- [ ] 4.2 Verify exact requirement traceability and a consumer compatibility test that imports `vampiro-cir`, creates a `CirGraph`, calls `validate()`, and integrates with the CLI finding/configuration contracts.
- [ ] 4.3 Run `openspec validate add-cir-plugin-platform --strict` and record CIR schema, plugin boundary, fixture versions, commands, and passing evidence location.