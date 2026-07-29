> Implementation is blocked until the roadmap approval gate records approval of the EARS source and this proposal.

## 0. Per-Language Parser Decision Gates (HITL)
- [x] 0.1 Evaluate Python parser options against supported versions, package/module loading, dynamic constructs, provenance, and performance; record scope, exclusions, approver, and review reference in `docs/decisions/python-frontend.md`.
- [x] 0.2 Evaluate Clojure parser options against supported versions, reader macros, namespace loading, private-var forms, provenance, and performance; record scope, exclusions, approver, and review reference in `docs/decisions/clojure-frontend.md`.
- [x] 0.3 Evaluate Julia parser options against supported versions, macro/module loading, ownership/type-piracy forms, provenance, and performance; record scope, exclusions, approver, and review reference in `docs/decisions/julia-frontend.md`.

## 1. Shared CIR Acceptance Contract
- [x] 1.1 Add per-language matrices for full node/edge/shape/effect extraction, direct/within/over-bound provenance, visibility, opaque/unknown, deterministic output, and construct-specific unsupported evidence (REQ-1–3, REQ-V1–V2).
- [x] 1.2 Add a versioned platform compatibility harness that consumes only published CIR/plugin contracts and can run independently for each language.
- [x] 1.3 Run the empty/reference harness for all three languages; publish matrix schema and fixtures under `tests/fixtures/add-python-clojure-julia-frontends/1/` and write the reference report to `reports/conformance/additional-frontends-v1.json`.

## 2. Python Extraction Tracer
- [x] 2.1 Add failing approved-version Python fixtures for CIR/provenance, effect/visibility idioms, dynamic unknowns, and `__init__.py` facade metadata.
- [x] 2.2 Implement the Python frontend and versioned idiom tables against only the shared CIR acceptance contract.
- [x] 2.3 Run deterministic CIR/visibility/advisory conformance and negative fixtures; publish frontend/table versions and report path.

## 3. Clojure Extraction Tracer
- [x] 3.1 Add failing approved-version Clojure fixtures for CIR/provenance, effects, namespace visibility, reader/unknown boundaries, and private-var metadata.
- [x] 3.2 Implement the Clojure frontend and versioned idiom tables against only the shared CIR acceptance contract.
- [x] 3.3 Run deterministic CIR/visibility/REQ-V6 conformance and negative fixtures; publish frontend/table versions and report path.

## 4. Julia Extraction Tracer
- [x] 4.1 Add failing approved-version Julia fixtures for CIR/provenance, effects, module visibility, macro/unknown boundaries, and generic-function/type ownership metadata.
- [x] 4.2 Implement the Julia frontend and versioned idiom tables against only the shared CIR acceptance contract.
- [x] 4.3 Run deterministic CIR/visibility/REQ-V5 conformance and negative fixtures; publish frontend/table versions and report path.

## 5. Python Law/Lifecycle/Core Integration Tracer
- [x] 5.1 Add failing Python fixtures for runner inputs/execution/unsupported, lifecycle facts/unknowns, L4 snapshots, and facade findings against published consumer contracts (REQ-10, REQ-17, REQ-T1–T3).
- [x] 5.2 Implement the registered Python runner and lifecycle facts without language-specific branches in core/law/lifecycle engines.
- [x] 5.3 Run Python runner, lifecycle snapshot, advisory, and core facade E2E tests; record contract versions and passing evidence.

## 6. Clojure Law/Lifecycle/Core Integration Tracer
- [x] 6.1 Add failing Clojure fixtures for runner inputs/execution/unsupported, lifecycle facts/unknowns, L4 snapshots, and private-var findings against published consumer contracts.
- [x] 6.2 Implement the registered Clojure runner and lifecycle facts without language-specific engine branches.
- [x] 6.3 Run Clojure runner, lifecycle snapshot, and REQ-V6 E2E tests; record contract versions and passing evidence.

## 7. Julia Law/Lifecycle/Core Integration Tracer
- [x] 7.1 Add failing Julia fixtures for runner inputs/execution/unsupported, lifecycle facts/unknowns, L4 snapshots, and type-piracy findings against published consumer contracts.
- [x] 7.2 Implement the registered Julia runner and lifecycle facts without language-specific engine branches.
- [x] 7.3 Run Julia runner, lifecycle snapshot, and REQ-V5 E2E tests; record contract versions and passing evidence.

## 8. Cross-Language Acceptance
- [x] 8.1 Run all deterministic, negative, advisory, runner, lifecycle, and multi-language integration suites plus workspace formatting and Clippy.
- [x] 8.2 Verify requirement traceability, all three conformance reports, and compatibility with the named CIR, core-result, law-evidence/runner, and lifecycle contracts.
- [x] 8.3 Run `openspec validate add-python-clojure-julia-frontends --strict` and record parser decisions, contract/table versions, commands, and evidence location.
