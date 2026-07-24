## ADDED Requirements

### Requirement: Implementation clusters satisfy declared theories
The system SHALL load project-declared law suites that explicitly replace or augment named built-ins, and dispatch every implementation-cluster member through its registered per-language law-runner plugin using extracted membership, proof tags, and serializable values or generator references. Unsupported runner or execution SHALL be an explicit result and SHALL never silently skip a member. Source requirements: REQ-10, REQ-18, REQ-C6.

#### Scenario: One model violates commutativity
- **WHEN** two cluster members implement the signature and only one fails the declared commutativity equation
- **THEN** an optionality finding names that member, law, and property-test evidence

#### Scenario: Custom suite replaces a built-in
- **WHEN** project configuration marks custom suite `S` as replacing built-in `monoid`
- **THEN** runners execute only `S` for every cluster member

#### Scenario: Custom suite augments a built-in
- **WHEN** project configuration marks custom suite `S` as augmenting built-in `monoid`
- **THEN** runners execute both suites for every cluster member

#### Scenario: Language runner is unsupported
- **WHEN** a cluster member has no compatible registered runner or cannot serialize its generator reference
- **THEN** that member receives explicit `RunnerUnsupported` evidence and is not treated as passing or skipped

### Requirement: Obligation IR is backend neutral
The system SHALL lower declared equations and tagged obligations into a versioned backend-neutral obligation IR before any runner or prover adapter. One end-to-end language runner SHALL consume this IR before Lean, Dafny, and TLA+ adapters are staged. Analyzed source SHALL execute only on an explicitly requested property/law path and never during static `check`.

#### Scenario: Static check encounters law metadata
- **WHEN** default static `check` extracts a cluster and proof tags
- **THEN** it emits metadata/obligations without executing analyzed source

### Requirement: Formal obligations use explicit prover adapters
The system SHALL translate backend-neutral tagged obligations through configured Lean or Dafny algebraic adapters or TLA+ concurrent-composition adapters and report exactly `Proved`, `Disproved`, `Timeout`, or `ProverUnavailable`, never substituting property evidence for proof evidence. Source requirements: REQ-12, REQ-16, REQ-17.

#### Scenario: Prover proves an obligation
- **WHEN** the adapter returns a valid proof before its deadline
- **THEN** the status is `Proved`

#### Scenario: Prover disproves an obligation
- **WHEN** the adapter returns a valid counterexample
- **THEN** the status is `Disproved` with counterexample evidence

#### Scenario: Prover times out
- **WHEN** the configured deadline expires without a result
- **THEN** the status is `Timeout`, not `Disproved` or `Proved`

#### Scenario: Configured prover cannot start
- **WHEN** a tagged obligation is dispatched and its prover is missing or misconfigured
- **THEN** the result is `ProverUnavailable`, is not a pass, and is not `Disproved`

### Requirement: Proof remains optional and evidence is combined
The system SHALL keep proof independent of default `check`; when optionality and proof are both enabled for the same obligation, it SHALL run property tests for every member, prove tagged members, and combine both evidence channels into one finding. Source requirements: REQ-17, REQ-26.

#### Scenario: A tagged member fails both checks
- **WHEN** its property test fails and the prover disproves the matching obligation
- **THEN** one finding contains both separately labeled results
