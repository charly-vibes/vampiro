> **Status:** Active OpenSpec proposal; not implemented or deployed. The source under `openspec/changes/` is authoritative.

## ADDED Requirements

### Requirement: Facade history detects authorized and breaking evolution
Before persistence implementation, the system SHALL select and document snapshot storage, schema/version, retention, baseline override, and migration declaration syntax. It SHALL persist each analyzed revision's L4 facade by qualified identity and compare by default with the nearest persisted first-parent ancestor. An explicit baseline SHALL exist and be an ancestor or produce an operational error. A first snapshot SHALL establish history without a breaking finding; multiple eligible snapshots SHALL resolve deterministically. A persistent incompatible shape without a declared migration SHALL raise a composition `breaking-change` finding. Source requirements: REQ-T1, REQ-T4.

#### Scenario: Persistent facade item changes incompatibly
- **WHEN** two snapshots match an item deterministically, its shape has a breaking edge, and no migration authorizes it
- **THEN** a breaking-change finding identifies the old and new shapes

#### Scenario: No usable explicit baseline
- **WHEN** an explicit baseline is missing or is not an ancestor
- **THEN** lifecycle analysis returns an operational error rather than choosing another snapshot

### Requirement: Ambiguous facade identity remains explicit
The system SHALL report `identity:ambiguous` when a moved or renamed facade item cannot be matched deterministically without a declared alias, rather than silently treating it as removed and added. Source requirements: REQ-T8.

#### Scenario: Item moves without alias
- **WHEN** two plausible historical matches exist for a moved facade item
- **THEN** identity is ambiguous and no independent add/remove classification is asserted

### Requirement: Retry idempotency is table-driven and conservative
The system SHALL classify retried edges using a versioned, conformance-tested write-shape idiom table; non-idempotent retries SHALL raise `unsafe-retry`, while unmatched write shapes SHALL remain `unknown` and surface a coverage gap without raising unsafe-retry by default. Source requirements: REQ-T2, REQ-T5, REQ-T9.

#### Scenario: Non-idempotent append is retried
- **WHEN** a retried edge has an append write shape and no idempotency mechanism
- **THEN** a robustness unsafe-retry finding names both the write shape and missing mechanism

#### Scenario: Write idiom is unknown
- **WHEN** no idempotency-table entry matches a retried write
- **THEN** the class is unknown, a coverage gap is emitted, and no default unsafe-retry finding is raised

### Requirement: Idempotency laws cross-reference retry findings
The system SHALL check a declared retry idempotency equation `f;f = f` through property/proof verification when enabled and SHALL issue a distinct optionality finding cross-referenced with any unsafe-retry finding on the edge. Source requirements: REQ-T6.

#### Scenario: Retried operation fails its declared law
- **WHEN** enabled law verification finds `f;f` differs from `f`
- **THEN** an optionality finding links to, but remains distinct from, the edge's robustness finding

### Requirement: Every acquisition has total release coverage
The system SHALL assign every acquisition a unique obligation/resource identity and match it one-to-one with a release on every reachable normal, early-return, error, or panic/abort path. Explicit transfer SHALL move the same obligation identity to the recipient. A duplicate release SHALL NOT discharge another obligation; any identity mismatch or insufficient release multiplicity that leaves an obligation pending SHALL raise the REQ-T7 `resource-leak` finding. An unresolved alias or identity SHALL emit the exact `identity:unknown` coverage diagnostic and SHALL NOT assert safety. Source requirements: REQ-T3, REQ-T7.

#### Scenario: Early return bypasses cleanup
- **WHEN** an acquire scope has an early-return path with no matching release
- **THEN** a resource-leak finding names the acquisition and that unreleased path

#### Scenario: Alias cannot be resolved
- **WHEN** release may target an acquisition through an unknown alias
- **THEN** `identity:unknown` is emitted and the acquisition is not classified safe

#### Scenario: Duplicate release cannot discharge another resource
- **WHEN** resources A and B are acquired and B is released twice while A remains pending
- **THEN** the second release does not discharge A and REQ-T7 reports A as a `resource-leak`
