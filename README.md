> *"¿Por qué me tratas tan mal? ¿Por qué te escapas? ¿Por qué no ves*
> *Que si me matas tal vez entre las sombras renaceré?*
> *No pensés en eso, yo estoy bien*
> *Solamente los espejos quieren mi reflejo esconder"*
> — Charly García

# Vampiro

Vampiro is a cross-language Rust CLI that checks whether code composes
correctly across call, module, effect, law, retry, resource, and trust
boundaries.

It asks one question at every call boundary: **does this edge compose validly
in the category it claims to compose in?**

## Seam detection axes

| Axis | Question |
|---|---|
| **Composition** | Does the produced structural shape match what the caller accepts? |
| **Modularity** | Does the edge respect the target module's declared interface? |
| **Optionality** | Are structurally interchangeable implementations lawfully interchangeable? |
| **Robustness** | Are effects, retries, fallbacks, and resource obligations handled completely? |

## Current status

**v0.3.1** — Full genesis module adoption and cross-language frontends across vampiro-cli:

| Language | Frontend | Data-flow edges | Tests |
|---|---|---|---|
| Python | ✅ | ✅ | 73 |
| Clojure | ✅ | ✅ | 37 |
| Julia | ✅ | ✅ | 31 |
| Rust | ✅ | Partial | 96 |

> 811 tests across the workspace (frontend suites per language plus CLI, CIR,
> seam-analysis, and lifecycle-analysis crates).

- **Composition seam analysis**: active — detects structural shape mismatches
  at call boundaries with ~99.8% precision on clean baselines.
- **Cross-language verification**: seeded-fault E2E suite verifies data-flow
  edge structure across all 3 non-Rust frontends.
- **Benchmarking**: 100 lines in ~10ms, 1k in ~270ms, 10k in ~21s.
- **Specification**: EARS v1.3.0 approved. Active OpenSpec changes under
  `openspec/changes/`.

## Quick start

```bash
cargo build --release
./target/release/vampiro check --path <file> --mode guidance
```

## Documentation

- [Documentation site](https://charly-vibes.github.io/vampiro/) — EARS
  specification, roadmap, proposals, and designs.
- [`vampiro-ears-spec.md`](vampiro-ears-spec.md) — authoritative requirements.
- [`CHANGELOG.md`](CHANGELOG.md) — release history.

## Local documentation build

```bash
python scripts/build_docs.py
mdbook build
```

The rendered site (`docs/book/`) is a derived artifact and is not committed.

## Project structure

```
crates/
  vampiro-cir/              # Composition IR types
  vampiro-cli/              # CLI binary
  vampiro-clojure-frontend/ # Clojure parser + CIR extraction
  vampiro-julia-frontend/   # Julia parser + CIR extraction
  vampiro-python-frontend/  # Python parser + CIR extraction
  vampiro-rust-frontend/    # Rust parser + CIR extraction
  vampiro-seam-analysis/    # Composition/modularity/robustness checks
  vampiro-law/              # Law verification
  vampiro-lifecycle-analysis/ # Lifecycle/safety analysis
  vampiro-frontend-harness/ # Frontend plugin harness
```
