---
tags: [requirements, ears, source-of-truth]
---

# EARS Specification: Vampiro

| Document control | Value |
|---|---|
| Document version | 1.1.0 |
| Status | Draft |
| Owner | Project maintainers |
| Last updated | 2026-07-24 |
| Approval | Not yet approved |
| Authoritative role | Normative, authoritative specification for Vampiro behavior; in a conflict with non-normative examples or notes, the requirements in this document govern. |

## Revision History

| Version | Date | Status | Change |
|---|---|---|---|
| 1.1.0 | 2026-07-24 | Draft | Remediated confirmed Rule of 5 findings: semantics, taxonomy, operational edge cases, measurability, traceability, and examples. |

Vampiro ("vampire" — Spanish) is a cross-language CLI that
checks whether the pieces of a codebase actually **compose**: whether call
boundaries line up structurally, whether module boundaries are respected,
whether interchangeable implementations are actually interchangeable, and
whether error/absence/retry channels are threaded through call chains
instead of silently discarded at the seam where two pieces meet.

This document is self-contained. It assumes no prior conversation, no other
document, and no familiarity with any other tool.

---

## 0. Background & Motivation

Two pieces of code can each be individually correct and still fail when
wired together. Four distinct failure modes recur across languages:

1. **Composition break** — the output shape of one function does not
   structurally match what the next function consumes, so the call site
   compiles or runs only because of an implicit coercion, an `any`/`object`
   escape hatch, or a runtime cast.

   ```python
   def parse_amount(raw: str) -> Decimal | None: ...
   def apply_discount(amount: Decimal, pct: float) -> Decimal: ...

   # seam:
   total = apply_discount(parse_amount(raw), 0.1)   # None slips through
   ```

2. **Modularity break** — a caller reaches past a module's declared
   interface into its internals, so the module can no longer be changed or
   swapped without breaking a caller that was never supposed to know about
   that detail.

   ```ts
   // billing/index.ts exports only `charge()`
   import { _internalLedgerCache } from "billing/internal/ledger"; // reach-through
   ```

3. **Optionality break** — two implementations satisfy the same interface
   type-wise but not law-wise, so they are not actually substitutable, even
   though nothing in the type system objects.

   ```rust
   trait Merge { fn merge(self, other: Self) -> Self; }

   impl Merge for RunningTotal { /* associative, as expected */ }
   impl Merge for LatestWins   { fn merge(self, other: Self) -> Self { other } }
   // LatestWins is NOT commutative/order-independent the way callers of
   // Merge assume when they fold in parallel or replay out of order.
   ```

4. **Robustness break** — an effect channel (error, absence, timeout, retry)
   present in a callee's return type is discarded at the call site instead
   of being propagated, transformed, or explicitly handled.

   ```go
   result, _ := riskyLookup(id)   // error silently discarded
   use(result)
   ```

   The same failure mode appears in redundancy/fallback constructs, which
   are meant to *increase* robustness but often introduce a second break:

   ```python
   try:
       data = primary_source.fetch(id)       # returns FullRecord
   except SourceUnavailable:
       data = cache.get(id)                  # returns PartialRecord | None
   use(data)   # caller assumes FullRecord on every path
   ```

None of these four are caught reliably by a type checker (types matched),
a linter (no syntax rule was violated), or a test suite (the untested path
is exactly the seam an agent just wrote). They are also not really separate
problems: (1) and (2) are about whether the *objects* and their declared
*morphisms* are well-formed; (3) and (4) are about whether composition is
happening in the category the code actually needs — plain function
composition, or the Kleisli category of some effect (Result, Option, retry)
— and whether that choice is honored consistently.

Vampiro treats all four as instances of one question, asked at every call
boundary: **does this edge compose validly, in the category it claims to
compose in?** It answers this without executing the code, across multiple
languages, using a shared intermediate representation rather than a full
type checker per language.

---

## 1. Scope & Definitions

- **Composition IR (CIR)**: a language-agnostic graph extracted from source,
  consisting of **nodes** (callables) and **edges** (call relationships).
- **Node**: one callable unit with a **domain shape**, a **codomain shape**,
  and an **effect channel**.
- **Domain shape / codomain shape**: a structural signature — parameter and
  return shapes described by name, arity, and shape tags (e.g. `record`,
  `list<T>`, `optional<T>`, `union<A,B>`) — deliberately coarser than a full
  type, so it can be extracted uniformly across languages without a full
  per-language type checker.
- **Effect channel**: a classification of what wraps a callable's return
  value. The closed built-in vocabulary is `plain`, `result`
  (error-or-value), `option` (value-or-absence), `throws` (unchecked
  exception), `async`, and `stream`; validated project declarations may add
  effect/functor IDs under REQ-C1. Built-in and declared IDs may combine
  recursively (e.g. `async<result>`). A wrapper that matches neither is the
  `unknown` sentinel, never silently `plain`.
- **Edge**: a directed relationship from callee to caller where an
  argument at the call site derives, directly or through a bounded number
  of intermediate local bindings, from the callee's return value.
- **Effect resolution**: how an edge handles the callee's effect channel.
  The closed built-in vocabulary is `propagated` (channel re-wrapped and passed up unchanged),
  `transformed` (channel mapped to a different but still-threaded
  representation), `unwrapped` (a wrapper was removed; this classification
  alone says nothing about totality), `swallowed` (effect
  discarded with no handling branch), or `retried` (effect triggers a
  bounded re-invocation before resolving another way). Validated project
  declarations may add resolution/natural-transformation IDs under REQ-C1;
  an unmatched pattern uses the `unknown` sentinel per REQ-21. Totality is
  recorded independently by checking whether every failure/absence summand
  has an intentional branch. Ordinary wrapper removal is
  `resolution=unwrapped` with independently computed totality. Panic/force
  removal (`unwrap`, `expect`, `try!`, and equivalents) records wrapper-removal
  evidence but is `resolution=swallowed, totality=partial` unless every
  failure/absence summand has an intentional branch.
- **Idiom table**: a per-language, versioned mapping from source patterns
  (e.g. Rust `.unwrap()`, Go `_, err :=` with `err` unused, Python bare
  `except: pass`, TypeScript missing `.catch`/`try`, Swift `try!`) to a
  built-in or validated project-declared effect-resolution ID, or to the
  `unknown` sentinel.
- **Seam**: an edge that is new or modified in the diff under evaluation,
  as opposed to the full repository graph.
- **Module boundary**: the exported interface of a source-level module or
  package, as declared by that language's native export mechanism
  (`pub`, `export`, `__all__`, public visibility modifiers, etc.).
- **Reach-through**: an edge or reference that resolves to a symbol not
  part of the target module's declared exported interface.
- **Law suite**: an ordered set of property-based tests attached to an
  interface (e.g. associativity and identity for a semigroup-shaped
  interface; `map(id) == id` and `map(f ∘ g) == map(f) ∘ map(g)` for a
  functor-shaped interface).
- **Implementation cluster**: the set of concrete nodes/types that
  structurally satisfy one interface's domain/codomain shape.
- **Redundancy chain**: a sequence of alternative call targets connected by
  fallback control flow (`try`/`catch` fallback, `.or_else`, retry loop,
  circuit breaker) intended to satisfy one logical request.
