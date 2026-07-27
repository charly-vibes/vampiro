# Contributing

## Current phase

Vampiro is in the planning and approval phase. Do not implement application
code until the human approval gate accepts EARS 1.1.0 and all eight active
OpenSpec proposals.

## Prerequisites

- [Rust](https://rustup.rs) — stable toolchain (edition 2024)
- [just](https://just.systems) — command runner (`cargo install just` or `brew install just`)
- [wai](https://github.com/charly-vibes/wai) — workflow context tracking
- [bd / beads](https://github.com/charly-vibes/beads) — issue tracking (`bd` CLI)
- [mdBook](https://rust-lang.github.io/mdBook/) — docs build (`cargo install mdbook`)
- [typos](https://github.com/crate-ci/typos) — spell checker (`cargo install typos-cli`)
- [lefthook](https://github.com/evilmartians/lefthook) — git hooks (`cargo install lefthook`)

## Setup

```bash
git clone https://github.com/charly-vibes/vampiro.git
cd vampiro
cargo build
just setup
```

## Workflow

1. **Start a session:** `wai prime`
2. **Find work:** `bd ready`
3. **Run checks:** `just check` before committing
4. **Build docs:** `just docs`
5. **End session:** `wai close`

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

## Quality gates

- All commits must pass `just pre-push` (fmt check + lint + test)
- Push is blocked if tests fail (bypass with `git push --no-verify` and `WIP` in commit message)
- All PRs must pass CI (OpenSpec validation, docs build, tests)

## Documentation rules

- Do not hand-edit generated files under `docs/book/`.
- Requirement IDs must remain stable and traceable.
- Active proposal pages must not claim deployed behavior.
- OpenSpec deltas remain under `openspec/changes/` until implementation, review, and archival are complete.