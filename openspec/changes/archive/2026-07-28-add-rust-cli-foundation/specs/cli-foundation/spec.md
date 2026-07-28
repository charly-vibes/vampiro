## ADDED Requirements

### Requirement: Stable command and configuration foundation
The system SHALL expose a `vampiro` binary and reserve `check` and `prove`. Before implementation it SHALL select/document configuration filename, format, discovery and precedence and exact numeric success, policy-failure, and operational/configuration-failure codes, then snapshot-test those contracts. This change SHALL NOT specify CI-generation spelling or claim analysis/proof behavior. Conformance references: REQ-5, REQ-12.

#### Scenario: Commands are discoverable before integrations exist
- **WHEN** a user requests CLI help
- **THEN** `check`, `prove`, and configuration options are shown without claiming analysis or proof was performed

### Requirement: Shared finding and exit contracts
The system SHALL define a shared finding envelope containing rule ID, file path, exact line range, configured severity, optional `filtration_distance` equal to mathematical `sev(e)`, and exactly one axis from `composition`, `modularity`, `optionality`, or `robustness`. Redundancy belongs to robustness; plugin diagnostics are outside findings. Source requirement: REQ-4; filtration evidence supports platform-owned REQ-C2.

#### Scenario: A capability constructs a finding
- **WHEN** a later capability creates a finding through the shared contract
- **THEN** all required fields and exactly one axis are required, and no secondary axis can be represented

#### Scenario: Filtration evidence is attached
- **WHEN** a later capability constructs a finding under an active filtration
- **THEN** the envelope preserves configured severity separately from `filtration_distance = sev(e)` evidence
