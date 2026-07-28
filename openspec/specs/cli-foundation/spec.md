# cli-foundation Specification

## Purpose
TBD - created by archiving change add-rust-cli-foundation. Update Purpose after archive.
## Requirements
### Requirement: Stable command and configuration foundation
The system SHALL expose a `vampiro` binary and reserve `check` and `prove`. Before implementation it SHALL select/document configuration filename, format, discovery and precedence and exact numeric success, policy-failure, and operational/configuration-failure codes, then snapshot-test those contracts. This change SHALL NOT specify CI-generation spelling or claim analysis/proof behavior. Conformance references: REQ-5, REQ-12.

#### Scenario: Commands are discoverable before integrations exist
- **WHEN** a user requests CLI help
- **THEN** `check`, `prove`, and configuration options are shown without claiming analysis or proof was performed

### Requirement: Shared finding and exit contracts

The system SHALL define a shared finding envelope containing rule ID, file path,
exact line range, configured severity, optional `filtration_distance` equal to
mathematical `sev(e)`, and exactly one axis from `composition`, `modularity`,
`optionality`, or `robustness`. Redundancy belongs to robustness; plugin
diagnostics are outside findings. JSON serialization SHALL wrap this contract
in `genesis::envelope::Envelope`, with findings nested under `data`. Source
requirement: REQ-4; filtration evidence supports platform-owned REQ-C2.

#### Scenario: A capability constructs a finding

- **WHEN** a later capability creates a finding through the shared contract
- **THEN** all required fields and exactly one axis are required, and no secondary axis can be represented

#### Scenario: Filtration evidence is attached

- **WHEN** a later capability constructs a finding under an active filtration
- **THEN** the envelope preserves configured severity separately from `filtration_distance = sev(e)` evidence

#### Scenario: check emits shared envelope

- **WHEN** `vampiro check --json` is run
- **THEN** the emitted JSON SHALL have top-level keys `ok`, `envelope_version`, `cli_version`, `envelope_kind`, `data`, `warnings`, `hints`, `meta`
- **AND** the composition findings SHALL be nested under `data`.

### Requirement: Unknown-command suggestions use the shared engine

Vampiro SHALL source unknown-command suggestions from
`genesis::suggestions::SuggestionEngine` and SHALL NOT define a local
suggestion engine or `Suggestion` enum.

#### Scenario: typo suggestion from genesis

- **WHEN** an unknown vampiro subcommand is run
- **THEN** vampiro SHALL emit a "Did you mean …?" footer via `genesis::suggestions`
- **AND** SHALL NOT define a local `Suggestion` enum.

### Requirement: Managed project blocks use the shared injector

Vampiro SHALL use `genesis::managed_block` to insert and update its WAI,
OPENSPEC, and DONT managed blocks idempotently while preserving all content
outside those blocks.

#### Scenario: Managed blocks are applied twice

- **WHEN** the same managed-block update is applied twice to a project file
- **THEN** the second output SHALL be byte-identical to the first
- **AND** surrounding user-authored content SHALL remain unchanged.

#### Scenario: Project is detected by wai

- **WHEN** Vampiro's three managed blocks are current
- **THEN** `wai status` SHALL detect the Vampiro project integration.

### Requirement: AIX artifacts use the shared renderer

Vampiro SHALL generate deterministic `llms.txt` and `llm.txt` artifacts through
`genesis::aix` from authoritative repository inputs and SHALL NOT maintain a
second local renderer.

#### Scenario: AIX artifacts are regenerated

- **WHEN** unchanged authoritative inputs are rendered repeatedly
- **THEN** `llms.txt` and `llm.txt` SHALL be byte-identical across runs.

