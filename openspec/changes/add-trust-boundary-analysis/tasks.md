> Implementation is blocked until the roadmap approval gate records approval of the EARS source and this proposal.

## 0. Trust and Evidence Contract Decision Gate (HITL)
- [x] 0.1 Compare value/arm representations, trust-domain/source idioms, transfer/join and over-`H` rules, declaration/idiom conflicts, smart-constructor and validation-identity declarations, default finding severities, boundary-class syntax, and evidence-schema alternatives against ambiguous source, mixed input, stale revision, unsupported version, constructor-hash mismatch, and empty/duplicate/incomplete-class examples; record the choice in `docs/decisions/trust-boundary-contract.md`.
- [x] 0.2 Record the initial Rust idiom scope, project configuration and validation-observation schemas, finding cardinality/severity, evidence producer/version/freshness and closed-reason contracts, rejected alternatives, approver, and immutable review reference before implementation.

## 1. Trust-Provenance Contract Tracer
- [x] 1.1 Add failing serialized-schema, propagation, conflict, and Rust E2E fixtures for value/arm-level trust-boundary sources, internal constants, explicit declarations, recognized smart-constructor success/non-success arms, mixed contributors, raw/refined shapes, `untrusted`/`trusted`/`unknown`, and direct/within-`H`/over-`H` argument-provenance separation (REQ-B1, REQ-B2, REQ-B6).
- [x] 1.2 Implement the approved versioned CIR/configuration extension, Rust source/refinement idioms, project declarations, value-level transfer/join rules, conflict/over-`H` unknown behavior, and exact `trust-provenance:unknown` diagnostic required by the fixtures.
- [x] 1.3 Run focused schema round-trip, plugin conformance, canonical-byte, and Rust E2E tests; publish schema/idiom versions and fixture paths as the consumer contract.

## 2. Boundary-Leak Tracer
- [x] 2.1 Add failing Rust E2E fixtures for direct and propagated raw flow into an interior node, flow into a recognized smart constructor, totally handled refinement outcomes, and unknown provenance (REQ-B3, REQ-C4).
- [x] 2.2 Implement language-neutral boundary-leak analysis so only proven `untrusted` flow into a non-boundary, non-constructor interior node emits exactly one default-`HIGH` robustness `boundary-leak` finding per violating edge, with REQ-24 deduplication and source, edge, and target evidence.
- [x] 2.3 Run focused positive/negative tests plus normalized-result compatibility; verify constructor flow and `unknown` produce no false boundary-leak finding and remain visible in diagnostics.

## 3. Validation-Duplication Tracer
- [ ] 3.1 Add failing CIR round-trip, Rust extraction/conformance, declaration/idiom, and E2E fixtures for validation observations, equivalent repeated validation, the recognized constructor itself, merely similar syntax, and unrelated constraints (REQ-B4).
- [ ] 3.2 Implement the approved validation-identity configuration/idioms, Rust validation-observation extraction, CIR fact, and language-neutral analysis, emitting exactly one default-`LOW` modularity `validation-duplication` finding per duplicate-check location with REQ-24 deduplication and identity, constructor, refined-shape, origin, and source-span evidence.
- [ ] 3.3 Run focused Rust E2E and finding-schema tests; verify syntactic similarity without equivalence evidence emits no finding.

## 4. Refinement-Confirmation Tracer
- [ ] 4.1 Add failing configuration, import, normalized-result, and consumer fixtures for non-empty unique classes with current complete passing evidence and absent, malformed, unsupported-version, stale-revision, constructor-identity/hash-mismatched, empty/duplicate/incomplete/unknown classes, or failing evidence (REQ-B5).
- [ ] 4.2 Implement the approved boundary-class/evidence-source configuration, versioned importer, and deterministic correlation against analyzed revision, constructor identity/hash, and the complete declared class set; emit `refinement_confirmation.status=confirmed` only for the positive case and `status=unknown` with the closed primary reason otherwise, permitting downstream `unreachable` only from `confirmed`.
- [ ] 4.3 Run focused import, canonical-byte, malformed-input, and reachability tests; publish the evidence schema/version and fixture path without coupling Vampiro to one companion implementation.

## 5. Trust-Boundary Acceptance
- [ ] 5.1 Run all four tracer suites, Rust E2E positives/negatives, normalized finding/diagnostic compatibility, workspace tests, formatting, and Clippy with warnings denied.
- [ ] 5.2 Verify exact REQ-B1–REQ-B6 traceability, no collision between trust and argument provenance, and no trusted/unreachable default from unknown or missing evidence.
- [ ] 5.3 Run `openspec validate add-trust-boundary-analysis --strict` and record contract versions, fixture paths, commands, and passing evidence location.
