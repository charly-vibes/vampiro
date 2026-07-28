## ADDED Requirements

### Requirement: Rust source maps conservatively into CIR
The Rust frontend SHALL conform to platform-owned node, edge, shape, recursive built-in/project effect, resolution, and configured bounded callee-to-caller argument-provenance contracts without executing source, preserving `opaque`, `unknown`, and over-bound classifications. Conformance references: REQ-1, REQ-2, REQ-3.

#### Scenario: Rust callable contains nested effects
- **WHEN** Rust syntax declares a callable and calls across a nested result/option boundary
- **THEN** the frontend emits deterministic callable, edge, shape, and recursive effect CIR records

#### Scenario: Rust provenance covers all bound cases
- **WHEN** fixtures pass a callee value directly, through at most `H` local bindings, and through more than `H`
- **THEN** extraction respectively records direct provenance, the bounded chain, and an explicit over-bound marker

### Requirement: Rust visibility metadata is conformant
The Rust frontend SHALL conform to platform-owned declaration visibility and independently versioned visibility-table contracts, including module ancestry and crate facade/export metadata. Conformance references: REQ-V1, REQ-V2.

#### Scenario: Crate facade re-exports an item
- **WHEN** a `pub use` exposes a module item at the crate facade
- **THEN** the frontend records both underlying and facade visibility metadata for later language-neutral checks

### Requirement: Rust extraction exposes lifecycle hooks
The system SHALL extract conservative write-shape and acquire/release/exit-path facts required by lifecycle analysis without itself issuing lifecycle findings. It SHALL also extract implementation-cluster membership, proof tags, and serializable values or generator references required by registered Rust law runners. Conformance references: REQ-T2, REQ-T3, REQ-10, REQ-17.

#### Scenario: Write and cleanup syntax is recognized
- **WHEN** a Rust scope contains a recognized write or resource-management idiom
- **THEN** the frontend emits the corresponding extraction facts for downstream classification
