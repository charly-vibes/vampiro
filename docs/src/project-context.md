> **Source:** Generated from the `context` block of `openspec/config.yaml` during the documentation build.

Vampiro is a cross-language static-analysis CLI that checks whether code
composes correctly at call, module, and trust boundaries: structural
composition mismatches, module reach-through, law-level substitutability
failures, discarded or inconsistent effect channels, and unrefined external
data crossing into trusted interiors. It never executes analyzed source; it
normalizes each language into a shared Composition IR (CIR).

The authoritative product requirements live in `vampiro-ears-spec.md`.
Preserve its requirement IDs (`REQ-*`, `REQ-V*`, `REQ-C*`, `REQ-T*`,
`REQ-B*`) in proposals, tests, findings, and traceability notes.

Stack: Rust workspace exposing the `vampiro` CLI; stable toolchain, rustfmt,
Clippy, Cargo-native unit/integration/doc tests; tree-sitter or
language-native parsers behind front-end plugins; CIR graph of callable nodes
and derived call edges; human, JSON, and SARIF output; optional Lean, Dafny,
and TLA+ proof backends that never enter the default `check` path.

Conventions: use the EARS domain vocabulary consistently (node, edge, seam,
shape, effect channel, resolution, finding, `filtration_distance`, facade,
implementation cluster, redundancy chain). Built-in vocabularies stay closed;
project extensions are declared and validated. Unknown shapes, wrappers,
resolutions, idempotency, and identities stay `unknown`, `opaque`, or
`ambiguous` — never silently coerced to a successful/default case. A finding
always has a rule ID, file, line range, severity, and exactly one axis.

Architecture: language-specific parsing and idiom recognition live in
versioned, conformance-tested front-end and resolver plugins; checking logic
is language-independent and operates on CIR, not source ASTs — a new language
adds a plugin, never a core branch. Domain/codomain shapes are modeled
structurally, deliberately coarser than a full type checker; effect channels
are recursive (`async<result<option<T>>>` retains every layer); legitimate
morphisms and filtrations are declarative data, and invalid subcategories or
non-nested filtrations are rejected. Diff-scoped seam analysis is the
interactive/agent default; full-repository analysis is an incremental mode.
Property testing and formal proof are distinct evidence — never report a
property-test result as a prover result.

Testing: every behavior starts from an openspec scenario traceable to EARS
requirement IDs. Maintain a shared, versioned conformance-fixture suite for
every front-end, effect resolver, visibility table, and write-shape table;
canonical UTF-8 result bytes and plugin-load manifests must be identical when
tool, plugin, config, and platform versions are unchanged. Test all output
formats against the same finding data; use property-based tests for declared
algebraic laws across every member of an implementation cluster; add prover
checks only for explicitly tagged obligations. Include negative fixtures for
unknown/opaque classifications, plugin conflicts, invalid filtrations,
advisory visibility crossings, fallback mismatches, unsafe retries, and
unreleased resource paths. Benchmark diff-scoped checks up to 50 seam edges
with a single-digit-second target.

Reporting axes: `composition` (produced and expected shapes do not unify,
including facade changes across versions); `modularity` (source crosses or
leaks a visibility/facade boundary); `optionality` (implementation matches the
signature but fails its declared laws); `robustness` (an effect, fallback,
retry, or acquire/release obligation is swallowed or threaded inconsistently,
or untrusted raw data crosses inward without a recognized refinement step).
The built-in visibility model is a five-level lattice from `L0 private` to
`L4 public-stable`; projects may declare deeper filtrations. Boundary kind
matters: advisory crossings are source findings, an allegedly enforced
crossing is a plugin defect. Findings are morphisms allowed by the ambient
category but absent from the declared legitimate subcategory;
`filtration_distance` is separate evidence, not configured severity.

Constraints: static analysis must never execute analyzed source; unknown or
unsupported input must be visible in output, never silently omitted or
treated as valid; every plugin must pass the shared conformance suite before
loading, and conflicting plugins are both rejected; `shape:opaque` nodes are
excluded only from composition checks, their edges stay eligible for
modularity and robustness; duplicate findings are identified stably from
rule, location, and shape hash; `guidance` mode never fails solely because of
findings while `gate` mode fails only when seam-scoped findings meet the
configured threshold; prover unavailability is `ProverUnavailable`, never
`Disproved` or success; N+1 detection, documentation truth maintenance, and
lock-order analysis are out of scope; trust provenance is distinct from CIR
argument provenance, and unknown, absent, stale, or incomplete refinement
evidence never establishes that an invalid state is unreachable.

The repository is greenfield: `openspec/specs/` describes built truth. Do not
copy aspirational EARS requirements there before implementation — add them
through reviewed changes and archive them as capabilities ship.