- **Front-end plugin**: a language-specific component that translates a
  source AST (via tree-sitter or a native parser) into CIR nodes and edges.
- **Effect resolver plugin**: a language-specific component implementing
  that language's idiom table.
- **Conformance fixture**: a fixed, versioned set of synthetic CIR graphs
  with known-correct classifications, used to validate any front-end or
  resolver plugin before it is loaded.
- **Visibility lattice**: a five-level ordering of how hidden a declaration
  is, from least to most accessible: `L0 private` (lexical/local scope
  only) < `L1 module-internal` (visible within its own module/namespace,
  not exported) < `L2 package-internal` (visible within its own
  package/crate, not exported beyond it) < `L3 public-unstable`
  (technically reachable from outside the package, but marked or
  positioned as not-for-external-use) < `L4 public-stable` (part of the
  declared facade). See Addendum V for the full per-language mapping.
- **Boundary kind**: `enforced` (the language/runtime prevents access
  from outside the boundary; a violation cannot compile or run) or
  `advisory` (the language permits access from outside the boundary;
  only convention discourages it). See Addendum V.
- **Facade**: the top-level, explicitly declared re-export surface of a
  package (e.g. a Rust crate root's `pub use` statements, a Python
  package's `__init__.py`, a Clojure project's designated API namespace,
  a Julia package's top-level module `export` list).
- **Finding**: one reported issue: rule ID, file, line range, severity, and
  one of the four axis categories — `composition`, `modularity`,
  `optionality`, `robustness`.
- **Finding taxonomy**: the preceding four values are the complete and exact
  set of finding axes. Shape legitimacy and facade compatibility map to
  `composition`; visibility maps to `modularity`; algebraic-law satisfaction
  maps to `optionality`; effect totality, retries, resource linearity, and
  redundancy map to `robustness`. Redundancy is a robustness check, never a
  fifth axis. Slash notation such as `robustness/composition` is prohibited
  because it could imply a combined axis. Operational/plugin diagnostics,
  including `boundary:enforced-unreachable`, are diagnostics and are not
  findings and do not have an axis.
- **Gate mode**: `guidance` (report only), `tiered` (report with
  escalating severity), or `gate` (non-zero exit at or above a threshold).
- **Prove mode**: an opt-in mode (`vampiro prove <target>`) that dispatches
  a tagged obligation to an external prover (Lean, Dafny, or TLA+ for
  concurrent composition) instead of, or in addition to, running the
  corresponding law suite as property tests.

---

## 2. Ubiquitous Requirements

*(always active, no trigger — "the system shall")*

- **REQ-1**: The tool shall extract a Composition IR from source files via
  a registered front-end plugin for each supported language, producing
  nodes with domain shape, codomain shape, and effect channel, and edges
  with source node, target node, and effect resolution, without executing
  the analyzed code.
- **REQ-2**: The tool shall classify every node's effect channel using a
  built-in ID from `{plain, result, option, throws, async, stream}`, a
  validated project-declared effect/functor ID under REQ-C1, or the
  `unknown` sentinel, including recursive combinations thereof; an
  unrecognized wrapper shall be `unknown` and never default to `plain`.
- **REQ-3**: The tool shall classify every edge's effect resolution using a
  built-in ID from `{propagated, transformed, unwrapped, swallowed, retried}`,
  a validated project-declared resolution/natural-transformation ID under
  REQ-C1, or the `unknown` sentinel, determined deterministically from the
  active language's idiom table.
- **REQ-4**: Every finding shall carry a rule ID, file path, line range,
  severity, and exactly one axis category from `{composition, modularity,
  optionality, robustness}`.
- **REQ-5**: The tool shall support both a full-repository scan and a
  diff-scoped scan restricted to seam edges; diff-scoped shall be the
  default for interactive and agent-facing invocation. Without explicit
  revisions, a local invocation SHALL compare `HEAD` with a synthetic current
  worktree target containing staged, unstaged, and untracked non-ignored
  files. With an explicit target commit, the default base SHALL be its first
  parent; with `--base <ref>`, the base SHALL be the merge base of that ref and
  the target. Generated pull-request CI SHALL pass the pull request's head
  commit as the explicit target and its base ref or commit through `--base`.
  Both inputs SHALL resolve to immutable commit IDs, and their merge base SHALL
  be locally available before analysis. Detached `HEAD` SHALL follow the same
  rules and SHALL not itself be an error. If `HEAD` or the target has no
  parent, the base SHALL be Git's empty tree. If a shallow clone lacks the
  selected parent or merge base, a CI revision cannot be resolved, or a fetch
  required to obtain a CI commit or merge base fails or is unavailable,
  diff-scoped scan SHALL return an operational error stating selected scope,
  base/target inputs, reason, and guidance to invoke an explicit `--full` scan;
  it SHALL never silently broaden the scan to full-repository scope. Every run
  result SHALL report selected scope and resolved base and target commit IDs.
  Explicit `--full` SHALL remain available in every such context.
- **REQ-6**: Every front-end plugin and every effect-resolver plugin shall
  pass the shared conformance fixture suite before the tool loads it.

---

## 3. Event-Driven Requirements

*(When \<trigger\>, the system shall \<response\>)*

- **REQ-7**: When a new or modified edge connects a codomain shape that
  does not structurally unify with the target's domain shape, the tool
  shall raise a `composition` finding showing the caller-expected shape
  and the callee-produced shape side by side.

  *Example*: `parse_amount(raw: str) -> Decimal | None` feeding into
  `apply_discount(amount: Decimal, pct: float)` without narrowing the
  `Decimal | None` union first raises a composition finding citing the
  unhandled `None` arm.

- **REQ-8**: When an edge resolves to a target whose visibility-lattice
  level is below what the caller's own position in the lattice permits,
  the tool shall raise a `modularity` (reach-through) finding naming the
  target's level, its boundary kind, and the boundary crossed. Per
  Addendum V REQ-V3, `enforced` crossings shall not occur in valid source
  and are not reported as findings; `advisory` crossings shall always be
  reported, since the language itself permits them.

  *Example*: `import { _internalLedgerCache } from "billing/internal/ledger"`
  where `billing/index.ts` exports only `charge()` raises a modularity
  finding on the import edge (`L2 package-internal`, `advisory`).

- **REQ-9**: When a seam edge resolves a `result`, `option`, or `throws`
  channel via an idiom classified `swallowed`, the tool shall raise a
  `robustness` finding naming the discarded effect channel and the exact
  line where it was discarded.

  *Example*: `result, _ := riskyLookup(id)` in Go, where `_` discards a
  named `error` return, raises a robustness finding on that line.

- **REQ-10**: When two or more nodes fall into the same implementation
  cluster and a law suite is declared for that interface, the tool shall
  run the law suite as property tests against every cluster member and
  raise an `optionality` finding for any member that fails a law another
  member satisfies.

  *Example*: `RunningTotal::merge` and `LatestWins::merge` both implement
  `Merge`; a commutativity law in the declared law suite passes for
  `RunningTotal` and fails for `LatestWins`, raising an optionality
  finding naming the failing member and the specific law.

- **REQ-11**: When a redundancy chain is detected, the tool shall compare
  codomain shape and effect channel across every branch and raise a
  `robustness` finding if any branch's shape or effect channel differs
  from the primary path's without an explicit adapter node reconciling
  the difference.

  *Example*: `primary_source.fetch(id) -> FullRecord` falling back to
  `cache.get(id) -> PartialRecord | None` with no adapter narrowing both
  branches to one shape raises a robustness finding on the fallback edge.

- **REQ-12**: When `vampiro prove <target>` is invoked against a node
  tagged with a formal obligation (e.g. `@law associative`), the tool
  shall translate the obligation to the configured external prover and
  report one of `Proved`, `Disproved`, or `Timeout`; it shall never
  substitute the corresponding property-test result for a prover result.

---

## 4. State-Driven Requirements

*(While \<state\>, the system shall \<behavior\>)*

- **REQ-13**: While gate mode is `gate`, the tool shall exit non-zero and
  block if any seam-scoped finding meets or exceeds the configured
  severity threshold.
- **REQ-14**: While gate mode is `guidance`, the tool shall report all
  findings and shall not force a non-zero exit on their account.
- **REQ-15**: While no front-end plugin is available for a file's
  language, the tool shall report that file as `unanalyzed` explicitly in
  its output, rather than omitting it silently.
- **REQ-16**: While a prover configured for `vampiro prove` is unreachable
  or misconfigured, the tool shall report `ProverUnavailable` as distinct
  from `Disproved`, and shall never treat unavailability as a pass.

---

## 5. Optional Feature Requirements

*(Where \<feature is included\>, the system shall \<behavior\>)*

- **REQ-17**: Where `--prove` is enabled, obligations tagged for formal
  proof shall be dispatched to the configured backend (Lean or Dafny for
  algebraic obligations; TLA+ for concurrent-composition obligations);
  this mode shall remain independent of, and never required for, the
  default `check` gate.
- **REQ-18**: Where a custom law suite is supplied for an interface, the
  tool shall use it in place of, or in addition to, the built-in law
  templates (semigroup, monoid, functor).
- **REQ-19**: Where an output format `human`, `json`, or `sarif` is
  selected, the tool shall serialize identical underlying finding data in
  that format.
- **REQ-20**: Where CI generation is requested, the tool shall emit a
  pipeline configuration that invokes diff-scoped `check` in `gate` mode
  on pull requests with the pull request's head commit as its explicit target
  and its base ref or commit passed through `--base`.

---

## 6. Unwanted Behavior Requirements

*(If \<trigger\>, then the system shall \<mitigation\>)*

- **REQ-21**: If an observed wrapper or unwrap pattern has no matching
  idiom-table entry for its language, the tool shall classify the edge as
  `resolution:unknown` and flag it for idiom-table review, rather than
  default it to `propagated`.
- **REQ-22**: If two loaded plugins produce conflicting classifications
  for the same conformance fixture, the tool shall refuse to load either
  and report the conflict, rather than silently prefer one.
- **REQ-23**: If a callable's domain or codomain shape cannot be extracted
  (fully dynamic, untyped, no annotations available), the tool shall
  classify it as `shape:opaque`, exclude it from composition-break
  checking, and continue to include its edges in modularity- and
  robustness-break checking.
- **REQ-24**: If the same underlying issue would be reported by both a
  full-repository scan and a diff-scoped scan, the tool shall deduplicate
  findings by a stable ID derived from rule, location, and shape hash.

---

## 7. Complex Requirements

*(compound trigger/state combinations)*

- **REQ-25**: While diff-scoped mode is active, when a seam edge's
  resolution is `swallowed` and the swallowed channel is `throws` in a
  language with unchecked exceptions, the tool shall additionally check
  reachability over the CIR for a handling branch on any ancestor call
  path up to a declared process/request boundary, and shall raise a
  `robustness` finding only if no ancestor path handles that exception
  type.
- **REQ-26**: Where optionality checks are enabled, when an
  implementation cluster's law suite includes an obligation also tagged
  for formal proof and `--prove` is enabled for that interface, the tool
  shall run the property-test version by default for every cluster
  member and additionally dispatch the tagged member(s) to the prover,
  combining both results into one finding rather than reporting them
  separately.

---

## 8. Non-Functional Requirements

- **REQ-27**: A diff-scoped `check` over at most 50 seam edges shall
  complete in less than 9 seconds under a published benchmark profile. The
  profile SHALL identify hardware, repository/corpus, cold- or warm-cache
  state, enabled plugin set, and measurement method.
- **REQ-28**: Full-repository CIR extraction shall be incremental,
  and SHALL re-extract zero unchanged compatible files. Cache identity and
  invalidation SHALL include source content plus analyzer, CIR schema,
  plugin, and effective configuration versions. The extraction report or
  telemetry SHALL expose cache hits, misses, and invalidation reasons so a
  test can assert the zero-re-extraction condition.
- **REQ-29**: The conformance fixture suite shall be versioned, and any
  plugin's pass/fail result against it shall be reproducible byte-for-byte
  across repeated runs on unchanged input. The compared byte artifact SHALL
  be the canonical UTF-8 serialization of the fixture result and load
  manifest. Reproducibility SHALL be evaluated with unchanged tool, plugin,
  configuration, and platform inputs.

---

## 9. Traceability Notes

The consolidated matrix in Section 10 is the authoritative traceability
index; the summaries below are navigation aids only.

- Composition-break detection → REQ-1, REQ-2, REQ-7, REQ-23.
- Modularity/reach-through detection → REQ-8.
- Optionality / substitutability via law suites → REQ-10, REQ-18, REQ-26.
- Robustness / effect-channel threading → REQ-3, REQ-9, REQ-11, REQ-21,
  REQ-25.
- Formal proof dispatch (opt-in, never required for the default gate) →
  REQ-12, REQ-16, REQ-17, REQ-26.
- Multi-language plugin architecture and self-conformance → REQ-1, REQ-6,
  REQ-15, REQ-22, REQ-29.
- Operational modes and CI integration → REQ-5, REQ-13, REQ-14, REQ-19,
  REQ-20, REQ-27, REQ-28.

## 10. Consolidated Requirement Traceability Matrix

This matrix covers every normative requirement ID without asserting
implementation status.

| Requirement | Capability / component | Verification method |
|---|---|---|
| REQ-1 | CIR front-end extraction | Golden AST-to-CIR fixtures; no-execution test |
| REQ-2 | Effect-channel classifier | Enumeration and unknown-wrapper fixtures |
| REQ-3 | Effect-resolution classifier | Versioned idiom-table fixtures |
| REQ-4 | Finding schema/taxonomy | JSON/SARIF schema and axis-enumeration test |
| REQ-5 | Scan context and diff base | Initial, shallow, non-Git, and `--full` CLI tests |
| REQ-6 | Plugin conformance gate | Reject/load integration fixtures |
| REQ-7 | Shape composition | Positive/negative structural-unification fixtures |
| REQ-8 | Visibility reach-through | Boundary-crossing fixtures |
| REQ-9 | Swallowed effects | Per-language discard fixtures |
| REQ-10 | Substitutability laws | Property-test pass/fail cluster fixtures |
| REQ-11 | Redundancy compatibility | Common-codomain adapter fixtures |
| REQ-12 | Prover result reporting | Mock backend status contract tests |
| REQ-13 | Gate mode | Exit-code threshold tests |
| REQ-14 | Guidance mode | Report and zero-exit tests |
| REQ-15 | Unsupported languages | Unanalyzed-file output test |
| REQ-16 | Prover availability | Unreachable/misconfigured backend tests |
| REQ-17 | Optional proof dispatch | CLI/backend integration tests |
| REQ-18 | Custom law suites | Override/addition configuration tests |
| REQ-19 | Output formats | Cross-format semantic equivalence test |
| REQ-20 | CI generation | Golden pipeline configuration test |
| REQ-21 | Unknown unwrap idioms | Coverage-gap fixture |
| REQ-22 | Conflicting plugins | Conflict rejection test |
| REQ-23 | Opaque shapes | Partial-analysis fixture |
| REQ-24 | Finding deduplication | Stable-ID full/diff test |
| REQ-25 | Ancestor exception handling | CIR reachability fixtures |
| REQ-26 | Combined test/proof evidence | Correlated-result integration test |
| REQ-27 | Diff-check latency | Published-profile benchmark |
| REQ-28 | Incremental extraction | Cache telemetry assertion test |
| REQ-29 | Fixture reproducibility | Canonical artifact byte comparison |
| REQ-V1 | Visibility extraction | Per-language visibility fixtures |
| REQ-V2 | Visibility idiom conformance | Version/load validation tests |
| REQ-V3 | Boundary-kind handling | Diagnostic-vs-finding fixture |
| REQ-V4 | Rust over-exposure | Rust facade fixture |
| REQ-V5 | Julia type piracy | Julia ownership fixture |
| REQ-V6 | Clojure private var dereference | Clojure namespace fixture |
| REQ-V7 | Facade leak | Re-export depth fixture |
| REQ-C1 | Declarative category models | Configuration/schema tests |
| REQ-C2 | Filtration distance | Membership/output-field tests |
| REQ-C3 | Arbitrary visibility depth | Variable-length filtration fixtures |
| REQ-C4 | Recursive effect totality | Nested-sum branch-coverage fixtures |
| REQ-C5 | Scope factorization | Reachability/path fixtures |
| REQ-C6 | Algebraic models | Signature/equation fixtures |
| REQ-C7 | Redundancy cocone | Common-codomain search fixtures |
| REQ-C8 | Subcategory validity | Identity/closure rejection tests |
| REQ-C9 | Filtration validity | Non-nesting rejection test |
| REQ-C10 | Functorial front-ends | Identity/composition/naturality fixtures |
| REQ-T1 | Facade snapshot baseline | Lineage, override, first-snapshot tests |
| REQ-T2 | Retry classification | Write-idiom conformance fixtures |
| REQ-T3 | Resource obligations | Identity, multiplicity, alias, transfer fixtures |
| REQ-T4 | Facade breaks | Compatible/breaking snapshot tests |
| REQ-T5 | Unsafe retries | Non-idempotent retry fixture |
| REQ-T6 | Retry law checking | Property/prover correlation test |
| REQ-T7 | Resource leaks | Exit-path discharge fixtures |
| REQ-T8 | Facade identity ambiguity | Rename/move fixture |
| REQ-T9 | Unknown retry coverage | Unknown-idiom diagnostic test |

---

## Addendum V: Layered Visibility for Rust, Python, Clojure, Julia

### V.0 Background & Motivation

"Modularity" is not a single boundary; a codebase hides things at several
nested levels at once (function-local, module, package/crate, and the
package's own declared public surface), and each level can be either
truly walled off by the language or merely marked as internal by
convention. Section 2–7's REQ-8 treats reach-through as one check, but
what counts as a violation — and whether the check is even reachable —
depends entirely on which level and which boundary kind is involved. This
addendum specifies that mapping for the first four target languages.

The four languages were chosen because they span the boundary-kind
spectrum: Rust enforces most of its levels at compile time; Python,
Clojure, and Julia enforce almost nothing above local lexical scope, and
rely on naming convention, documentation, and namespace/module
positioning instead. Static analysis tools built primarily against
statically-typed, enforcement-heavy languages therefore tend to
under-check exactly the three languages where the check matters most.

Julia's multiple-dispatch model introduces one further wrinkle with no
analogue in the other three: a package can modify another package's
runtime behavior by adding a method to a generic function it does not
own, for a type it does not own ("type piracy"). This is not a
reach-through in the visibility-lattice sense — nothing was accessed that
shouldn't have been — but it is a modularity violation of the same
family: a boundary that should have constrained *where behavior can be
defined* was crossed.

### V.1 Scope & Additional Definitions

- **Type piracy** (Julia-specific): a method definition that extends a
  generic function defined in neither the current package nor any
  package the current package owns, for a type defined in neither the
  current package nor any package the current package owns.
- **Var dereference** (Clojure-specific): access to a namespace's var via
  its fully-qualified reader form (`#'ns/name`) or `find-var`, which
  bypasses `:refer`/`:require` and can reach a `^:private`-tagged var from
  outside its defining namespace despite that var's declared privacy.
- **Visibility idiom table**: a per-language, versioned mapping from
  syntactic markers (visibility modifiers, naming conventions, namespace
  or module path segments, facade re-export presence/absence) to a
  `(lattice level, boundary kind)` pair, structured identically to the
  effect-resolution idiom table in Section 1 and validated by the same
  conformance-fixture mechanism (REQ-6).

**Per-language visibility idiom table:**

| Level | Rust | Python | Clojure | Julia |
|---|---|---|---|---|
| L0 private | no modifier (item-default); local bindings — *enforced* | local variables/closures — *enforced* | `let`/local bindings — *enforced* | local bindings inside a function body — *enforced* |
| L1 module-internal | `pub(self)` — *enforced* | single leading underscore `_name` — *advisory* | `defn-` / `^:private` metadata — *advisory* | top-level binding absent from `export` — *advisory* |
| L1.5 | `pub(super)` — *enforced* | double leading underscore `__name` in a class (name-mangled) — *advisory* | — | leading-underscore naming convention — *advisory* |
| L2 package-internal | `pub(crate)` / `pub(in path)` — *enforced* | `_internal/`/`internal/` subpackage convention, or absent from `__all__` — *advisory* | namespace segment convention (`.internal.`, `.impl.`) — *advisory* | submodule not re-exported by the top-level package module — *advisory* |
| L3 public-unstable | `pub` item marked `#[doc(hidden)]`, or `pub` with leading-underscore name — *advisory (technically enforced-open)* | present in a module but not re-exported by the package's `__init__.py` facade — *advisory* | public var not referred from the project's designated API namespace — *advisory* | exported from a submodule that is not itself part of the documented public surface — *advisory* |
| L4 public-stable | `pub`, re-exported at the crate root and documented — *enforced-open* | re-exported in `__init__.py` or listed in `__all__` — *advisory* | referred/re-exported from the designated API namespace — *advisory* | exported at the top-level package module and documented — *advisory* |

Note that only Rust has genuine `enforced` boundaries above L0; the other
three languages are `advisory` at every level above local scope, which is
exactly why REQ-V3's asymmetric treatment (below) matters in practice.

### V.2 Ubiquitous Requirements

- **REQ-V1**: Each front-end plugin shall classify every declaration's
  visibility-lattice level and boundary kind using that language's
  visibility idiom table.
- **REQ-V2**: Each language's visibility idiom table shall be versioned
  and validated against conformance fixtures under the same mechanism as
  REQ-6, independent of that language's effect-resolution idiom table.

### V.3 Event-Driven Requirements

- **REQ-V3**: When an edge crosses a visibility boundary, the tool shall
  determine the boundary kind before reporting. `Enforced` crossings
  shall not appear in valid source (the compiler/runtime already
  prevents them); if a front-end nonetheless reports one, the tool shall
  emit a `boundary:enforced-unreachable` diagnostic against that plugin
  rather than a modularity finding against the source. `Advisory`
  crossings shall always be raised as a `modularity` finding, since the
  language itself permits them.
- **REQ-V4**: When a declaration is at boundary kind `enforced-open`
  (Rust `pub`) but is also marked internal-by-convention (`#[doc(hidden)]`,
  leading-underscore name, or excluded from the crate root's `pub use`
  facade), the tool shall raise a `modularity` finding classified
  `over-exposure`, distinct from `reach-through`: the problem is that the
  item is reachable at all, not that a caller reached it improperly.
- **REQ-V5**: When a Julia front-end observes a method definition
  matching the type-piracy definition in V.1, the tool shall raise a
  `modularity` finding classified `type-piracy`, naming the foreign
  generic function and the foreign type.
- **REQ-V6**: When a Clojure front-end observes a var dereference
  resolving to a `^:private`/`defn-`-tagged var from outside its
  defining namespace, the tool shall raise a `modularity` finding
  classified `reach-through`, regardless of the access being permitted
  by the Clojure runtime.

### V.4 Unwanted Behavior Requirements

- **REQ-V7**: If a facade re-exports a symbol whose underlying
  declaration sits at a deeper (more hidden) lattice level than the
  facade's own `L4 public-stable` level, the tool shall raise a
  `modularity` finding classified `facade-leak`.

  *Example*: a Python package's `__init__.py` contains
  `from .internal.pricing import RawTierTable`, promoting an
  `L2 package-internal` symbol to the `L4` facade — this is a
  facade-leak finding, distinct from a caller-side reach-through, because
  the violation originates at the boundary's own declaration.

### V.5 Traceability Notes (Addendum)

- Lattice/boundary-kind classification and per-language mapping →
  REQ-V1–V2.
- Reach-through with enforced/advisory asymmetry → REQ-8, REQ-V3.
- Rust-specific over-exposure → REQ-V4.
- Julia-specific type piracy → REQ-V5.
- Clojure-specific var-deref reach-through → REQ-V6.
- Facade-originated leaks (any of the four languages) → REQ-V7.

---

## Addendum C: Categorical Semantics and Generic Level Extension

### C.0 Background & Motivation

Sections 2–7 and Addendum V specify composition, modularity, and
optionality checks using closed built-in vocabularies—six built-in effect
channels, five default visibility levels, and built-in law templates—plus
validated project extensions and explicit `unknown` sentinels. Each built-in
vocabulary is useful as a concrete default, but treating it as exhaustive hard-codes a
depth or a size that real codebases will exceed — a Cargo workspace
nests crates inside a workspace inside (sometimes) another workspace; a
monorepo nests packages inside teams inside an org; a theory of
substitutability shouldn't be limited to semigroup/monoid/functor.

This addendum gives one abstract semantics that the earlier sections
specialize, so that adding a deeper visibility hierarchy, a new effect
type, a richer law suite, or a new language requires supplying a new
*instance* of the same abstract data — never a change to the checking
logic itself.

### C.1 Scope & Definitions

- **Ambient category 𝒜** (per axis): objects are the entities being
  related for that axis (shapes, scopes, or interface signatures);
  morphisms are every relationship the target language actually permits
  the code to construct, legitimate or not — i.e. everything that
  compiles or runs.
- **Legitimate subcategory 𝒢**: a *wide* subcategory of 𝒜 (same
  objects, a subset of morphisms, containing every identity, closed
  under composition) — the morphisms Vampiro treats as valid for that
  axis.
- **Finding**: an edge that is a morphism of 𝒜 but does not factor
  through 𝒢. This generalizes REQ-4: every finding raised anywhere in
  this document, on any axis, is an instance of this one condition,
  specialized per axis (C.3).
- **Filtration**: a chain of wide subcategories `𝒢₀ ⊆ 𝒢₁ ⊆ … ⊆ 𝒢ₙ = 𝒜`
  of any length `n`, used to grade how far a non-legitimate edge is from
  full legitimacy.
- **Filtration distance**: for an edge `e`, `sev(e) = min{ i : e ∈ 𝒢ᵢ }` under
  a declared filtration; `sev(e) = ⊥` if `e ∉ 𝒜` at all — reserved for
  `boundary:enforced-unreachable` diagnostics (REQ-V3), which flag a
  plugin bug rather than a source-code finding. User-facing output names
  this independent evidence field `filtration_distance`. It is not the
  configured finding severity (`LOW`, `MEDIUM`, `HIGH`, or project-defined
  equivalents).
- **Coproduct elimination**: where an effect channel is realized as a
  sum type `T(B) = B + E` (or a richer sum, as for `Option`, tagged
  unions, or nested effects), a morphism out of `T(B)` is *total* if it
  supplies a case for every summand (the universal property of a
  coproduct: `[f, g] : B + E → C` requires both `f : B → C` and
  `g : E → C`) and *partial* if it supplies a case for only one summand,
  leaving the rest undefined (panics, uncaught throws).
- **Algebraic theory / model**: a signature (operation symbols with
  arities) plus a set of equations over terms built from it; a *model*
  assigns an object and operations matching the signature; a model
  *satisfies the theory* only if it additionally satisfies every
  declared equation. (Underlies the optionality axis, REQ-10/REQ-18.)
- **Functor**: a structure-preserving map between categories (preserves
  identity morphisms and composition: `F(f∘g) = F(f)∘F(g)`). Front-end
  plugins are required to be functors into a shared target category
  (C.5).

### C.2 Ubiquitous Requirements

- **REQ-C1**: For each of the four checked axes (`composition`,
  `modularity`, `optionality`, `robustness`), the tool shall define that
  axis's ambient category 𝒜
  and legitimate subcategory 𝒢 as declarative data, not as a hardcoded
  enumeration. REQ-2/REQ-3's effect channels and resolutions, Addendum
  V's visibility lattice, and REQ-18's law suites shall each be
  expressible as one instantiation of `(𝒜, 𝒢)`, never treated as the
  only possible instantiation.
- **REQ-C2**: Given a declared filtration for an axis, the tool shall
  compute `sev(e)` for every finding on that axis and report it
  as `filtration_distance` alongside the finding's existing configured
  severity field (REQ-4), without requiring the filtration to have a fixed
  number of levels. Gating SHALL use configured finding severity, not
  filtration distance, unless the project explicitly declares a mapping
  from filtration distance to severity and that mapping passes configuration
  schema, totality, and determinism validation.
- **REQ-C3**: The tool shall accept a project-declared filtration of
  arbitrary length for the visibility axis. Addendum V's five-level
  table (`L0`–`L4`) shall remain the built-in default filtration when no
  project-specific filtration is declared, but shall not act as a
  ceiling on filtration depth.

### C.3 Event-Driven Requirements

- **REQ-C4** (generalizes REQ-9, REQ-25): When an edge's effect
  resolution is computed, the tool shall determine whether the resolving
  code is a total case analysis over every summand of the effect's
  coproduct structure. Totality SHALL be determined independently of the
  resolution label: `unwrapped` means only that a wrapper was removed.
  `propagated` or `transformed` is total only when every summand remains
  represented, and `unwrapped` is total only when every summand has an
  intentional branch. Panic/force unwrap (`unwrap`, `expect`, `try!`, and
  equivalents) SHALL be treated as partial and classified `swallowed` unless
  every failure/absence summand has an intentional branch. This SHALL apply
  recursively to nested or combined effect
  channels (e.g. `async<result<option<T>>>`), one coproduct layer at a
  time, with no fixed limit on nesting depth.
- **REQ-C5** (generalizes REQ-8, REQ-V3): When an edge is checked
  against the visibility axis, the tool shall test whether a morphism
  from the target's declared scope to the caller's scope exists in that
  axis's legitimate subcategory 𝒢 — built from the declared filtration's
  generators (nesting/ancestor edges plus explicit facade/export edges)
  — independent of how many levels the filtration declares.
- **REQ-C6** (generalizes REQ-10): When an implementation cluster is
  checked against the optionality axis, the tool shall treat the
  declared interface as an algebraic theory and each cluster member as a
  candidate model; a member satisfying the theory's signature but
  failing one of its equations is a finding, regardless of how many
  operations or equations the theory declares.
- **REQ-C7** (generalizes REQ-11): When a redundancy chain is checked,
  the tool shall test whether a colimiting cocone exists over the
  branches' codomain shapes — a common object every branch legitimately
  maps into, via an explicit adapter wherever shapes differ. Absence of
  such a cocone is a finding, independent of the number of branches.

### C.3.1 Operational Semantics for Implementers

The formal terms above SHALL be implemented by the following finite checks;
category terminology does not authorize a different result from these
engineering procedures.

```text
legitimate_shape(edge, G):
  reject shape:opaque from this check per REQ-23
  produced := normalize(edge.codomain); expected := normalize(edge.domain)
  return unify(produced, expected) through only adapters declared in G

visible(target_scope, caller_scope, G):
  graph := nesting/ancestor edges + explicit export/facade edges in G
  return reachable(target_scope, caller_scope, graph)

total_effect(effect, syntax):
  for each wrapper layer outside-in:
    enumerate every value/failure/absence summand
    require an intentional branch, propagation, or transformation per summand
    recurse into each branch's nested wrapper; panic/force unwrap is uncovered
  return total only if every recursive check succeeds

model_satisfies(theory, implementation):
  require every operation with declared arity and shape
  evaluate every equation using its declared property/proof strategy
  return satisfied only if all equations pass; preserve unavailable/timeout

redundancy_compatible(branches, G):
  candidates := common codomain shapes reachable from every branch in G
  require a deterministic candidate and an explicit adapter on each unequal path
  return compatible only if those paths form one common-codomain cocone
```

Thus “factor through 𝒢” means “find a permitted path in the declared graph,”
“model” means “an implementation with all required operations and passing
equations,” and “cocone” means “all fallback outputs reach the same usable
shape through explicit adapters.” Shape legitimacy maps to `composition`,
visibility to `modularity`, model/law checking to `optionality`, and recursive
effect totality and redundancy to `robustness`.

### C.4 Unwanted Behavior Requirements

- **REQ-C8**: If a declared 𝒢 is not closed under composition, or omits
  an identity morphism for some object, the tool shall reject the
  declaration as an invalid legitimate-subcategory definition rather
  than silently using it.
- **REQ-C9**: If a declared filtration's subcategories are not nested
  (some `𝒢ᵢ` is not a subcategory of `𝒢ᵢ₊₁`), the tool shall reject the
  filtration rather than compute an inconsistent severity index.

### C.5 Complex Requirements — Functorial Front-Ends

- **REQ-C10** (generalizes REQ-1, REQ-6): Each front-end plugin shall be
  treated formally as a functor `F : Syntax_lang → CIR`, and its
  conformance-fixture check (REQ-6) shall verify functoriality — `F`
  preserves identity edges and `F(f∘g) = F(f)∘F(g)` on every fixture
  composition — plus naturality against any other loaded plugin sharing
  a fixture (both plugins' images of the same abstract fixture diagram
  must agree). Adding a new language therefore requires only a new
  functor satisfying this check; no change to C.2–C.4 is required.

### C.6 Traceability Notes (Addendum C)

- Master finding definition and severity grading → REQ-C1–C3, REQ-4.
- Robustness effect-totality check as coproduct elimination → REQ-C4, REQ-9,
  REQ-25.
- Modularity axis as scope-category reachability at arbitrary depth →
  REQ-C5, REQ-8, Addendum V.
- Optionality axis as theory/model satisfaction → REQ-C6, REQ-10,
  REQ-18.
- Redundancy robustness check as colimit existence → REQ-C7, REQ-11.
- Malformed axis declarations → REQ-C8–C9.
- Multi-language extension as functoriality → REQ-C10, REQ-1, REQ-6.

---

## Addendum T: Facade Evolution, Retry Idempotency, Resource Linearity

### T.0 Background & Motivation

Of the additional engineering axes considered for this tool suite — facade
evolution, retry idempotency, resource linearity, N+1 query cost,
documentation truth maintenance, and lock-ordering consistency — only the
first three integrate into Vampiro without adding a new extraction
primitive. Each is a specialization of machinery already defined in
Addendum C: facade evolution is a composition break (REQ-C4/REQ-7) taken
across two time-indexed snapshots instead of two call sites; retry
idempotency is an algebraic-theory check (REQ-C6/REQ-10) on the equation
`f;f = f`; resource linearity is REQ-C4's total/partial coproduct
elimination, generalized from effect-channel summands to scope
exit-paths. None of the three require a new kind of graph.

The other three candidates are deliberately excluded here. N+1 query
cost requires loop/control-flow membership, which the CIR (REQ-1) does
not capture — a legitimate future extension, but not a free one.
Documentation truth maintenance is a grounding check against prose
claims, which is `dont`'s stated purpose, not Vampiro's; folding it in
would itself be the kind of scope reach-through Vampiro exists to flag.
Lock-ordering consistency needs a distinct lock-acquisition graph and
per-thread reasoning, better wrapped externally in the pattern `pretender`
uses for mutation testers than absorbed into the CIR.

### T.1 Scope & Additional Definitions

- **Facade snapshot `F_t`**: the `L4` facade (Addendum V) as computed at
  analyzed commit or version `t`. Its default comparison baseline is the
  nearest successfully persisted ancestor snapshot on `t`'s first-parent
  lineage; among retained snapshots, “nearest” means the smallest positive
  first-parent distance, with commit identity as the deterministic tie-break.
- **Breaking edge**: a facade item whose qualified identity persists
  across two snapshots `F_t1`, `F_t2`, but whose domain/codomain shape at
  `t2` does not admit the shape at `t1` to factor through it — a
  composition break (C.1) indexed across time rather than across nodes.
- **Declared migration**: an explicit, project-declared authorization for
  a breaking edge (semver major bump, changelog entry, `@breaking`
  annotation) — a generator added to the legitimate subcategory for that
  edge only, analogous to the explicit adapter in a redundancy chain
  (REQ-11/REQ-C7).
- **Retry obligation**: a `retried`-classified edge (Section 1, REQ-3),
  additionally tagged with an idempotency class — `idempotent`
  (unique-constraint-backed upsert, supplied idempotency key, or a
  safe/idempotent verb), `non-idempotent` (plain insert, or a write verb
  with no dedupe mechanism), or `unknown` — derived from a write-shape
  idiom table using the same mechanism as REQ-3's effect idiom table.
- **Idempotency law**: the equation `f;f = f` — applying a retried
  operation twice under duplication yields the same observable state as
  applying it once — checked as one instance of REQ-C6.
- **Acquire/release obligation**: an edge from an acquire-classified node
  (open, connect, lock, spawn) creating a unique pending obligation with
  multiplicity one, tied to a conservative resource identity (the allocation
  site plus a uniquely resolved handle/alias). It must be discharged by a
  matching release node reachable on every exit path from
  the acquiring scope (normal return, each declared exception/error type,
  early return, panic/abort) — a coproduct elimination per REQ-C4, with
  the coproduct taken over exit paths rather than effect-channel
  summands. A release discharges at most one obligation of matching identity;
  ownership transfer transfers that same obligation rather than satisfying or
  duplicating it.

### T.2 Ubiquitous Requirements

- **REQ-T1**: The tool shall persist the `L4` facade snapshot per
  analyzed commit or version, keyed by qualified item identity, to
  support cross-version comparison. By default it SHALL compare with the
  nearest successfully persisted ancestor snapshot on the analyzed revision's
  first-parent lineage, selected by smallest first-parent distance and then
  commit identity. It SHALL support an explicit baseline override. On the
  first snapshot it SHALL persist the baseline and emit no breaking findings.
  A requested baseline that is missing or is not an ancestor SHALL produce an
  operational/configuration error rather than selecting another baseline.
- **REQ-T2**: The tool shall extend the `retried` effect-resolution
  classification with an idempotency-class field, derived from a
  write-shape idiom table versioned and conformance-tested identically to
  the effect idiom table (REQ-3, REQ-6).
- **REQ-T3**: The tool shall extend node extraction (REQ-1) to record,
  for every acquire-classified node, the full set of exit paths reachable
  from its enclosing scope and, for each, whether a matching release node
  is present. Every acquisition SHALL create a distinct pending obligation
  tied to conservative resource identity. Each release SHALL discharge only
  matching identity and at most one obligation; a duplicate release SHALL
  not discharge another resource's obligation. Ownership transfer SHALL move
  the existing obligation to the new owner. Unresolved aliases or identity
  SHALL emit an `identity:unknown` coverage diagnostic and SHALL not be
  treated as safe.

### T.3 Event-Driven Requirements

- **REQ-T4** (composition axis, historical instance of REQ-7/REQ-C1):
  When a facade item's identity persists across two analyzed snapshots
  with a shape change that is a breaking edge, and no declared migration
  authorizes it, the tool shall raise a `composition` finding classified
  `breaking-change`.
- **REQ-T5** (robustness axis): When a `retried` edge's idempotency class
  is `non-idempotent`, the tool shall raise a `robustness` finding
  classified `unsafe-retry`, naming the write shape and the missing
  idempotency mechanism.
- **REQ-T6** (optionality axis, instance of REQ-C6): When an idempotency
  law is declared for a retried operation's interface and property
  testing or `--prove` is enabled for that interface, the tool shall
  check `f;f = f` the same way REQ-10 checks any declared law, and raise
  an `optionality`-axis finding — distinct from, and cross-referenced
  with, any REQ-T5 finding on the same edge — if the law fails.
- **REQ-T7** (robustness axis, instance of REQ-C4): When an acquire
  node's set of exit paths does not admit a total release — at least one
  exit path has no reachable matching-identity release node, including due
  to identity mismatch or insufficient release multiplicity — the tool shall
  raise a `robustness` finding classified `resource-leak`, naming the
  acquisition, conservative resource identity, and unreleased exit path.

### T.4 Unwanted Behavior Requirements

- **REQ-T8**: If a facade item's identity cannot be matched deterministically
  across two snapshots (renamed or moved without a declared alias), the
  tool shall report `identity:ambiguous` rather than silently classify it
  as independently added and removed.
- **REQ-T9**: If a write-shape idiom has no entry in the idempotency
  idiom table, the tool shall classify it `unknown`; `unknown` shall not
  itself raise a REQ-T5 finding by default, but shall be surfaced as an
  idiom-table coverage gap, mirroring REQ-21.

### T.5 Traceability Notes (Addendum T)

- Facade evolution → REQ-T1, REQ-T4, REQ-T8; reuses the Addendum V facade
  definition and the C.1 finding definition.
- Retry idempotency → REQ-T2, REQ-T5, REQ-T6, REQ-T9; reuses the Section
  1 effect-resolution classification and the REQ-C6 algebraic-theory
  machinery.
- Resource linearity → REQ-T3, REQ-T7; reuses REQ-C4's total/partial
  coproduct-elimination machinery, generalized from effect channels to
  exit-path coproducts.
- Deliberately excluded (belong elsewhere or need new extraction): N+1
  query cost, documentation truth maintenance, lock-ordering consistency
  — see T.0.

---

## Appendix: Worked Examples by Axis

**Composition break (Python → seam)**
```python
def parse_amount(raw: str) -> Decimal | None: ...
def apply_discount(amount: Decimal, pct: float) -> Decimal: ...

total = apply_discount(parse_amount(raw), 0.1)
```
CIR: `parse_amount` codomain shape = `union<Decimal, None>`.
`apply_discount` domain shape (param 1) = `Decimal`. Shapes do not unify
because the union's `None` arm is unhandled at the call site → REQ-7
finding, axis `composition`.

**Modularity break (TypeScript)**
```ts
// billing/index.ts exports: { charge }
import { _internalLedgerCache } from "billing/internal/ledger";
```
`_internalLedgerCache` is not in `billing`'s declared exported interface →
REQ-8 finding, axis `modularity`.

**Optionality break (Rust)**
```rust
trait Merge { fn merge(self, other: Self) -> Self; }
impl Merge for RunningTotal { fn merge(self, o: Self) -> Self { ... } } // associative & commutative
impl Merge for LatestWins   { fn merge(self, o: Self) -> Self { o } }   // not commutative
```
Both implement `Merge`, forming one implementation cluster. A declared law
suite for `Merge` includes commutativity. `RunningTotal` passes;
`LatestWins` fails → REQ-10 finding, axis `optionality`, naming the failed
law and the non-conforming member.

**Arbitrary-depth filtration (Rust, workspace-of-workspaces)**

Addendum V's built-in table stops at `L4`, but a project can declare a
longer filtration without changing any rule. A Cargo workspace nested
inside a larger workspace has two extra generators beyond the
crate-local ones:

```
𝒢₀ (L0 private)      ⊆
𝒢₁ (L1 module)       ⊆
𝒢₂ (L2 crate)        ⊆
𝒢₃ (L2.5 workspace)  — pub item, but only re-exported within this
                        workspace's own facade crate, not the outer one
𝒢₄ (L3 public-unstable) ⊆
𝒢₅ (L4 public-stable, outer-workspace facade) = 𝒜
```

An item that is `pub` (compiles as accessible, so it is in `𝒜`) but only
re-exported by the inner workspace's facade crate is in `𝒢₃` but not
`𝒢₅`. An edge from a crate in the *outer* workspace directly into that
item has `sev(e) = 5` and output `filtration_distance: 5`: it factors through 𝒜 (the compiler allows it —
`pub` is `pub`) but not through any `𝒢ᵢ` below the outermost level. This
is exactly REQ-V4's `over-exposure` classification, now accompanied by
independent filtration-distance evidence, with no change to configured
finding severity or REQ-C5's checking logic —
only the filtration declaration grew by one level.

**Nested-coproduct elimination (Rust, `async<result<option<T>>>`)**

```rust
async fn fetch_nickname(id: UserId) -> Result<Option<String>, DbError> {
    ...
}

async fn greet(id: UserId) {
    let name = fetch_nickname(id).await.unwrap().unwrap();
    println!("hi {name}");
}
```

The effect channel is `async<result<option<String>>>` — three nested
coproduct layers. `.await` resolves the outer `async` layer legitimately.
The first `.unwrap()` eliminates `Result<Option<String>, DbError>` using
only the `Ok` case — partial, not total, over that layer's coproduct: a
`DbError` panics instead of being handled. The second `.unwrap()`
eliminates `Option<String>` using only the `Some` case — partial again.
Per REQ-C4, the tool recurses one coproduct layer at a time and raises
two separate `swallowed` findings, one per layer, rather than a single
finding for the whole chain — each layer's totality is independently
checkable and independently violated.

**Robustness break — direct (Go)**
```go
result, _ := riskyLookup(id)
use(result)
```
`riskyLookup` codomain effect channel = `result` (Go idiom: named `error`
return). The `_` discard matches the idiom-table entry for `swallowed` →
REQ-9 finding, axis `robustness`.

**Over-exposure (Rust)**
```rust
// crate root: pub use pricing::{Tier};   <- declared facade
pub mod pricing {
    pub struct Tier { ... }
    pub fn _raw_discount_curve() -> Vec<f64> { ... }  // pub, but not in facade
}
```
`_raw_discount_curve` is `enforced-open` (compiles as accessible from any
downstream crate) but is absent from the crate root's facade and carries
the leading-underscore advisory marker → REQ-V4 finding, axis
`modularity`, classification `over-exposure`.

**Type piracy (Julia)**
```julia
# inside package MyPlots, extending Base.iterate for LinearAlgebra.Diagonal
function Base.iterate(d::LinearAlgebra.Diagonal, state=1)
    ...
end
```
`Base.iterate` is not defined in `MyPlots`; `LinearAlgebra.Diagonal` is
not defined in `MyPlots` either → REQ-V5 finding, axis `modularity`,
classification `type-piracy`.

**Var-deref reach-through (Clojure)**
```clojure
(ns my.app.reporting
  (:require [my.app.billing :as billing]))

(defn total [] (@#'billing/raw-ledger-sum))
```
`raw-ledger-sum` is `defn-` in `my.app.billing`; `reporting` reaches it
via `#'billing/raw-ledger-sum` var dereference from outside its defining
namespace → REQ-V6 finding, axis `modularity`, classification
`reach-through`.

**Facade leak (Python)**
```python
# pricing/internal/tables.py
class RawTierTable: ...

# pricing/__init__.py  (the package's declared facade)
from .internal.tables import RawTierTable
```
`RawTierTable` lives under `pricing/internal/`, an `L2 package-internal`
path by convention, but the facade re-exports it to `L4 public-stable` →
REQ-V7 finding, axis `modularity`, classification `facade-leak`.

**Robustness break — redundancy chain (Python)**
```python
try:
    data = primary_source.fetch(id)      # -> FullRecord
except SourceUnavailable:
    data = cache.get(id)                 # -> PartialRecord | None
use(data)
```
Primary branch codomain shape `FullRecord`; fallback branch codomain shape
`union<PartialRecord, None>`. No adapter node reconciles the two shapes
before `use(data)` → REQ-11 finding, axis `robustness`, even though this
code was written specifically to *add* resilience.

### End-to-End Decision Examples

Essential finding fields below are `rule_id`, `axis`, `file`, `line_range`,
`severity`, and rule-specific evidence. Diagnostics instead carry
`diagnostic`, location/context, and remediation and have no `axis`.

**Formal proof — available and unavailable.** A Lean proof of associativity
returns `Proved`: REQ-12 emits status `Proved` and no finding. The same
obligation with a counterexample returns `Disproved`: if REQ-26 combines it
with a failed property test, the `optionality` finding contains the essential
fields plus `law`, `property_result`, and `proof_status: Disproved`. An
unreachable Lean service produces `{diagnostic: ProverUnavailable, target,
backend, guidance}` under REQ-16, no axis, and never `Proved` or `Disproved`.

**Category and filtration declarations.** A category containing every
identity and closed composite, with nested `G0 ⊆ G1 ⊆ A`, is accepted by
REQ-C8/REQ-C9. Omitting `id(Customer)` produces
`{diagnostic: invalid-category, object: Customer, reason: missing-identity}`
under REQ-C8. Declaring `G1` without a morphism present in `G0` produces
`{diagnostic: invalid-filtration, lower: G0, upper: G1, witness}` under
REQ-C9. These configuration diagnostics have no finding axis.

**Facade evolution.** At revision `a`, no ancestor snapshot exists: REQ-T1
persists `F_a` and emits no breaking finding. At first-parent child `b`,
`charge(Money)->Receipt` changes to `charge(Int)->Receipt`; REQ-T4 emits
`{rule_id: REQ-T4, axis: composition, file, line_range, severity,
classification: breaking-change, baseline: a, old_shape, new_shape}`. A
declared compatible adapter emits no finding. A missing explicit override
produces an operational error with the requested baseline and remediation,
not a fallback selection.

**Retries.** A retrying PUT with a stable idempotency key is `idempotent` and
produces no REQ-T5 finding. A retrying plain INSERT is `non-idempotent` and
emits `{rule_id: REQ-T5, axis: robustness, file, line_range, severity,
classification: unsafe-retry, write_shape, missing_mechanism}`. An unresolved
custom write idiom emits `{diagnostic: idempotency-coverage-unknown, edge,
idiom_table_version}` under REQ-T9, no axis, and is not treated safe.

**Resource identity and multiplicity.** `a=open(); close(a)` on every exit is
safe. For `a=open(); b=open(); close(b); close(b)`, the second release cannot
discharge `a`; REQ-T7 emits `{rule_id: REQ-T7, axis: robustness, file,
line_range, severity, classification: resource-leak, resource_identity: a,
exit_path}`. Likewise, `close(b)` cannot discharge an obligation for `a`
(identity mismatch). `a=open(); owner=move(a); close(owner)` is safe because
REQ-T3 transfers the obligation. If alias analysis cannot prove `x` is `a`,
it emits `{diagnostic: identity:unknown, acquisition, alias: x, exit_path}`
with no axis and does not mark the path safe.
