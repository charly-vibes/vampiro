## Context
Each language maps syntax and conventions to the same CIR and analysis contracts.

## Goals / Non-Goals
- Goals: independently conformant full-CIR frontends, bounded provenance, registered law runners, lifecycle facts/unknowns, L4 snapshot integration, and faithful advisory semantics.
- Non-goals: one shared parser abstraction beyond CIR contracts, or language-specific branches in core.

## Decisions
- Recommend tree-sitter grammars for an initial uniform implementation, but confirm each against native-parser options and representative fixtures during that language's task.
- Keep effect and visibility tables independently versioned per language.
- Emit facts for Python package facades, Clojure private-var reach-through, and Julia foreign-function/foreign-type ownership; core emits shared findings where applicable.
- Require each frontend to emit cluster/tags/serializable generator references, register a language runner for representable values, and emit lifecycle write/retry/resource/exit facts. Unsupported results apply to specific constructs, not an entire supported language, and remain explicit.
- Stage extraction against stabilized CIR contracts, then run final core/law/lifecycle contract acceptance without whole-change predecessor serialization.

## Risks / Trade-offs
- Dynamic constructs create opaque/unknown facts; surface them rather than guessing.
- Combining three languages risks a large delivery; use separate test-first milestones and conformance gates.

## Open Questions
- Confirm parser choice, supported language versions, and macro/module-loading boundaries separately for Python, Clojure, and Julia.
