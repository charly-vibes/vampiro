# Project Context

## Purpose
Vampiro is a cross-language static-analysis CLI that checks whether code
composes correctly at call and module boundaries. It detects four classes of
seam defects: structural composition mismatches, module reach-through,
law-level substitutability failures, and discarded or inconsistent effect
channels. It analyzes source without executing it and normalizes each
language into a shared Composition IR (CIR).

The authoritative product requirements are in `vampiro-ears-spec.md`. Preserve
its requirement IDs (`REQ-*`, `REQ-V*`, `REQ-C*`, and `REQ-T*`) in proposals,
tests, findings, and traceability notes.

## Tech Stack
- Implementation language: Rust, organized as a Cargo workspace and exposed
  as the `vampiro` CLI binary.
- Rust baseline: stable toolchain, rustfmt formatting, Clippy linting, and
  Cargo-native unit, integration, and documentation tests.
- Language analysis: tree-sitter or a language-native parser behind
  front-end plugins.
- Core model: a language-agnostic graph (CIR) of callable nodes and derived
  call edges.
- CLI output: human-readable, JSON, and SARIF.
- Optional proof backends: Lean, Dafny, and TLA+; proof support must remain
  independent of the default `check` path.

## Project Conventions

### Code Style
- Format with rustfmt and keep Clippy clean under the lint policy established
  by the CLI-foundation change.
- Use domain terms from the EARS specification consistently: node, edge,
  seam, shape, effect channel, resolution, finding, `filtration_distance`, facade,
  implementation cluster, and redundancy chain.
- Keep built-in vocabularies closed and project extensions explicitly
  declared and validated. Unknown shapes, wrappers, resolutions, idempotency,
  and identities remain `unknown`, `opaque`, or `ambiguous`; never silently
  coerce them to a successful/default case.
- Use stable requirement and rule IDs. A finding always has a rule ID, file,
  line range, severity, and exactly one axis.

### Architecture Patterns
- Keep language-specific parsing and idiom recognition in versioned,
  conformance-tested front-end and resolver plugins.
- Keep checking logic language-independent and operate on CIR rather than
  source ASTs. A new language should add a plugin, not branch the core engine.
- Model domain/codomain shapes structurally, deliberately coarser than a full
  type checker.
- Represent effect channels recursively so combinations such as
  `async<result<option<T>>>` retain every layer.
- Define legitimate morphisms and filtrations as declarative data;
  reject invalid subcategories or non-nested filtrations.
- Treat diff-scoped seam analysis as the interactive/agent default and full
  repository analysis as an incremental mode.
- Keep property testing and formal proof as distinct evidence. Never report a
  property-test result as a prover result.

### Testing Strategy
- Develop every behavior from an OpenSpec scenario traceable to one or more
  EARS requirement IDs.
- Maintain a shared, versioned conformance-fixture suite for every front-end,
  effect resolver, visibility table, and write-shape table. Canonical UTF-8
  serialized result and plugin-load manifest bytes must be identical when tool,
  plugin, configuration, and platform versions are unchanged.
- Test all supported output formats against the same underlying finding data.
- Use property-based tests for declared algebraic laws across every member of
  an implementation cluster; add optional prover checks only for explicitly
  tagged obligations.
- Include negative fixtures for unknown/opaque classifications, plugin
  conflicts, invalid filtrations, advisory visibility crossings, fallback
  mismatches, unsafe retries, and unreleased resource paths.
- Benchmark diff-scoped checks with up to 50 seam edges; the target is
  single-digit seconds.

### Git Workflow
- Use OpenSpec changes for new capabilities, architecture changes, breaking
  behavior, and performance work. Do not implement a proposal before approval.
- Keep commits focused and preserve requirement IDs in change artifacts and
  test names where practical.
- Pull-request CI should run diff-scoped `check` in `gate` mode at the
  configured severity threshold unless configuration declares a validated
  mapping from `filtration_distance` (mathematical `sev(e)`).

## Domain Context
Vampiro asks whether each edge is valid in the category in which it claims to
compose. Its four reporting axes are:

- `composition`: produced and expected structural shapes do not unify,
  including facade changes across versions.
- `modularity`: source crosses or leaks a declared visibility/facade boundary.
- `optionality`: an implementation matches an interface signature but fails
  its declared algebraic laws.
- `robustness`: an effect, fallback, retry, or acquire/release obligation is
  swallowed or threaded inconsistently.

The built-in visibility model is a five-level lattice from `L0 private` to
`L4 public-stable`, but projects may declare deeper filtrations. Boundary kind
matters: advisory crossings are source findings; an allegedly enforced
crossing indicates a plugin defect. Findings are morphisms allowed by the
ambient category but absent from the declared legitimate subcategory.
Their `filtration_distance` is separate evidence, not configured severity.

## Important Constraints
- Static analysis must never execute analyzed source code.
- Unknown or unsupported input must be visible in output, never silently
  omitted or treated as valid.
- Every plugin must pass the shared conformance suite before loading;
  conflicting plugins must both be rejected.
- `shape:opaque` nodes are excluded only from composition checks; their edges
  remain eligible for modularity and robustness analysis.
- Duplicate findings are identified stably from rule, location, and shape
  hash.
- `guidance` mode never fails solely because of findings; `gate` mode fails
  only when seam-scoped findings meet the configured threshold.
- Prover unavailability is `ProverUnavailable`, not `Disproved` or success.
- N+1 detection, documentation truth maintenance, and lock-order analysis are
  explicitly out of scope because they require other extraction models or
  belong to other tools.
- The repository is greenfield. `openspec/specs/` describes built truth, so do
  not copy aspirational EARS requirements there before implementation; add
  them through reviewed changes and archive them as capabilities ship.

## External Dependencies
- Git diff/repository metadata for seam detection, incremental extraction, and
  historical facade snapshots.
- Per-language tree-sitter grammars or native parser APIs.
- Property-based testing libraries selected with each implementation plugin.
- Optional Lean, Dafny, and TLA+ installations or services.
- CI platforms capable of consuming generated pipeline configuration and SARIF.
