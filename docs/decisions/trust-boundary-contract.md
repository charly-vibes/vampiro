# Trust-Boundary Contract Decision

> Trust-provenance classification, smart-constructor recognition, boundary-leak and validation-duplication findings, refinement-confirmation evidence schema, and configuration/idiom scope for Vampiro's trust-boundary analysis.

**Approver:** charly vibes
**Review reference:** bd issue vampiro-0vb.10.1 — Approve the trust and evidence contracts
**Date:** 2026-07-28
**Review commit:** `3216348`

---

## 1. Trust-Provenance Representation

### Alternatives considered

| Approach | Description | Decision |
|----------|-------------|----------|
| **Three-value enum** (`untrusted`/`trusted`/`unknown`) | Attach to every CIR value occurrence (node parameter/output, sum arm, edge argument slot). Separate from argument provenance. | ✅ Selected |
| Boolean flag (`trusted`/`untrusted`, default trusted) | Binary classification; no way to represent missing or ambiguous provenance. | ❌ Rejected — `unknown` is needed to avoid defaulting to trusted when evidence is absent. |
| Reuse argument-provenance chain | Extend `H` bound to cover trust sources; conflates data-flow hops with external-source trust. | ❌ Rejected — would make `unknown` behavior impossible to interpret and mix two different semantic axes. |

**Decision:** Three-value `untrusted`/`trusted`/`unknown` attached to CIR value occurrences, fully independent from argument-provenance chains.

**Rationale:** Only a three-value representation can distinguish "proven trusted" from "no evidence either way." Conflating trust with argument provenance would create an unrecoverable semantic ambiguity for evidence import and boundary-leak analysis.

---

## 2. Trust-Boundary Source and Smart-Constructor Recognition

### Alternatives considered

| Approach | Description | Decision |
|----------|-------------|----------|
| **Versioned conformance idioms + explicit project declarations** | Frontends ship tested idiom matchers; users can supplement or override via config. | ✅ Selected |
| Pure idiom matching (no config) | No configuration surface; all recognition happens through built-in frontend knowledge. | ❌ Rejected — users cannot declare custom smart constructors or boundary sources outside the tested set. |
| Pure declarations (no idioms) | Users must hand-declare every source and constructor. | ❌ Rejected — onerous for common patterns; defeats the value of a Rust-aware frontend. |
| Machine learning / heuristic inference | Unsound by design; not reproducible across tool versions. | ❌ Rejected — Vampiro requirements demand deterministic, versioned, auditable analysis. |

**Decision:** Frontends recognize trust-boundary sources and smart constructors through versioned, conformance-tested idioms (e.g., `std::io::Read::read`, `serde_json::from_str`, `Result`-returning constructors with refinement) OR through explicit project declarations in `.vampiro/config.toml`. Declaration takes precedence over idiom when both match.

---

## 3. Propagation and Conflict Rules

### Alternatives considered

| Aspect | Options | Decision |
|--------|---------|----------|
| **Join order** | `untrusted > unknown > trusted` vs. `untrusted > trusted > unknown` vs. lattice meet | `untrusted > unknown > trusted` — any untrusted contributor makes the result untrusted; else any unknown contributor makes it unknown; else trusted. |
| **Over-`H` behavior** | Trusted vs. unknown | `unknown` — exceeding the argument-provenance bound `H` (default 32) means the dependency graph is too deep to classify confidently. |
| **Declaration/idiom conflict** | Trusted vs. unknown | `unknown` — disagreement between a project declaration and a conformance-tested idiom is a signal that the configuration may be stale or misapplied. |
| **Unmatched pattern** | Trusted vs. unknown | `unknown` + `trust-provenance:unknown` diagnostic — never default to trusted when no idiom or declaration applies. |

**Decision:** The conservative order `untrusted > unknown > trusted` governs propagation. Any over-`H`, conflict, or unmatched case yields `unknown` and emits a `trust-provenance:unknown` diagnostic (no finding axis).

---

## 4. Internal Literal and Constructor-Arm Classification

| Pattern | Trust classification |
|---------|---------------------|
| Recognized internal literal (e.g., constant `0`, `""`, `false`) | `trusted` — originates inside the trust domain |
| Smart-constructor success-arm value | Follows contributor rule (if any contributor is untrusted → untrusted; else unknown; else trusted) |
| Smart-constructor non-success-arm payload | `trusted` — the error/None variant is an internal construct |
| Ordinary derived occurrence | Follows the `untrusted > unknown > trusted` contributor order |

---

## 5. Default Finding Severities

| Finding | Axis | Default Severity | Rationale |
|---------|------|------------------|-----------|
| `boundary-leak` | robustness | `HIGH` | Untrusted data reaching interior code without refinement is a security-relevant concern. |
| `validation-duplication` | modularity | `LOW` | Duplicate validation is a code-quality concern; it is not a soundness issue by itself. |
| `refinement_confirmation` | (diagnostic status, not a finding) | — | Reported as `confirmed` or `unknown` with a closed reason vocabulary; never emitted as a finding. |

---

## 6. Validation-Equivalence and Duplication Detection

### Alternatives considered

| Approach | Description | Decision |
|----------|-------------|----------|
| **Stable validation identity + conformance-tested idiom** | Frontends assign a validation identity (declared or idiom-derived); duplication fires when the same identity appears outside the smart constructor body. | ✅ Selected |
| Syntactic similarity only | AST heuristics for "looks like the same check." | ❌ Rejected — unsound; false positives from coincidental similarity. |
| Data-flow equivalence | Trace whether the same value is checked twice. | ❌ Rejected — over-engineered for v1; cannot distinguish "same value, different constraint" from "same constraint, different value." |

