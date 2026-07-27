# Vampiro

Vampiro is a planned cross-language Rust CLI that asks one question at every
call boundary: **does this edge compose validly in the category it claims to
compose in?**

It is designed to detect four classes of seam defect:

| Axis | Question |
|---|---|
| Composition | Does the produced structural shape match what the caller accepts? |
| Modularity | Does the edge respect the target module's declared interface? |
| Optionality | Are structurally interchangeable implementations lawfully interchangeable? |
| Robustness | Are effects, retries, fallbacks, and resource obligations handled completely? |

> **Draft and unimplemented**
>
> EARS 1.1.0 and the eight OpenSpec changes are active proposals awaiting
> human approval. This site documents intended behavior; it does not claim
> that the CLI has been implemented.

## Start here

- Read the [authoritative EARS specification](specification/ears.md).
- Review the [implementation roadmap](roadmap/index.md).
- See the [project context](project-context.md) for architecture and testing
  constraints.
- Read [contributing](contributing.md) before changing requirements or starting
  implementation.

## Documentation provenance

The Pages site is assembled directly from `vampiro-ears-spec.md` and
`openspec/changes/` during CI. The rendered copies are generated artifacts;
the repository source files remain authoritative.
