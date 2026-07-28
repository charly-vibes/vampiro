# Structural Shape Canonicalization

> Approved decisions for how vampiro canonicalizes CIR shapes for composition
> unification, finding deduplication, and cross-version fixture comparison,
> together with the hashing and collision-handling policy.

**Approver:** charly vibes *(pending sign-off)*
**Review reference:** bd issue vampiro-0vb.4.1 — Structural-shape canonicalization decision gate
**Date:** 2026-07-28
**Status:** DRAFT — awaiting human approval (HITL). Do not begin analysis
implementation (tasks 1–5 of `add-core-seam-analysis`) until this section is
marked **Approved** below.

---

## 1. Normalization form

**Decision:** Normalize shapes **in place on the existing `Shape` enum**
(no separate canonical IR). A pure function
`Shape::normalize(&self) -> Shape` returns a canonical representation by:

- **Union arms:** sort the arms of every `Union` by a stable structural key
  (`(variant_tag, normalized_serialization)`) so `union<A,B>` and `union<B,A>`
  compare equal. Union arms are treated as an *unordered set* for shape
  purposes, because shapes are deliberately coarser than full types
  (EARS §1, "Domain shape / codomain shape"). Positional sum types whose
  arm order is semantically load-bearing (e.g. `Result<Ok, Err>`) are
  modeled as `Parameterized { base: "Result", parameters: [Ok, Err] }` by
  frontends, so their arm order is preserved through `Parameterized` and
  is not normalized away.
- **Record fields:** sort by field name to make record shapes
  field-order-independent, since records are structural products.
- **Opaque sentinel:** `Opaque` short-circuits composition comparison. A
  shape that is `Opaque` at its top level, or whose only extraction evidence
  is `Opaque`, is excluded from composition-break checking per REQ-23 and
  never reported as a composition finding; the same edge remains eligible for
  modularity and robustness checks. `Opaque` inside a non-opaque compound
  (e.g. `Record<..., Opaque, ...>`) degrades only that arm to opaque and the
  shape is reported with the opaque arm marked, not silently promoted to
  fully opaque.
- **Bottom:** `Bottom` (the `!`/never shape) is preserved as a distinct
  leaf; it unifies only with itself. A divergence-reachable continuation
  past a `Bottom`-returning call is not a composition break.
- **Depth bound:** normalization respects the existing `MAX_SHAPE_DEPTH`
  (`crates/vampiro-cir/src/shape.rs`, currently 64); a shape exceeding it is
  treated as `Opaque` for composition and flagged as an extraction diagnostic,
  never silently accepted.

**Rationale:**
- The `Shape` enum already exists and is the extraction surface; a second
  canonical IR would duplicate the data model for no consumer (YAGNI),
  matching the no-serialization / workspace-crate-only stance of the plugin
  boundary decision.
- Sort-based normalization is fully deterministic, content-sensitive, and
  cheap to verify in property tests.
- The unordered-union rule is the one place this decision deliberately
  loses information; it is justified by the EARS definition of shapes as
  coarse structural signatures, and is recoverable: any frontend that
  needs arm order to matter emits `Parameterized` instead of `Union`.

**Trigger to revisit:** A real composition-break case is found that depends
on union arm order and cannot be expressed as `Parameterized`; OR a
serialization/cross-process boundary is introduced (see plugin-boundary
decision §2).

---

## 2. Hashing

**Decision:** The shape-hash component of the REQ-24 dedupe identity is
`SHA256(canonical_json(normalize(shape)))` truncated to 128 bits (16 bytes),
hex-encoded — the same scheme already used for `StableId`
(`crates/vampiro-cir/src/provenance.rs`).

- `canonical_json` is a fixed, deterministic serialization: keys sorted,
  no whitespace, UTF-8, variants in the serde `kebab-case` tag order
  already defined for `Shape`.
- The full finding dedupe identity (REQ-24) is then
  `SHA256(rule_id + ":" + location + ":" + shape_hash)` over the
  normalized inputs, so two findings on the same edge with the same rule
  and location but different shapes still dedupe correctly only when the
  shapes are genuinely equal.

**Rationale:** Reusing the existing, documented `StableId` scheme keeps one
hashing discipline in the codebase, is byte-for-byte reproducible (REQ-29),
and is content-sensitive (REQ-24).

**Trigger to revisit:** A 128-bit collision is observed in practice
(treated as effectively impossible); OR a serialization boundary requires a
stable cross-version wire hash distinct from the in-memory canonicalization.

---

## 3. Collision handling

**Decision:** 128-bit truncated SHA256 is the primary comparison key. For
dedupe (REQ-24) the identity already includes `rule_id` and `location`, so a
shape-hash collision alone cannot merge two distinct findings. For
fixture comparison (REQ-29) and unification (REQ-7), hash equality is treated
as a *candidate* match and is **confirmed by a structural re-comparison** of
the normalized shapes before any finding is suppressed or fixture accepted.

