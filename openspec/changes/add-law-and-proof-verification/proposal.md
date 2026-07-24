# Change: Add law and proof verification

## Why
Signature compatibility alone cannot establish substitutability, and formal proof must remain distinct from property-test evidence.

## What Changes
- Add project-declared replacing/augmenting law suites, registered language runners, backend-neutral obligation IR, explicit unsupported results, optional Lean/Dafny/TLA+ adapters, exact statuses, and combined evidence.

## Impact
- Affected capability: `law-verification`
- Source requirements: REQ-10, REQ-12, REQ-16–18, REQ-26, REQ-C6

## Dependencies and Order
Depends only on stable CLI finding/command contracts, CIR platform contracts, and the Rust frontend's runner-input milestone. This change owns the first registered Rust law runner. It does not depend on completed scan and never executes source during static `check`.
