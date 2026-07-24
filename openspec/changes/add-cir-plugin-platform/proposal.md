# Change: Add CIR plugin platform

## Why
Language-independent checks require a deterministic, extensible representation and trustworthy plugin boundary.

## What Changes
- Define CIR nodes, provenance-bearing edges, structural shapes, built-in/project-declared recursive effect coproducts, classifications, and finite decidable category/filtration declarations.
- Define frontend/resolver contracts, validation, conformance fixtures, functoriality/naturality checks, and safe unknown/conflict behavior.

## Impact
- Affected capability: `cir-plugin-platform`
- Source requirements: REQ-1–3, REQ-6, REQ-21–22, REQ-29, REQ-C1–C3, REQ-C8–C10, REQ-V1–V2

## Dependencies and Order
Depends on the CLI foundation contracts. CIR/plugin contract stabilization unlocks all frontends; it need not wait for unrelated foundation implementation.
