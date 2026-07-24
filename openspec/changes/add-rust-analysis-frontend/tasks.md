## 1. Frontend Fixtures
- [ ] 1.1 Add failing Rust fixtures for full node/edge/shape effects and direct/within-bound/over-bound argument provenance.
- [ ] 1.2 Add failing visibility fixtures for module ancestry, `pub` forms, `pub use`, hidden/internal conventions, and facades (REQ-V1–V2).
- [ ] 1.3 Add failing extraction fixtures for lifecycle facts, clusters, proof tags, serializable values/generator references, unknowns, macros, and unsupported constructs.

## 2. Extraction
- [ ] 2.1 Evaluate the recommended parser against fixtures, record the confirmed choice, and implement syntax-to-CIR extraction.
- [ ] 2.2 Implement versioned Rust effect and visibility tables plus facade metadata independently.
- [ ] 2.3 Implement lifecycle extraction hooks without lifecycle classification/findings.

## 3. Contract Milestones
- [ ] 3.1 Publish and verify the first Rust CIR E2E milestone for core consumers.
- [ ] 3.2 Publish and verify the Rust cluster/tag/serializable-value/generator runner-input milestone for the law change; runner implementation remains law-owned.
- [ ] 3.3 Publish and verify the Rust write/resource/exit-path extraction milestone for lifecycle consumers.

## 4. Verification
- [ ] 4.1 Run CIR conformance, deterministic fixture, negative, and Rust integration tests.
- [ ] 4.2 Run rustfmt and Clippy.
- [ ] 4.3 Verify all generic IDs are conformance references and each milestone satisfies its platform contract independently.
