> *"¿Por qué me tratas tan mal? ¿Por qué te escapas? ¿Por qué no ves*
> *Que si me matas tal vez entre las sombras renaceré?*
> *No pensés en eso, yo estoy bien*
> *Solamente los espejos quieren mi reflejo esconder"*
> — Charly García

# Vampiro

> **Why:** cross-language codebases break at the seams — call, module, effect,
> law, retry, resource, and trust boundaries where two pieces *look* compatible
> but don't compose. Vampiro proves composition across those boundaries
> mechanically instead of hoping integration tests catch it.
> **Status:** [beta](docs/src/status.md) · prove/check shipped · [Motivation & design](docs/src/index.md) · [charly-vibes Tool Ecosystem](https://charly-vibes.github.io/dulce-de-leche/ecosystem-map.html)
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

**v0.5.0** — Production-precision composition analysis across 4 languages
(Rust, Python, Clojure, Julia):

| Language | Frontend | Data-flow edges | Field-type registry |
|---|---|---|---|
| Rust | ✅ | ✅ | ✅ |
| Python | ✅ | ✅ | — |
| Clojure | ✅ | ✅ | — |
| Julia | ✅ | ✅ | — |

> 898 tests across the workspace (frontend suites per language plus CLI, CIR,
> seam-analysis, law, and lifecycle-analysis crates).

- **Composition seam analysis**: active — per-slot argument shape inference
  with struct-field registry, variant rewraps, method-output table, deref
  coercion, and success-channel Result unification. **0 false-positive
  composition and redundancy findings on all 8 foreign ecosystem repos**
  (dogfood round 5; see `docs/verification/dogfood-5.md`).
- **Redundancy / modularity / robustness axes**: active, expression-level
  branch grouping, test-code filtering, artifact-dir exclusion at discovery.
- **Law verification** and **lifecycle analysis** crates are available in the
  workspace but not yet integrated into the CLI (`vampiro prove` prints a
  placeholder message).
- **Benchmarking**: 100 lines in ~10ms, 1k in ~65ms, 10k in ~0.63s.
- **Specification**: EARS v1.3.0 approved. No active OpenSpec changes —
  backlog clear.

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
