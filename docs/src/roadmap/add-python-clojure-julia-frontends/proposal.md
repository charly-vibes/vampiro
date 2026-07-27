> **Status:** Active OpenSpec proposal; not implemented or deployed. The source under `openspec/changes/` is authoritative.

# Change: Add Python, Clojure, and Julia frontends

## Why
The source specification requires cross-language analysis and names concrete visibility defects unique to these three ecosystems.

## What Changes
- Add independently staged, conformant full-CIR Python, Clojure, and Julia frontends with bounded provenance, law-runner inputs, lifecycle facts/unknowns, and L4 snapshots.
- Extract Python facade metadata, Clojure private-var dereferences, and Julia type-piracy facts; language-neutral core owns shared facade-leak behavior.

## Impact
- Affected capability: `additional-frontends`
- Source requirements: REQ-V5, REQ-V6; conformance references: REQ-1–3, REQ-V1–V2, REQ-V7, REQ-10, REQ-17, REQ-T1–T3

## Dependencies and Order
Each language may start independently once CIR/platform contracts stabilize. Final acceptance integrates against stable core, law, and lifecycle contracts, but does not require those entire changes to complete first; all three languages remain required for completion.
