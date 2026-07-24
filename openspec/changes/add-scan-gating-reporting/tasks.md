## 1. Workflow Tests
- [ ] 1.1 Add failing local synthetic-worktree, staged/unstaged/untracked, explicit-target first-parent, explicit-base merge-base, detached-HEAD, initial-empty-tree, shallow-missing-base, unresolved/failed-fetch CI revision, non-Git, no-silent-full-fallback, explicit-full, immutable scope/base/target metadata, unsupported-file, and reason/guidance tests (REQ-5, REQ-15, REQ-28).
- [ ] 1.2 Add failing incremental tests proving zero compatible unchanged files are re-extracted; cover source-content and analyzer/schema/plugin/config invalidation with observable cache telemetry (REQ-28).
- [ ] 1.3 Add guidance-all/success, below/equal-threshold gate, tiered, interactive/agent diff-default, explicit-full, valid filtration mapping, and schema/non-total/nondeterministic mapping rejection tests (REQ-5, REQ-13–14, REQ-C2).
- [ ] 1.4 Add human/JSON/SARIF configured-severity and `filtration_distance` parity, dedupe, validated-mapping gate, and scan-owned CI-generation goldens that bind the explicit target to each provider's pull-request head commit and `--base` to its base ref or commit, fetch required history, and fail operationally without scope broadening when revision resolution or fetch fails (REQ-5, REQ-19–20, REQ-24, REQ-C2).
- [ ] 1.5 Document hardware, OS, repository size/state, cache state, plugin/tool/config/platform versions, fixture, warm-up, repetitions, and statistic; add reproducible 1/50-edge benchmarks enforcing <9 seconds at 50 (REQ-27).

## 2. Workflows
- [ ] 2.1 Implement Git-aware diff scope and versioned incremental full extraction.
- [ ] 2.2 Implement normalized run results, explicit `unanalyzed` coverage, and stable dedupe.
- [ ] 2.3 Implement modes, threshold exit behavior, three renderers, and CI generation.

## 3. Verification
- [ ] 3.1 Run cold/warm integration, output parity, CI golden, and performance tests.
- [ ] 3.2 Run rustfmt and Clippy.
- [ ] 3.3 Verify requirement traceability, canonical UTF-8 serialized result bytes under unchanged inputs, and E2E integration with CLI/core.
