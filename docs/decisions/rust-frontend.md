# Rust Frontend Decision

> Parser and idiom boundary for the Rust language frontend.

**Approver:** charly vibes
**Review reference:** bd issue vampiro-0vb.3.1 — Rust parser and idiom boundary decision gate
**Date:** 2026-07-28

---

## 1. Parser choice

**Decision:** `syn` — the de facto standard Rust parser library.

**Rationale:**
- `syn` produces a typed AST directly from Rust source, which maps naturally to CIR nodes, edges, and shapes.
- It is the most widely used Rust parser in the ecosystem, with excellent maintenance and community support.
- A typed AST requires no CST→CIR mapping layer — `syn::visit`/`visit_mut` traits let us walk the tree and emit CIR types directly.
- `syn` handles all stable Rust syntax and editions explicitly via `syn::parse_file` with a `ParseStream`.
- The dependency is lightweight (pure Rust, no C FFI, no runtime).
- tree-sitter was considered but rejected: its CST→CIR mapping layer adds complexity for no benefit with a single frontend, and the incremental/error-tolerant parsing features are not needed for static analysis that scans entire files.

**Trigger to revisit:** A second frontend enters active development (P1+). At that point, tree-sitter's shared runtime across languages becomes a stronger argument.

---

## 2. Rust edition and version range

**Decision:** Minimum supported edition is **Rust 2021**. No specific version cap — `syn` tracks the latest stable Rust.

**Rationale:**
- Rust 2021 was released in 2021 and is now the baseline for the ecosystem.
- `syn`'s `parse_file` accepts an edition parameter; we default to `Edition::Edition2021`.
- Projects on older editions (2015, 2018) will need to opt in explicitly or be rejected at the frontend boundary.

---

## 3. Macro expansion boundary

**Decision:** No macro expansion. We parse post-expansion source or treat macro invocations as opaque.

**Rationale:**
- `syn` parses the *output* of macro expansion, not the pre-expansion token stream.
- Full macro expansion requires a compiler (rustc or rust-analyzer), which is out of scope.
- For files that contain unexpanded macros at the top level, we emit an `unknown` / `opaque` sentinel for the affected declarations.
- This is consistent with the CIR design principle of explicit unknowns rather than guesses.

---

## 4. Initial effect and idiom table

**Decision:** Recognize the following built-in wrapper patterns as idiom entries:

| Pattern | Effect Channel | Resolution |
|---------|---------------|------------|
| `Result<T, E>` | `EffectChannel::Result` | `EffectResolution::Unwrapped` (ordinary) |
| `Option<T>` | `EffectChannel::Option` | `EffectResolution::Unwrapped` (ordinary) |
| `async { ... }` / `async fn` | `EffectChannel::Async` | `EffectResolution::Propagated` |
| `impl Iterator` / `-> T where ...` | `EffectChannel::Stream` | `EffectResolution::Propagated` |
| `unwrap()` | — | `EffectResolution::Unwrapped` (ordinary, total) |
| `expect(...)` | — | `EffectResolution::Unwrapped` (ordinary, total) |
| `?` operator | — | `EffectResolution::Unwrapped` (ordinary, total) |
| Panic/force: `.unwrap_unchecked()`, indexing `[i]` | — | `EffectResolution::Swallowed` (force, partial) |

**Rationale:** These patterns cover the vast majority of effect-manipulation code in Rust. Additional patterns can be added to the idiom table as needed without changing the extraction contract.

---

## 5. Exclusions

The following Rust constructs are explicitly **unsupported** and will produce an `unknown` / `opaque` sentinel:

| Construct | Reason |
|-----------|--------|
| Inline assembly (`asm!`) | Cannot analyze without execution |
| Procedural macros (custom derive, attribute) | Require compiler execution |
| Unsafe blocks (body only — `unsafe fn` signature is analyzed) | Safety analysis is a separate concern |
| Dynamic dispatch via trait objects | Cannot determine concrete type statically |
| Cross-crate re-exports (non-local `pub use`) | Requires crate graph resolution |

---

## 6. Rejected alternatives

| Alternative | Reason |
|-------------|--------|
| tree-sitter-rust | CST→CIR mapping adds complexity; incremental parsing not needed; C FFI dependency |
| rust-analyzer-assisted | Too heavy; full semantic analysis is overkill for structural CIR extraction |
| Manual recursive descent parser | Would duplicate years of `syn` development effort |
| `rustc` as a library (via `rustc_driver`) | Unstable API, long compile times, heavy dependency |

---

## 7. Scope and compatibility

- **Supported scope:** Rust 2021+ source files, parsed with `syn`, extracted to CIR v0.1.0.
- **Immutability:** This decision is valid until the trigger condition is met. When revisited, the new decision record SHALL supersede this one and SHALL preserve the rejected alternatives and rationale.