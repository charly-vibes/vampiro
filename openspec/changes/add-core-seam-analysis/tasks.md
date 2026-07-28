> Implementation is blocked until the roadmap approval gate records approval of the EARS source and this proposal.

## 0. Shape Canonicalization Decision Gate (HITL)
- [x] 0.1 Compare structural normalization/hash alternatives against unions, opaque shapes, stable dedupe inputs, cross-version fixtures, and collision handling; record the choice in `docs/decisions/shape-canonicalization.md`.
- [x] 0.2 Record compatibility/version policy, rejected alternatives, approver, and immutable review reference before analysis implementation.

## 1. Composition Tracer
- [x] 1.1 Add failing structural-unification, union-arm, opaque-shape exclusion, and side-by-side mismatch-evidence tests (REQ-7, REQ-23).
- [x] 1.2 Implement coarse structural normalization/unification under the approved canonicalization contract and preserve opaque edges for non-composition checks.
- [x] 1.3 Run focused unit/property tests plus one Rust frontend E2E negative fixture; record expected finding fields and passing command output.

## 2. Modularity Tracer
- [x] 2.1 Add failing advisory/enforced, arbitrary-depth visibility, Rust over-exposure, facade-leak, and plugin-diagnostic fixtures (REQ-8, REQ-V3–V4, REQ-V7, REQ-C5).
- [x] 2.2 Implement language-neutral visibility/facade reachability, exactly-one-axis modularity findings, and enforced-unreachable plugin diagnostics outside findings.
- [x] 2.3 Run focused fixtures plus a Rust facade E2E test; record finding/diagnostic schemas and passing evidence.

## 3. Effect Handling Tracer
- [ ] 3.1 Add failing direct result/option/throws discard-line, nested/custom effect, ancestor-boundary, ordinary partial/total unwrap, and panic/force-unwrap fixtures (REQ-9, REQ-25, REQ-C4).
- [ ] 3.2 Implement recursive coproduct resolution, independent totality, and memoized bounded ancestor handling search.
- [ ] 3.3 Run focused effect and Rust E2E negative tests; verify swallowed findings use only the robustness axis and preserve exact discard evidence.

## 4. Redundancy Tracer
- [ ] 4.1 Add failing arbitrary-branch common-codomain, explicit-adapter, incompatible-effect, and no-cocone fixtures (REQ-11, REQ-C7).
- [ ] 4.2 Implement deterministic redundancy reconciliation over any branch count without introducing a fifth or combined axis.
- [ ] 4.3 Run focused property/E2E tests and record common-codomain evidence plus passing commands.

## 5. Core Acceptance and Result Contract
- [ ] 5.1 Run all four slice suites, Rust E2E negatives, workspace formatting, and Clippy with warnings denied.
- [ ] 5.2 Publish the normalized finding/result consumer contract and compatibility fixture used by scan and lifecycle; verify every owned requirement maps to a test.
- [ ] 5.3 Run `openspec validate add-core-seam-analysis --strict` and record contract version, fixture path, commands, and passing evidence location.
