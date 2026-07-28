> **Status:** Active OpenSpec proposal; not implemented or deployed. The source under `openspec/changes/` is authoritative.

## ADDED Requirements

### Requirement: Trust provenance and refinement are explicit contracts
The system SHALL attach trust provenance separately from CIR argument provenance to every CIR value occurrence (node parameter/output, sum arm, or edge argument slot), using exactly `untrusted`, `trusted`, or `unknown`. Frontends SHALL recognize trust-boundary sources and smart constructors only through versioned, conformance-tested idioms or explicit project declarations. A recognized external source SHALL be untrusted; a recognized internal literal and smart-constructor success-arm value SHALL be trusted; a non-success-arm payload or ordinary derived occurrence SHALL be untrusted if any contributor is untrusted, otherwise unknown if any contributor is unknown, otherwise trusted. A path over argument-provenance bound `H`, an unmatched pattern, or a declaration/idiom conflict SHALL be unknown and emit `trust-provenance:unknown` without defaulting to trusted. A smart constructor SHALL return a distinct refined shape through a `Result`, `Option`, or equivalent sum-typed codomain. Trust classification SHALL remain independent from external refinement confirmation. Source requirements: REQ-B1, REQ-B2, REQ-B6.

#### Scenario: Raw input reaches a recognized constructor
- **WHEN** a declared external source produces a raw value that flows through a recognized smart constructor and the caller totally handles every constructor outcome
- **THEN** CIR records `untrusted` before refinement and `trusted` only for the successful refined value while retaining the independent argument-provenance chain

#### Scenario: Trust classification is unknown
- **WHEN** no source, propagation, or refinement idiom or declaration matches a node output
- **THEN** the output remains `unknown`, `trust-provenance:unknown` is emitted without a finding axis, and the output is not treated as trusted

#### Scenario: Contributors have mixed trust
- **WHEN** a derived value has one untrusted and one trusted contributor, one unknown and one trusted contributor, or only trusted contributors
- **THEN** its trust provenance is respectively `untrusted`, `unknown`, or `trusted`

#### Scenario: Provenance exceeds its bound or declarations conflict
- **WHEN** a dependency path exceeds `H` or matching declaration and idiom classifications disagree
- **THEN** the value occurrence is `unknown` with `trust-provenance:unknown`, never trusted

#### Scenario: Similar function is not a smart constructor
- **WHEN** a function returns the same primitive shape after checking a condition but has no recognized idiom or explicit smart-constructor declaration
- **THEN** it is not classified as a smart constructor or used to establish trusted provenance

### Requirement: Proven raw flow produces a boundary-leak finding
The system SHALL emit exactly one default-`HIGH` robustness `boundary-leak` finding per violating edge when proven `untrusted` raw data flows into an interior node that is neither a trust-boundary node nor a recognized smart constructor. The finding SHALL identify the source, edge, and interior target and use REQ-24's rule/location/shape deduplication identity. `unknown` trust provenance SHALL remain visible as a diagnostic and SHALL NOT produce a boundary-leak finding by default. Source requirement: REQ-B3; total refinement handling reuses REQ-C4.

#### Scenario: Raw external value enters the interior
- **WHEN** a declared request-field source flows directly or through propagated dataflow into an ordinary interior function
- **THEN** one robustness `boundary-leak` finding identifies the source, carrying edge, and target

#### Scenario: Raw external value enters its smart constructor
- **WHEN** the same raw value flows into a recognized smart constructor
- **THEN** no boundary-leak finding is emitted for that edge

#### Scenario: Trust provenance is unknown
- **WHEN** an edge into an interior node carries `unknown` rather than proven `untrusted` trust provenance
- **THEN** no boundary-leak finding is emitted and the trust-provenance coverage diagnostic remains in the run result

### Requirement: Duplicate validation requires equivalence evidence
Frontends SHALL extract a validation observation containing stable validation identity, recognized smart constructor and refined shape, exact source span, and recognition origin `declaration` or `idiom`. The system SHALL emit exactly one default-`LOW` modularity `validation-duplication` finding per duplicate-check location only when a node outside the recognized smart constructor repeats validation tied to the same stable identity by project declaration or conformance-tested idiom. Mere syntactic similarity SHALL NOT establish equivalence. The finding SHALL include the observation fields and use REQ-24's rule/location/shape deduplication identity. Source requirement: REQ-B4.

#### Scenario: Declared validation is repeated
- **WHEN** an interior node checks a constraint with the same declared validation identity as the refined shape's recognized smart constructor
- **THEN** one modularity `validation-duplication` finding links the duplicate check to that constructor and identity

#### Scenario: Similar check has no identity evidence
- **WHEN** an interior condition is syntactically similar to a smart-constructor check but has no shared declaration or conformance-tested idiom
- **THEN** no validation-duplication finding is emitted

### Requirement: Refinement confirmation requires current complete boundary evidence
Before evidence import implementation, the system SHALL select and document a versioned evidence schema and compatibility policy. Imported boundary-coverage evidence SHALL identify its producer/version, analyzed revision, smart-constructor stable identity and source/shape hash, every project-declared boundary-class ID, and each class result. The system SHALL emit `refinement_confirmation.status=confirmed` only when the declared class set is non-empty and unique, evidence matches the analyzed revision and constructor identity/hash, and every declared class is present and passing. Every other case SHALL emit `status=unknown` with exactly one primary reason: the first applicable value in the ordered vocabulary `{absent, malformed, unsupported-version, stale, mismatched, empty-classes, duplicate-class, incomplete, unknown-class, failing}`. A downstream consumer SHALL derive `unreachable` from refinement evidence only when status is `confirmed`. Source requirement: REQ-B5.

#### Scenario: Every declared boundary class passes
- **WHEN** current evidence matches the analyzed revision and constructor identity/hash and reports passing results for exactly the complete declared boundary-class set
- **THEN** the normalized result contains `refinement_confirmation.status=confirmed` and a downstream reachability consumer may report `unreachable`

#### Scenario: Evidence cannot confirm the refinement
- **WHEN** evidence is absent, malformed, unsupported, stale, identity/hash-mismatched, empty, duplicate, incomplete, contains an undeclared class, or reports any non-passing class
- **THEN** the normalized result contains `refinement_confirmation.status=unknown` with the applicable closed primary reason and no consumer reports `unreachable` from that evidence
