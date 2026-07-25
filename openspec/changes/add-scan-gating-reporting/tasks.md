> Implementation is blocked until the roadmap approval gate records approval of the EARS source and this proposal.

## 0. CI Provider and Tier Policy Decision Gate (HITL)
- [ ] 0.1 Compare initial CI provider set and tiered-policy configuration shapes against PR-head/base resolution, unavailable fetch, equal-threshold, and invalid-mapping examples; record the choice in `docs/decisions/scan-policy.md`.
- [ ] 0.2 Record supported providers, event/ref variables, tier semantics, exclusions, approver, and immutable review reference before CI/policy implementation.

## 1. Git Scope and Incremental Cache Tracer
- [ ] 1.1 Add failing synthetic-worktree, staged/unstaged/untracked, explicit-target/base, detached/initial/shallow/non-Git, failed-fetch, no-silent-fallback, explicit-full, and versioned cache invalidation tests (REQ-5, REQ-15, REQ-28).
- [ ] 1.2 Implement deterministic diff scope with immutable base/target metadata and incremental full extraction with observable hit/miss/invalidation reasons.
- [ ] 1.3 Run cold/warm focused integration tests proving zero compatible unchanged re-extractions and operational errors without scope broadening; attach passing commands.

## 2. Normalized Result and Rendering Tracer
- [ ] 2.1 Add failing normalized-result, explicit `unanalyzed`, stable-dedupe, configured-severity versus `filtration_distance`, canonical-byte, and human/JSON/SARIF parity tests (REQ-15, REQ-19, REQ-24, REQ-C2).
- [ ] 2.2 Implement one normalized result and derive all three renderers and stable IDs from it.
- [ ] 2.3 Run parity, dedupe, canonical-byte, and core-result compatibility tests; publish the result schema/version and fixture path.

## 3. Policy and CI Tracer
- [ ] 3.1 Add failing guidance/tiered/gate, below/equal threshold, valid/invalid filtration mapping, and approved-provider CI golden tests including explicit PR head/base and failed-fetch behavior (REQ-13–14, REQ-20, REQ-C2).
- [ ] 3.2 Implement deterministic policy exits and CI generation from the approved decision, gating on configured severity only after validated mapping.
- [ ] 3.3 Run focused policy/CLI tests and each approved provider golden; record generated workflow paths and passing evidence.

## 4. Performance and Workflow Acceptance
- [ ] 4.1 Document hardware, OS, repository state/size, cache state, plugin/tool/config/platform versions, fixture, warm-up, repetitions, and statistic; add reproducible 1/50-edge benchmarks (REQ-27).
- [ ] 4.2 Run cold/warm, output parity, CI golden, and benchmark suites; pass only when the 50-edge result is below 9 seconds under the published profile.
- [ ] 4.3 Run workspace formatting/Clippy and `openspec validate add-scan-gating-reporting --strict`; record requirement traceability, contract versions, commands, and evidence location.
