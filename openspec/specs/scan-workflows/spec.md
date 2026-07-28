# scan-workflows Specification

## Purpose
TBD - created by archiving change add-scan-gating-reporting. Update Purpose after archive.
## Requirements
### Requirement: Scans are diff-default and incrementally complete
The system SHALL default interactive and agent `check` invocations to seam-only diff scope and support explicit full-repository scope. Local default SHALL compare `HEAD` to a synthetic worktree target containing staged, unstaged, and untracked non-ignored files. An explicit target SHALL default to its first parent; `--base` SHALL resolve the merge base with the target; detached `HEAD` SHALL use the same rules. A target without a parent SHALL use Git's empty tree. Explicit revision inputs SHALL resolve to immutable commit IDs before analysis, and their selected parent or merge base SHALL be locally available. A missing shallow parent/merge base, non-Git context, unresolved CI revision, or failed/unavailable fetch required to obtain a CI commit or merge base SHALL return an operational error naming scope, base/target inputs, reason, and explicit-full guidance, with no silent full fallback. Every result SHALL identify resolved scope and base/target commit IDs. In full scope, zero unchanged compatible files SHALL be re-extracted; compatibility SHALL be keyed by source content plus analyzer, schema, plugin, and configuration versions, with observable cache hit/miss/invalidation telemetry. Source requirements: REQ-5, REQ-28.

#### Scenario: Unchanged repository is rescanned fully
- **WHEN** a compatible full-scan cache exists and only one file changed
- **THEN** only that file's CIR is re-extracted while the complete repository is checked

#### Scenario: Interactive and agent checks default to diff
- **WHEN** an interactive user or agent invokes `check` without a scope option
- **THEN** only changed seams are selected and scope metadata says `diff`

#### Scenario: Full scope is explicit
- **WHEN** the caller explicitly requests full-repository scope
- **THEN** every supported repository file participates via fresh or compatible cached CIR

#### Scenario: Diff context is unavailable
- **WHEN** the merge base is absent from a shallow clone, a required CI revision cannot resolve or be fetched, or the context is not a Git repository
- **THEN** check returns an operational error with scope, base/target inputs, reason, and explicit-full guidance and performs no full scan

#### Scenario: Local worktree and detached target resolve normally
- **WHEN** a local worktree has staged, unstaged, or untracked non-ignored changes, or `HEAD` is detached with an available parent
- **THEN** diff scope resolves the specified base and target deterministically and reports both in result metadata

### Requirement: Analysis coverage is explicit
The system SHALL include every file lacking a frontend as `unanalyzed` rather than silently omit it. Source requirements: REQ-15.

#### Scenario: Unsupported extension is present
- **WHEN** a selected scan contains a file with no available frontend
- **THEN** every output format identifies that file as unanalyzed

### Requirement: Workflow modes apply deterministic policy
The system SHALL support `guidance`, `tiered`, and `gate` modes; guidance SHALL not fail for findings, tiered SHALL classify configured reporting tiers, and gate SHALL exit non-zero only when a seam-scoped finding meets the configured severity threshold. Gate SHALL ignore `filtration_distance` unless project configuration declares a mapping that passes schema, totality, and determinism validation. Source requirements: REQ-13, REQ-14, REQ-C2.

#### Scenario: Below-threshold finding in gate mode
- **WHEN** gate mode finds only seam findings below the threshold
- **THEN** findings are reported and the policy exit remains successful

#### Scenario: Equal-threshold finding fails gate
- **WHEN** a seam-scoped finding severity equals the configured gate threshold
- **THEN** it is reported and the policy exit is non-zero

#### Scenario: Guidance reports all and succeeds
- **WHEN** guidance mode observes findings at multiple severities
- **THEN** it reports every finding and does not fail solely because of them

#### Scenario: Filtration mapping is validated before gating
- **WHEN** a project mapping omits a filtration distance, maps one input nondeterministically, or violates the configuration schema
- **THEN** configuration is rejected before findings are gated

#### Scenario: Valid filtration mapping controls configured severity
- **WHEN** a total deterministic schema-valid mapping maps a finding's `filtration_distance` to the gate threshold
- **THEN** the mapped configured severity is reported and the gate exits non-zero

### Requirement: Reports share data and stable identities
The system SHALL render identical normalized finding data as human, JSON, or SARIF, preserving configured severity separately from `filtration_distance = sev(e)` evidence when active, and deduplicate the same issue across scopes by an ID derived from rule, location, and shape hash. Source requirements: REQ-19, REQ-24; supports REQ-C2.

#### Scenario: One issue appears in two scopes
- **WHEN** full and diff scan inputs contain the same underlying issue
- **THEN** the normalized result has one stable finding represented equivalently by all formats

### Requirement: CI generation encodes pull-request gating
The system SHALL generate pipeline configuration that runs diff-scoped `check` in `gate` mode for pull requests, passes the pull request's head commit as the explicit target, and passes its base ref or commit through `--base`. The generated workflow SHALL make both revisions and their merge base locally available, resolve them to immutable commit IDs, and return the diff-context operational error without broadening scope if resolution or a required fetch fails. Source requirements: REQ-5, REQ-20.

#### Scenario: CI configuration is requested
- **WHEN** a user selects a supported CI target
- **THEN** generated configuration invokes the required pull-request command and mode with its explicit target bound to the provider's pull-request head commit and `--base` bound to the provider's base ref or commit

#### Scenario: Required pull-request history is unavailable
- **WHEN** the generated workflow cannot resolve or fetch the pull-request head, base, or merge-base object
- **THEN** the check returns an operational error naming the input revisions and reason and does not silently perform a full scan

### Requirement: Small diff checks meet the interaction target
The system SHALL complete a diff-scoped check of at most 50 seam edges in less than 9 seconds under the documented reference hardware, repository state, cache state, plugin set, and measurement method. Source requirements: REQ-27.

#### Scenario: Fifty-edge benchmark runs
- **WHEN** the reference benchmark checks 50 seam edges
- **THEN** its measured duration is reproducible and passes only when below 9 seconds