- A confirmed structural match on hash equality is the dedupe/accept path.
- A structural mismatch on hash equality (a true collision) is reported as
  an `identity:hash-collision` diagnostic with both witnesses and never
  silently treated as equal.

**Rationale:** Never convert a hash collision into a false dedupe or a false
fixture pass; the spec's standing rule is "never convert uncertainty into
validity" (EARS Addendum C, Risks/Trade-offs; REQ-23/REQ-B6).

**Trigger to revisit:** A true collision is observed; the diagnostic is
promoted to a finding or the hash width is raised.

---

## 4. Compatibility and version policy

**Decision:** Shape canonicalization is an **internal contract of the
`vampiro-cir` crate**, versioned with the crate SemVer — consistent with the
plugin-boundary decision (no separate schema version while no serialization
boundary exists).

- The canonical form is pinned by the CIR schema version, which is already
  part of the incremental-cache invalidation key (REQ-28: "source content
  plus analyzer, CIR schema, plugin, and effective configuration versions").
- Any change to normalization rules, `canonical_json`, or the union-arm sort
  key **MUST** bump the CIR schema version, which invalidates incremental
  cache and triggers conformance-fixture regeneration (REQ-6, REQ-29).
- Conformance fixtures are versioned artifacts: a normalization change
  requires regenerating fixtures and recording the new schema version in
  the fixture manifest; an unchanged fixture against a changed schema is a
  test failure, not a silent pass.
- Exclusions: this decision does **not** constrain effect-channel
  combination canonicalization (EARS §1, REQ-2, v1.3.0 grammar), which is
  governed by its own requirement and the effect-id vocabulary; shape
  canonicalization only covers the `Shape` domain/codomain structures.

**Rationale:** One version key (the crate / CIR schema version) drives cache
and fixture invalidation uniformly; no separate canonicalization version is
warranted while everything stays in-process.

**Trigger to revisit:** A serialization/cross-process boundary is added
(plugin-boundary §2), at which point the canonical form becomes the wire
format and receives its own explicit schema version.

---

## 5. Alternatives considered (rejected)

- **B. Separate `CanonicalShape` IR.** Normalize `Shape` into a distinct
  canonical data structure used by all consumers.
  *Rejected:* no consumer currently needs a representation different from
  the extraction shape; the duplication would be pure speculation and would
  create a second source of truth for shape data. Revisit only if a consumer
  needs a canonical form that diverges structurally from the extraction
  shape (e.g., hash-consed sharing for very large graphs).

- **C. Pure content-hash comparison (no structural re-check).** Compare
  shapes solely by their 128-bit hash.
  *Rejected:* REQ-7 requires side-by-side caller-expected vs.
  callee-produced shape evidence in the finding, which a bare hash cannot
  provide; and REQ-29 fixture failures would lose the structural witness.
  Hashes are used as a fast candidate key (§3) but never as the sole
  comparator.

- **D. Order-preserving union normalization (do not sort union arms).**
  Preserve insertion order in `Union` and compare order-sensitively.
  *Rejected:* contradicts the EARS definition of shapes as coarse
  structural signatures and would raise spurious composition findings
  whenever two frontends emit the same sum type in different arm orders.
  Order-sensitivity is recoverable through `Parameterized` (§1) where it
  is genuinely needed.

- **E. Promote any shape containing `Opaque` to fully opaque.** Treat a
  compound shape with one opaque arm as opaque for all checks.
  *Rejected:* over-broad; REQ-23 excludes opaque only from composition-break
  checking, and modularity/robustness checks must still run on the
  non-opaque arms. Degrading only the opaque arm (§1) preserves those checks.

- **F. Separate canonicalization schema version (independent of CIR
  schema version).** Version the normalization rules on their own track.
  *Rejected:* adds a version dimension with no current consumer; the CIR
  schema version already participates in cache (REQ-28) and fixture
  (REQ-29) invalidation and is sufficient. Revisit when a serialization
  boundary introduces a wire format.

---

## 6. Scope and exclusions

- **In scope:** domain/codomain `Shape` normalization, shape hashing,
  dedupe identity composition, collision handling, and the
  schema-version coupling for cache/fixture invalidation.
- **Out of scope:** effect-channel combination canonicalization (REQ-2),
  finding severity defaults (REQ-4, v1.3.0 table), filtration-distance
  computation (REQ-C2), and the structural-unification algorithm itself
  (task 1.2). This decision defines the *canonical form* the unifier
  consumes; the unifier is implemented and tested under task 1.

---

## Approval

- [ ] Human approver (charly vibes) signs off on this decision record.
- [ ] Update `openspec/changes/add-core-seam-analysis/tasks.md` checkboxes
      0.1 and 0.2.
- [ ] `openspec validate add-core-seam-analysis --strict` remains passing.

**Approver signature:** ____________________  **Date:** __________
