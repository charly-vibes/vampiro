## Context
The frontend maps Rust syntax and semantic conventions into CIR, but does not decide findings.

## Goals / Non-Goals
- Goals: deterministic useful extraction, Rust visibility/facade fidelity, extensible lifecycle hooks.
- Non-goals: full Rust type checking, core findings, lifecycle findings, or source execution.

## Decisions
- Recommend tree-sitter-rust for fast syntax extraction, augmented by Cargo metadata and conservative inference; confirm against `rust-analyzer`/`syn` fixture fidelity during implementation.
- Encode `pub` variants, module ancestry, `pub use`, `#[doc(hidden)]`, and underscore conventions as versioned idiom data.
- Emit opaque/unknown rather than guessing when syntax and available metadata are insufficient.

## Risks / Trade-offs
- Macros and type inference can hide edges/shapes; surface extraction gaps and preserve conservative classifications.
- Parser choice affects correctness and performance; gate confirmation on representative fixtures and the 50-seam workflow.

## Decision Gate
- A HITL decision ticket SHALL confirm the parser stack, supported Rust
  editions/versions, macro-expansion boundary, and initial effect, write,
  acquire, and release idioms before extraction implementation.
