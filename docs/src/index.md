> *"¿Por qué me tratas tan mal? ¿Por qué te escapas? ¿Por qué no ves*
> *Que si me matas tal vez entre las sombras renaceré?*
> *No pensés en eso, yo estoy bien*
> *Solamente los espejos quieren mi reflejo esconder"*
> — Charly García

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

> **Approved specification, proposals in progress**
>
> EARS 1.3.0 is approved. The active OpenSpec changes are proposals
> awaiting human approval. This site documents intended behavior; it does
> not claim that the CLI has been implemented.

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
