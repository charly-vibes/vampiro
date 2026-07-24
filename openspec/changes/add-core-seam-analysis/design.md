## Context
All checks consume CIR and declarative category data; language plugins only provide facts.

## Goals / Non-Goals
- Goals: language-neutral deterministic checks, recursive effects, correct diagnostic asymmetry, conservative opacity.
- Non-goals: law/proof checks, scan modes/output, or language-specific parsing.

## Decisions
- Implement structural unification as a deliberately coarse shape relation with explicit union/sum arms and opaque short-circuit only for composition.
- Traverse recursive effect coproducts one layer at a time; ancestor handling stops at declared request/process boundaries.
- Treat ordinary removal as `resolution=unwrapped` with independently computed totality; panic/force removal carries wrapper-removal evidence but uses `resolution=swallowed, totality=partial` unless every summand is intentionally handled.
- Model visibility legitimacy from ancestry plus facade/export generators; enforced-unreachable edges diagnose plugins, while advisory crossings diagnose source.
- Require explicit adapters to form a common codomain for every redundancy branch.
- Emit redundancy only on the robustness axis; plugin defects remain diagnostics outside the four finding axes.

## Risks / Trade-offs
- Conservative extraction can reduce composition coverage; never convert uncertainty into validity.
- Ancestor traversal can grow; memoize by edge, exception/effect layer, and boundary.

## Open Questions
- Confirm initial structural-shape hash/canonicalization details during implementation.
