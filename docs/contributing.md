# Contributing

## Current phase

Vampiro is in the planning and approval phase. Do not implement application
code until the human approval gate accepts EARS 1.1.0 and all eight active
OpenSpec proposals.

## Change workflow

1. Update the authoritative EARS document when product requirements change.
2. Update the affected OpenSpec proposal, design, capability delta, and tasks.
3. Run strict OpenSpec validation.
4. Keep `.beads/issues.jsonl` synchronized with the approved task graph.
5. Build the documentation site in strict mode.

```bash
openspec validate --all --strict --no-interactive
python scripts/check_planning.py
python scripts/build_docs.py
mkdocs build --strict
```

## Documentation rules

- Do not hand-edit generated files under `.build/docs/` or `site/`.
- Requirement IDs must remain stable and traceable.
- Active proposal pages must not claim deployed behavior.
- OpenSpec deltas remain under `openspec/changes/` until implementation,
  review, and archival are complete.

## GitHub Pages

Pages uses GitHub's official artifact deployment actions. Pull requests build
the site but do not deploy it. Pushes to `main` upload the rendered `site/`
directory and deploy it to the protected `github-pages` environment. No
publishing branch is created.
