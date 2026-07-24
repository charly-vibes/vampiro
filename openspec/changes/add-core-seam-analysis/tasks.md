## 1. Analysis Tests
- [ ] 1.1 Add failing structural-unification and opaque-shape tests (REQ-7, REQ-23).
- [ ] 1.2 Add distinct advisory/enforced, Rust over-exposure, facade-leak, and arbitrary-depth visibility fixtures (REQ-8, REQ-V3–V4, REQ-V7, REQ-C5).
- [ ] 1.3 Add direct result/option/throws exact-discard-line, nested-layer, ancestor-throws, custom-effect, `unwrapped` partial versus all-summands-total, panic/force-unwrap, and robustness-axis redundancy fixtures (REQ-9, REQ-11, REQ-25, REQ-C4, REQ-C7).

## 2. Engine
- [ ] 2.1 Implement structural unification and side-by-side mismatch evidence.
- [ ] 2.2 Implement language-neutral visibility/facade checking and plugin diagnostic asymmetry.
- [ ] 2.3 Implement recursive coproduct resolution and bounded ancestor handling search.
- [ ] 2.4 Implement arbitrary-branch redundancy common-codomain checking.

## 3. Verification
- [ ] 3.1 Run unit/property tests and Rust frontend E2E negative fixtures.
- [ ] 3.2 Run rustfmt and Clippy.
- [ ] 3.3 Verify all owned IDs map to tests, findings use exactly one of four axes, plugin diagnostics are outside findings, and integration excludes scan/law/lifecycle behavior.
