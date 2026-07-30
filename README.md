> *"¿Por qué me tratas tan mal? ¿Por qué te escapas? ¿Por qué no ves*
> *Que si me matas tal vez entre las sombras renaceré?*
> *No pensés en eso, yo estoy bien*
> *Solamente los espejos quieren mi reflejo esconder"*
> — Charly García

# Vampiro

Vampiro is a planned cross-language Rust CLI for checking whether code composes
correctly across call, module, effect, law, retry, resource, and trust
boundaries.

The project is currently in specification and proposal review. No application
implementation has started. The authoritative requirements are in
[`vampiro-ears-spec.md`](vampiro-ears-spec.md), and proposed implementation
changes are under [`openspec/changes/`](openspec/changes/).

## Documentation

The documentation site publishes the authoritative EARS specification and all
active OpenSpec proposals, designs, tasks, and capability deltas:

<https://charly-vibes.github.io/vampiro/>

GitHub Pages is deployed from an Actions artifact. The repository does not use
or require a `gh-pages` branch.

## Local documentation build

```bash
python scripts/build_docs.py
mdbook build
```

The rendered site (`docs/book/`) is a derived artifact and is not committed.

## Planning validation

```bash
openspec validate --all --strict --no-interactive
python scripts/check_planning.py
```

Implementation remains blocked until the human approval gate recorded in
`.beads/issues.jsonl` approves every active OpenSpec proposal. The EARS
specification (v1.3.0) was approved on 2026-07-28.
