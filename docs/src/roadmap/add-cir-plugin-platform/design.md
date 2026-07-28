> **Status:** Active OpenSpec proposal; not implemented or deployed. The source under `openspec/changes/` is authoritative.

## Context
CIR is the sole boundary between syntax plugins and language-neutral analysis.

## Goals / Non-Goals
- Goals: loss-aware recursive models, deterministic plugins, declarative extensibility, validated conformance.
- Non-goals: language parsing, source findings, scan orchestration, or law execution.

## Data Model Decisions

### Effect channels as recursive coproducts
Model built-in IDs (`plain`, `result`, `option`, `throws`, `async`, `stream`) and project-declared effect/functor IDs as recursive coproduct terms; reserve `unknown` for undeclared/unrecognized wrappers. `unknown` never defaults to `plain`.

### Structural shapes with opaque sentinel
Represent shapes structurally (scalar, record, union, function, ref, parameterized) and permit `Opaque` for cases where extraction is not possible (fully dynamic, untyped, no annotations). The choice of `Opaque` (rather than `Unknown`) for the shape sentinel follows the convention: *sentinel variants use `Unknown` for extraction failures, `Opaque` for intrinsically non-extractable structures*.

### Stable identities
Stable identities follow the scheme: `StableId = SHA256(content_hash + ":" + path + ":" + line)` truncated to 128 bits, where `content_hash` is a hash of the source content at the declaration/call site, `path` is the relative file path, and `line` is the start line. This ensures:
- Same source + same location → same ID (repeatable)
- Different source at same location → different ID (content-sensitive)
- Same source at different location → different ID (location-sensitive)

### Discard spans and provenance
Record exact discard spans and callee-to-caller argument provenance through configured `H` local-binding hops. Chains exceeding `H` hops are terminated explicitly as `OverBound`, preserving the hops that were successfully traced.

### Unwrap evidence and totality semantics
The full 2×2 matrix for unwrap evidence:

| Kind \ Totality | Total | Partial |
|-----------------|-------|---------|
| **Ordinary** | `resolution=unwrapped, totality=total` — all branches handled (e.g. `?` operator) | `resolution=unwrapped, totality=partial` — ordinary unwrap with unhandled branches |
| **Force** | `resolution=swallowed, totality=total` — every summand has an intentional branch | `resolution=swallowed, totality=partial` — force/panic unwrap with unhandled branches |

## Validation Decisions

### Resource limits
Declarations and graphs exceeding configured resource limits are rejected as configuration errors, never as partial validation. Fixtures test the exhaustive algorithm but never substitute for validating each declaration. The current limits:
- **Effect channels:** max 64 nesting levels
- **Shapes:** max 64 nesting levels

### Category and filtration validation
Require an explicit identity morphism for every declared object and reject missing identities before closure; closure never repairs the declaration. Construct finite closure from the validated identities and non-identity generators, then exhaustively validate composition tables, laws, wide subcategories, filtration nesting, and `filtration_level(e)` (the least containing filtration level for a finding edge, distinct from `Severity::sev()`).

> **Note:** Section 2 (category/filtration validation) is committed — pure validation logic, testable without frontends. Section 3 (plugin load and conformance) is partially deferred until at least one frontend exists (see tasks.md).

## Plugin Boundary Decisions

### Plugin loading and conformance
Load plugins only after versioned fixtures verify deterministic output, identity/composition preservation, and shared-diagram naturality.

## Post-Hoc Adjustments (YAGNI)

The YAGNI decision gate (see `docs/decisions/plugin-boundary.md`) reduced the scope:
- **Plugin ABI:** workspace-crate ABI only (no dynamic loading, no serialization, no subprocess)
- **Sections 3–4:** scoped to in-process trait dispatch via the `Frontend` trait
- **Trigger to revisit:** a second frontend enters active development (P1+) or a third-party contributor requests a plugin API

## Risks / Trade-offs
- Finite closure can still grow; reject declarations exceeding configured resource limits as configuration errors, never as partial validation. Fixtures test the exhaustive algorithm but never substitute for validating each declaration.
- Plugin ABI stability is difficult in Rust; packaging/ABI and serialization/versioning are pre-implementation decisions covered by compatibility fixtures.

## Decision Gate
- A HITL decision ticket SHALL select and document plugin packaging/ABI,
  serialization/schema-version policy, compatibility window, migration policy,
  resource limits, approver, and immutable review reference before platform
  implementation.