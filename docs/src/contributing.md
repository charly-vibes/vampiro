# Contributing

## Current phase

Vampiro is in the planning and approval phase. Do not implement application
code until every active OpenSpec proposal is approved. The EARS
specification (v1.3.0) was approved on 2026-07-28; the OpenSpec changes
remain the open approval gate.

## Change workflow

1. Update the authoritative EARS document when product requirements change.
2. Update the affected OpenSpec proposal, design, capability delta, and tasks.
3. Run strict OpenSpec validation.
4. Keep `.beads/issues.jsonl` synchronized with the approved task graph.
5. Build the documentation site.

```bash
openspec validate --all --strict --no-interactive
python scripts/check_planning.py
python scripts/build_docs.py
mdbook build
```

## Epistemic gate (dont)

Vampiro maintains a real `dont` claim corpus (`.dont/`), so per the
`dont-bpuo` ADR the epistemic gate is adopted. The gate is
`just check-claims`, which runs `dont check` and fails on ungrounded
claims or rule violations.

Wiring points:

- **pre-commit** — `lefthook.yml` runs `just check-claims` before every
  commit (bypass requires a `WIP` commit message per repo policy).
- **CI** — the `planning` job runs `dont check` after validating the
  OpenSpec specs and issue export.
- **Local** — `just ci` includes the gate in the full pipeline.

If a commit is rejected by the gate, run `dont list` to find the
offending claim, then resolve it (`dont flag <id>` with evidence) or
retract it (`dont forget`/`dont ignore` per the dont docs).

## Documentation rules

- Do not hand-edit generated files under `docs/book/`.
- Requirement IDs must remain stable and traceable.
- Active proposal pages must not claim deployed behavior.
- OpenSpec deltas remain under `openspec/changes/` until implementation,
  review, and archival are complete.

## GitHub Pages

Pages uses GitHub's official artifact deployment actions. Pull requests build
the site but do not deploy it. Pushes to `main` upload the rendered `docs/book/`
directory and deploy it to the protected `github-pages` environment. No
publishing branch is created.
