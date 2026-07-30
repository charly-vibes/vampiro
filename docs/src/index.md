> *"¿Por qué me tratas tan mal? ¿Por qué te escapas? ¿Por qué no ves*
> *Que si me matas tal vez entre las sombras renaceré?*
> *No pensés en eso, yo estoy bien*
> *Solamente los espejos quieren mi reflejo esconder"*
> — Charly García

# Vampiro

Vampiro is a cross-language Rust CLI (v0.2.0) that asks one question at every
call boundary: **does this edge compose validly in the category it claims to
compose in?**

It detects four classes of seam defect:

| Axis | Question |
|---|---|
| Composition | Does the produced structural shape match what the caller accepts? |
| Modularity | Does the edge respect the target module's declared interface? |
| Optionality | Are structurally interchangeable implementations lawfully interchangeable? |
| Robustness | Are effects, retries, fallbacks, and resource obligations handled completely? |

## Current status

**v0.2.0** — Working CLI with frontends for 4 languages (Python, Clojure,
Julia, Rust), composition seam analysis, cross-language data-flow edge
verification, and benchmarked performance. EARS specification v1.3.0 approved.
Active OpenSpec changes under `openspec/changes/`.

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
