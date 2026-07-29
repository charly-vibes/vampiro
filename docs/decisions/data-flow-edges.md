# Data-Flow Edge Representation

> Approved decision for how vampiro represents per-slot argument binding in CIR,
> enabling the composition analyzer to compare argument shapes against callee
> parameter slots (the `parse_amount → apply_discount` seam).

**Approver:** charly vibes
**Review reference:** bd issue vampiro-bhf — Approve data-flow edge representation
**Date:** 2026-07-29
**Status:** Approved

---

## 1. Representation

**Decision:** Add an optional `slot` field to the existing `CirEdge` struct
(Option A). A separate `DataFlowEdge` type is not justified at this time.

**Rationale:**
- **Cost/benefit:** Neither downstream tool (crua, livin) requires vampiro to
  emit data-flow edges — they consume shape data, not edge structure. A
  separate type would add ~30 lines of new CIR code, a second vector on
  `CirGraph`, dual-collection validation, and fixture churn across 50+ files,
  for zero external consumer benefit.
- **Forward-compatible migration:** If data-flow edges ever become a first-class
  concept (e.g., livin consuming vampiro's call graph to skip re-parsing), the
  `slot` field can be extracted into a separate `DataFlowEdge` type as a
  backwards-compatible refactor: `#[serde(flatten)]` on `CirEdge` or a new
  `data_flow_edges` vec alongside the existing `edges` vec. Existing graphs
  without the field remain valid.
- **Analyzer complexity:** A single `if let Some(slot)` branch in the
  composition analyzer's one-edge loop is simpler than iterating two
  collections and joining them.

### Specification

```rust
/// Per-slot argument binding: which parameter slot of the callee
/// receives the caller's value at this call site.
///
/// `None` means the slot is unknown or the edge represents only
/// control flow (backward-compatible default). `Some(n)` means the
/// value produced by the source node flows into callee parameter
/// index `n` (0-based).
///
/// Frontends SHOULD set this field when the argument position can
/// be determined statically. Non-frontend callers (fixtures, tests)
/// MAY set it to `None` when slot information is irrelevant.
#[serde(skip_serializing_if = "Option::is_none")]
pub slot: Option<u32>,
```

## 2. Schema version

**Decision:** Bump `CirGraph.version` from `"0.1.0"` to `"0.2.0"`.

**Rationale:** Adding a semantically meaningful field to `CirEdge` changes the
wire format enough that consumers comparing graphs across schema versions can
detect incompatibility. `0.2.0` indicates a backwards-compatible addition
(serialized `0.1.0` graphs deserialize with `slot: None`; serialized `0.2.0`
graphs with `slot` set are rejected by a `0.1.0` consumer due to the unknown
field, which serde ignores by default — bumping the version string at least
provides an explicit signal for version-checking code.)

## 3. Frontend behavior

**Decision:** Each frontend tracks argument position at call sites and emits
one `CirEdge` per argument with the correct slot value.

- A call site `foo(a, b)` produces two edges: one with `slot: Some(0)`, one
  with `slot: Some(1)`.
- A call site `foo(a)` where `foo` is variadic or the arity cannot be
  determined produces one edge with `slot: Some(0)` for the known positional
  args and no edge for variadic trailing args.
- Method calls `receiver.method(arg)` produce one edge with `slot: Some(0)`
  for the receiver (self parameter) and one per additional argument slot.

## 4. Analyzer behavior

**Decision:** The composition analyzer runs two independent checks:

1. **Return-boundary check** (unchanged): compare `callee.codomain` vs
   `caller.codomain`. Catches the case where a caller's return type claims X
   but a callee produces Y ≠ X.
2. **Slot-boundary check** (new): for edges with `slot: Some(n)`, retrieve
   `callee.domain`. If it has a tuple-like shape with at least `n+1` arms,
   compare `caller.codomain` (the value flowing in) against
   `callee.domain[n]` (what the callee expects at that slot). On mismatch,
   emit a composition-break finding with slot information in the evidence.

Both checks use the existing `unify_shapes` primitive. The slot-boundary check
is what enables precise `parse_amount → apply_discount` seam detection.

## 5. Rejected alternatives

### Option B: Separate `DataFlowEdge` type

A new struct and a second vector on `CirGraph`.

```rust
pub struct DataFlowEdge {
    pub id: StableId,
    pub source_value: StableId,
    pub consumer: StableId,
    pub slot: u32,
    pub span: SourceSpan,
}
```

**Rejected because:** No downstream consumer requires it; adds ~30 lines of CIR
code + dual-collection validation + 50+ fixture file changes; the forward-
compatible migration path from Option A to Option B is cheap and backwards-
compatible if the need arises.

### Option C: Data-flow edges as first-class CIR edges with a separate variant

Use an enum `CirEdgeKind { Call { slot: Option<u32> }, DataFlow { ... } }`.

**Rejected because:** Adds type complexity without benefit — the `Call` variant
still needs the optional slot, and no other edge kind exists yet.