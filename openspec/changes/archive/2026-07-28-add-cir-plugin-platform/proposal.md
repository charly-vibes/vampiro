# Change: Add CIR plugin platform

## Why
Language-independent checks require a deterministic, extensible representation and trustworthy plugin boundary.

Currently, vampiro has no CIR types, no frontend contract, and no way to represent effects, shapes, or provenance. Analysis logic cannot be written because there is no common data model to analyze.

This proposal introduces the CIR (Composition IR) data model, frontend extraction contract, and plugin boundary — the foundation that all analysis (seam, modularity, law, trust-boundary) depends on.

## What Changes
- Define CIR nodes, provenance-bearing edges, structural shapes, built-in/project-declared recursive effect coproducts, classifications, and finite decidable category/filtration declarations.
- Define frontend/resolver contracts via the `Frontend` trait, validation, conformance fixtures, functoriality/naturality checks, and safe unknown/conflict behavior.

## Impact
- **Affected capability:** `cir-plugin-platform`
- **Source requirements by group:**
  - **CIR data model:** REQ-1, REQ-2, REQ-3
  - **Plugin conformance:** REQ-6, REQ-22, REQ-29, REQ-C10, REQ-V1, REQ-V2
  - **Unknown/error safety:** REQ-21
  - **Category/filtration:** REQ-C1, REQ-C2, REQ-C3, REQ-C8, REQ-C9

## Success Criteria
1. All tasks in `tasks.md` are complete.
2. `openspec validate add-cir-plugin-platform --strict` passes.
3. A consumer test verifies that the CLI can import `vampiro-cir` types and call `Frontend::extract()`.
4. All traced requirements have verified test coverage.

## Dependencies and Order
- Depends on the CLI foundation contracts (finding envelope, config, exit codes).
- CIR/plugin contract stabilization unlocks all frontends (Rust, Python, Clojure, Julia).
- Section 2 (category/filtration) can proceed in parallel with frontend implementation.
- Section 3 (plugin load) is partially deferred until at least one frontend exists (see YAGNI note in tasks.md).
- The CIR types themselves need not wait for any frontend implementation.