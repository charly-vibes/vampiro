## 1. Shared Acceptance
- [ ] 1.1 Add per-language matrices for full node/edge/shape/effect extraction, direct/within-bound/over-bound provenance, visibility, unknowns, and deterministic output (conformance references REQ-1–3, REQ-V1–V2).
- [ ] 1.2 Add per-language runner-input, successful obligation-IR execution, construct-specific unsupported, lifecycle fact/unknown, and L4 snapshot fixtures (REQ-10, REQ-17, REQ-T1–T3 conformance references).

## 2. Python Milestone
- [ ] 2.1 Add failing Python fixtures, confirm parser, and implement full CIR, provenance, a registered Python law runner, lifecycle facts, and `__init__.py` facade extraction.
- [ ] 2.2 Pass CIR, visibility, law-runner, lifecycle snapshot, advisory, and core facade E2E tests for Python.

## 3. Clojure Milestone
- [ ] 3.1 Add failing Clojure fixtures, confirm parser, and implement full CIR/provenance, a registered Clojure law runner, lifecycle facts, and private-var metadata.
- [ ] 3.2 Pass conformance, lifecycle snapshot, runner, and REQ-V6 E2E tests.

## 4. Julia Milestone
- [ ] 4.1 Add failing Julia fixtures, confirm parser, and implement full CIR/provenance, a registered Julia law runner, lifecycle facts, and ownership metadata.
- [ ] 4.2 Pass conformance, lifecycle snapshot, runner, and REQ-V5 E2E tests.

## 5. Verification
- [ ] 5.1 Run all deterministic, negative, advisory, and multi-language integration suites.
- [ ] 5.2 Run rustfmt and Clippy.
- [ ] 5.3 Verify requirement traceability, all three frontend conformance reports, and final compatibility with stable core, law, and lifecycle contracts.
