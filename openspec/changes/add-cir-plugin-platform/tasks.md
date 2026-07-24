## 1. Model Tests
- [ ] 1.1 Select and document plugin packaging/ABI and serialization/schema-version policy; add failing cross-version compatibility and canonical UTF-8 result/load-manifest byte tests.
- [ ] 1.2 Add failing round-trip/fixture tests for recursive built-in/custom effects, unmatched wrapper/unwrap, partial force/panic unwrap, total all-summand handling, and direct/within-`H`/over-`H` provenance (REQ-1–3, REQ-21).
- [ ] 1.3 Add failing exhaustive finite-declaration tests for identities, closure/table laws, non-nesting, valid `filtration_distance = sev(e)`, and arbitrary filtration depth (REQ-C1–C3, REQ-C8–C9).

## 2. Platform
- [ ] 2.1 Implement CIR, project effect definitions, finite closure/composition-table validation, filtrations, and `filtration_distance = sev(e)` evidence distinct from configured severity.
- [ ] 2.2 Implement frontend/resolver traits and explicit unknown idiom behavior.
- [ ] 2.3 Build independently versioned visibility/effect fixtures plus deterministic functoriality/naturality checks (REQ-6, REQ-29, REQ-C10, REQ-V1–V2).
- [ ] 2.4 Reject both conflicting plugins with actionable diagnostics (REQ-22).

## 3. Verification
- [ ] 3.1 Run declaration-validation, model, failed-load, negative-fixture, reproducibility, and plugin integration tests; prove fixtures do not replace declaration validation.
- [ ] 3.2 Run rustfmt and Clippy.
- [ ] 3.3 Verify exact requirement traceability and integration with `add-rust-cli-foundation`.