**Decision:** Validation equivalence requires a shared stable validation identity (from project declaration or conformance-tested idiom). Mere syntactic similarity without identity evidence produces no finding.

---

## 7. Boundary-Classification and Evidence Schema

### Evidence schema shape (versioned, JSON)

```json
{
  "schema_version": "v0.1.0",
  "evidence": {
    "producer": { "name": "<companion-tool-name>", "version": "<semver>" },
    "analyzed_revision": "<git-commit-sha>",
    "constructor": {
      "stable_identity": "<id>",
      "source_hash": "<sha256>",
      "shape_hash": "<sha256>"
    },
    "boundary_classes": [
      {
        "id": "<class-id>",
        "status": "passing" | "failing",
        "details": "<optional-text>"
      }
    ]
  }
}
```

### Alternatives considered

| Aspect | Options | Decision |
|--------|---------|----------|
| **Evidence transport** | Versioned JSON schema with file references in config | ✅ Selected — explicit, auditable, diffable. |
| | Inline in config.toml | ❌ Rejected — config bloat; evidence is external and versioned independently. |
| | Binary protocol (protobuf/flatbuffers) | ❌ Rejected — over-engineered for v1; JSON is human-readable and trivially diffable. |
| **Compatibility policy** | Minor schema version → backward-compatible; major → require migration | ✅ Selected — `schema_version` minor bump allows additive fields; major bump requires explicit re-import. |
| | Any version mismatch → `unknown` | ❌ Too strict — minor schema additions should not break existing evidence. |

---

## 8. Refinement-Confirmation Vocabulary

Ordered closed vocabulary for `refinement_confirmation.status=unknown`:

| Reason | When applicable |
|--------|-----------------|
| `absent` | No evidence file is found at the configured path |
| `malformed` | Evidence file exists but cannot be parsed |
| `unsupported-version` | Evidence schema version is not recognized |
| `stale` | `analyzed_revision` does not match the current analyzed commit |
| `mismatched` | Constructor `stable_identity`, `source_hash`, or `shape_hash` does not match |
| `empty-classes` | `boundary_classes` is empty |
| `duplicate-class` | A boundary-class ID appears more than once |
| `incomplete` | Not every declared boundary class appears in `boundary_classes` |
| `unknown-class` | An evidence boundary-class ID has no matching project declaration |
| `failing` | Any boundary class has `status=failing` |

Only one reason applies — the first in this ordered list that matches. A downstream consumer MUST NOT derive `unreachable` unless status is `confirmed`.

---

## 9. Configuration Scope

Initial `.vampiro/config.toml` sections for trust-boundary analysis:

```toml
[trust]
# Optional: override argument-provenance bound H (default 32)
provenance-bound = 32

# Declare external trust-boundary sources (function paths)
sources = [
  "std::io::Read::read",
  "std::io::Read::read_to_string",
]

# Declare smart constructors (function or method)
# identity: stable validation identity for duplicate detection
[[trust.constructors]]
path = "myapp::validate_user"
refined_shape = "myapp::User"
identity = "validate_user"

# Declare boundary classes for evidence import
[[trust.boundary-classes]]
id = "req-body-parsed"
description = "Request body is valid JSON and schema-conformant"

# Optional evidence import source
[trust.evidence]
path = ".vampiro/evidence.json"
```

Idiom-based recognition (no config needed) for:
- `std::io::Read::read` and `read_to_string` → boundary source
- `serde_json::from_str`, `serde_json::from_reader` → smart constructor (refined shape from type inference)
- `Result<_, E>`-returning functions where an arm calls `Ok( refined_type { .. } )` → smart constructor
- `Option<T>`-returning functions where `Some` arm constructs `T` from validated fields → smart constructor
- Constant literals `0`, `""`, `false`, `[]` → internal trusted literal

---

## 10. Rejected Alternatives Summary

| Alternative | Rejected because |
|------------|------------------|
| Boolean trust flag | Cannot represent unknown/missing evidence; silent trust default is unsound. |
| Reuse argument-provenance chain | Conflates two independent semantic axes; makes evidence import impossible to interpret. |
| Pure idiom matching | No user override for project-specific constructors or sources. |
| Pure project declarations | No built-in recognition; onerous for common library patterns. |
| ML/heuristic recognition | Non-deterministic; violates auditability and reproducibility requirements. |
| Syntactic similarity for validation equivalence | Unsound; produces false positives from coincidental similarity. |
| Data-flow equivalence for validation | Over-engineered; cannot distinguish constraint identity from value identity. |
| Protobuf/flatbuffers for evidence | Over-engineered for v1; JSON is sufficient, human-readable, and diffable. |
| Strict version match → `unknown` | Too strict; minor schema additions should be backward-compatible. |
| Inline evidence in config.toml | Config bloat; evidence is external and independently versioned. |

---

## 11. Immutable Reference

This decision record is immutable. Any future change to the trust-boundary contract SHALL supersede this record via a new decision gate and SHALL preserve the rejected alternatives and rationale.

---

## 12. Scope Trigger for Revisit

Trigger to revisit: A companion evidence-production tool is selected for integration, revealing schema gaps or configuration friction requiring contract changes.