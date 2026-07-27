> **Status:** Active OpenSpec proposal; not implemented or deployed. The source under `openspec/changes/` is authoritative.

## ADDED Requirements

### Requirement: Additional languages map independently into CIR
Each Python, Clojure, and Julia frontend SHALL conform to platform-owned full node, edge, domain/codomain shape, recursive built-in/project effect, resolution, visibility, and configured bounded callee-to-caller argument-provenance contracts. Each visibility table SHALL be independently versioned and tested from its effect table. Conformance references: REQ-1, REQ-2, REQ-3, REQ-V1, REQ-V2.

#### Scenario: One frontend fails conformance
- **WHEN** the Julia frontend fails its visibility fixtures while Python and Clojure pass
- **THEN** Julia is not loaded and the other conformant frontends remain independently eligible

#### Scenario: Each language records bounded provenance
- **WHEN** each language fixture passes a callee value directly, through at most `H` local bindings, and through more than `H`
- **THEN** each frontend records direct, bounded-chain, and explicit over-bound provenance respectively

### Requirement: Additional frontends supply law and lifecycle facts
Each frontend SHALL extract implementation-cluster membership, proof tags, and serializable values or generator references and SHALL register a compatible per-language runner that consumes the backend-neutral obligation IR. Every supported value or construct SHALL execute through that runner; an explicit unsupported result is permitted only for the particular value or construct the runner cannot represent, not for the language as a whole. Each frontend SHALL also extract language lifecycle facts and idioms for writes, retry idempotency, acquire/release, and exit paths, or emit an explicit `unknown` classification, and SHALL integrate persistence of each language's L4 facade snapshot. Conformance references: REQ-10, REQ-17, REQ-T1, REQ-T2, REQ-T3.

#### Scenario: Dynamic idiom is unsupported
- **WHEN** a Python, Clojure, or Julia construct prevents lifecycle or runner extraction
- **THEN** the relevant fact is explicitly unknown or unsupported rather than omitted

#### Scenario: Each supported language executes a law
- **WHEN** a supported Python, Clojure, or Julia cluster member and law are representable by that language's registered runner
- **THEN** the runner consumes the obligation IR and returns property evidence rather than a language-wide unsupported result

#### Scenario: Language facade snapshot is integrated
- **WHEN** an L4 facade is extracted for Python, Clojure, or Julia
- **THEN** its qualified identities and shapes enter lifecycle snapshot persistence

### Requirement: Python facades preserve declaration origins
The Python frontend SHALL extract package facade re-exports and underlying declaration visibility so the core can detect facade leaks without treating Python's advisory access as enforced. Conformance references: REQ-V1, REQ-V2, REQ-V7; generic contracts are platform/core-owned.

#### Scenario: Package initializer re-exports an internal symbol
- **WHEN** `__init__.py` re-exports a declaration from an internal module
- **THEN** CIR records both the facade edge and original visibility for core analysis

### Requirement: Clojure private-var reach-through is reported
The system SHALL raise a modularity `reach-through` finding when Clojure dereferences a `^:private` or `defn-` var outside its defining namespace even though the runtime permits it. Source requirements: REQ-V6.

#### Scenario: External namespace dereferences private var
- **WHEN** a Clojure edge resolves to another namespace's private var
- **THEN** a reach-through finding identifies the defining namespace and advisory boundary

### Requirement: Julia type piracy is reported
The system SHALL raise a modularity `type-piracy` finding when Julia defines a method for both a foreign generic function and a foreign type. Source requirements: REQ-V5.

#### Scenario: Package extends foreign generic for foreign type
- **WHEN** the Julia frontend resolves both owners outside the current package
- **THEN** a type-piracy finding names the foreign generic and foreign type
