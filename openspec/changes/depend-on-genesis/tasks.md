> Implementation is blocked until the roadmap approval gate approves this proposal and Genesis publishes tag `v0.1.0` with the required modules.

## 1. Genesis Dependency and API Tracer
- [x] 1.1 Add a failing compatibility fixture requiring Genesis tag `v0.1.0` and public `envelope`, `suggestions`, `managed_block`, and `aix` APIs from the Rust workspace skeleton.
- [x] 1.2 Add the exact tagged Git dependency and minimal imports needed to make the compatibility fixture pass; keep Vampiro domain logic outside Genesis.
- [x] 1.3 Run the focused fixture, workspace build/tests, formatting, and Clippy; record the resolved commit and API evidence in `docs/verification/depend-on-genesis-1.md`.

## 2. Shared Envelope Tracer
- [x] 2.1 Add a failing `vampiro check --json` conformance fixture for exact top-level keys and composition findings nested under `data`.
- [x] 2.2 Route the normalized composition result through `genesis::envelope::Envelope` without moving finding or analysis logic into Genesis.
- [x] 2.3 Run focused JSON/golden and existing result-contract tests; record the envelope version, fixture, and passing commands in `docs/verification/depend-on-genesis-2.md`.

## 3. Shared Suggestions Tracer
- [x] 3.1 Add failing CLI fixtures for a close typo, an unrelated token, deterministic candidate ordering, and no locally defined suggestion engine.
- [x] 3.2 Register Vampiro's command list with `genesis::suggestions::SuggestionEngine` and route unknown-command footers through its result.
- [x] 3.3 Run focused CLI snapshots and workspace tests; record suggestion API/version and passing commands in `docs/verification/depend-on-genesis-3.md`.

## 4. Managed-Block Tracer
- [x] 4.1 Add failing fixtures for insert/update/idempotent replay of WAI, OPENSPEC, and DONT managed blocks while preserving surrounding user content.
- [x] 4.2 Source injector mechanics from `genesis::managed_block` and carry the three project blocks needed for `wai status` detection (`wai-bdqw.9`).
- [x] 4.3 Run focused byte/idempotency fixtures and a `wai status` integration check; record block/API versions and passing commands in `docs/verification/depend-on-genesis-4.md`.

## 5. AIX Artifact Tracer
- [ ] 5.1 Add failing golden fixtures for deterministic `llms.txt` and `llm.txt` generation from the repository's authoritative project inputs.
- [ ] 5.2 Generate both artifacts through `genesis::aix`, retaining no second local renderer or hand-maintained divergent content.
- [ ] 5.3 Run focused golden/idempotency tests and verify committed artifacts are current; record AIX API/version and passing commands in `docs/verification/depend-on-genesis-5.md`.
