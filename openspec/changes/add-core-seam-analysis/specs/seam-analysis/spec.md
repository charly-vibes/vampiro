## ADDED Requirements

### Requirement: Structural seams unify conservatively
The system SHALL raise a `composition` finding with expected and produced shapes when a new or modified edge does not structurally unify; `shape:opaque` SHALL skip only composition checking and remain eligible for modularity and robustness checks. Source requirements: REQ-7, REQ-23.

#### Scenario: Union arm is not accepted
- **WHEN** a produced union contains an arm absent from the target domain
- **THEN** a composition finding shows both shapes and identifies the unmatched arm

### Requirement: Visibility boundaries distinguish source and plugin defects
The system SHALL check visibility legitimacy at arbitrary filtration depth, report advisory reach-through, Rust over-exposure, and facade leaks as modularity findings, and report an enforced-unreachable crossing as a plugin diagnostic rather than a source finding. Source requirements: REQ-8, REQ-V3, REQ-V4, REQ-V7, REQ-C5.

#### Scenario: Advisory internal target is reached
- **WHEN** an edge crosses a language-permitted advisory boundary absent from the legitimate subcategory
- **THEN** a modularity finding names the level, boundary kind, and crossed boundary

#### Scenario: Enforced crossing appears in CIR
- **WHEN** a plugin emits an edge the language enforcement makes unreachable
- **THEN** the plugin receives `boundary:enforced-unreachable` and source receives no modularity finding

#### Scenario: Rust declaration is over-exposed
- **WHEN** a Rust item is exported above its declared legitimate visibility level
- **THEN** a modularity over-exposure finding identifies the item and levels

#### Scenario: Facade leaks an internal origin
- **WHEN** an L4 facade re-exports an item whose origin is absent from the permitted facade subcategory
- **THEN** a distinct facade-leak finding identifies both facade and origin

### Requirement: Effect resolution is recursively total
The system SHALL inspect every nested built-in or project-declared coproduct layer from declarations, report swallowed `result`, `option`, or `throws` effects at the exact discard line, and in diff scope report unchecked `throws` only when no matching ancestor handler exists before a declared boundary. `unwrapped` SHALL record wrapper removal but SHALL NOT imply totality: totality requires explicit handling of every summand, and panic/force unwrap is partial/swallowed. Source requirements: REQ-9, REQ-25, REQ-C4.

#### Scenario: Direct result option and throws are discarded
- **WHEN** direct edges discard `result`, `option`, and `throws` values
- **THEN** each robustness finding names its effect and the exact source line where that value is discarded

#### Scenario: Nested layers are handled independently
- **WHEN** an edge handles outer `async` and `result` but discards inner `option`
- **THEN** only the unresolved `option` layer produces a swallowed-effect finding

#### Scenario: Unwrapped edge is still partial
- **WHEN** an edge removes a result wrapper by force unwrap or handles only the success summand
- **THEN** it produces a robustness swallowed-effect finding; an edge explicitly handling every summand does not

#### Scenario: Multiple nested layers are discarded
- **WHEN** one edge partially unwraps and discards both a `result` layer and its nested `option` layer
- **THEN** exactly two swallowed-effect findings identify their respective layers and discard spans

#### Scenario: Ancestor handles an unchecked exception
- **WHEN** a seam swallows `throws` but an ancestor path handles that exception before the process boundary
- **THEN** no swallowed-throws robustness finding is raised for that seam

### Requirement: Redundancy branches have an explicit common codomain
The system SHALL check every redundancy branch for a common legitimate codomain and effect channel, requiring explicit adapters where branches differ, regardless of branch count, and SHALL classify divergence only on the robustness axis. Source requirements: REQ-11, REQ-C7.

#### Scenario: Fallback differs without adapter
- **WHEN** a fallback branch has a different shape or effect from the primary path and no adapter reconciles it
- **THEN** a robustness finding identifies the divergent branch
