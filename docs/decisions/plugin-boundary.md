# Plugin Boundary

> Approved decisions for the vampiro CIR plugin boundary: packaging, ABI, serialization, and compatibility.

**Approver:** charly vibes
**Review reference:** bd issue vampiro-0vb.2.1 — Plugin boundary decision gate
**Date:** 2026-07-28

---

## 1. Plugin ABI and packaging

**Decision:** Workspace-crate ABI only. Frontends live as workspace crates under `crates/vampiro-*-frontend/` and implement `vampiro-cir` traits as compile-time dependencies.

**Rationale:**
- Only one frontend (Rust) is planned in the current scope (P0).
- A second frontend is a P2 epic; no third-party plugins are anticipated.
- Workspace-crate linking is zero-cost, requires no serialization, no dynamic loading, and no ABI stabilization.
- Adding a dynamic plugin boundary later is a purely additive change — the CIR traits remain the same, only the transport layer changes.

**Trigger to revisit:** A second frontend enters active development (P1+) OR a third-party contributor requests a plugin API.

---

## 2. Serialization

**Decision:** No serialization. CIR types are in-process Rust structs/enums. Serialization is deferred until a cross-language or cross-process boundary exists.

**Rationale:**
- Within a single workspace, serialization is pure overhead with no benefit.
- Choosing a format now (Protobuf, FlatBuffers, etc.) with no consumers would be pure speculation.
- When a second frontend appears, the choice will be informed by real requirements from both consumers.

**Trigger to revisit:** A frontend in a different language enters development OR a cross-process boundary is needed.

---

## 3. Versioning

**Decision:** CIR types version with the `vampiro-cir` crate SemVer. No separate schema version.

**Rationale:**
- Single-crate, single-repo: crate version IS the schema version.
- Workspace dependencies are pinned by path; no version negotiation needed.
- When a serialization boundary is added, a schema version will be introduced then.

**Trigger to revisit:** Serialization is introduced (see above).

---

## 4. Compatibility window and migration policy

**Decision:** N/A — no serialization boundary exists. When a boundary is added, the compatibility window and migration policy will be defined at that time.

---

## 5. Resource limits

**Decision:** N/A — no dynamic loading, no runtime boundary. Resource limits for declaration validation (max closure size, etc.) are implementation constants, not plugin contract items.

---

## 6. Rejected alternatives

| Alternative | Reason |
|-------------|--------|
| Dynamic library ABI (`libloading`) | ABI stability is brittle in Rust; no benefit with a single frontend in the same workspace |
| Subprocess ABI (IPC) | Adds serialization, lifecycle, and error-handling complexity for zero benefit |
| WASM plugin runtime | Runtime dependency; sandboxing not needed for first-party code |
| Protobuf / FlatBuffers / Cap'n Proto / MessagePack | Pure speculation without a cross-language consumer |
| Custom binary format | Re-inventing wheels; no consumer to justify the design effort |
| Compiled-in monolithic frontend | Only one frontend exists, so this is indistinguishable from workspace-crate ABI — but the workspace-crate approach is cleaner for future extraction |

---

## 7. Scope and compatibility

- **Supported scope:** All scenarios listed above.
- **Immutability:** This decision is valid until the trigger condition is met. When revisited, the new decision record SHALL supersede this one and SHALL preserve the rejected alternatives and rationale.