# Changelog

## [0.3.0] — 2026-07-31

### Added

- **Full genesis v0.4.0 module adoption across vampiro-cli** — all 14 modules
  now imported and used.
- **`vampiro init` command** — project scaffolding via `genesis::scaffold::Scaffold`,
  including `.genesis/tools.toml` manifest registration via `genesis::discovery`.
- **`vampiro feedback` command** — file bug/feature reports via
  `genesis::feedback::handle_feedback` (scratch, redactor, gh).
- **`vampiro --completions <SHELL>`** — shell completion generation via
  `genesis::cli::generate_completions`.
- **`--version --json` support** — pre-parse check via
  `genesis::cli::maybe_print_version_json`.
- **`genesis::fixture::Fixture` in test infra** — replaces manual `tempfile` + git
  CLI setup in scan/gating tests.
- **Genesis compatibility test suite** — 12 tests covering all 14 genesis modules.

### Changed

- `clap_complete` added as direct dependency.
- Updated AIX artifacts (`llms.txt`) with genesis adoption table.
- CLI help snapshot updated for new commands and options.

## [0.2.0] — 2026-07-30

### Added

- **Data-flow edge infrastructure**: per-slot expression nodes (`NodeKind::Expression`)
  and slot-indexed edges in CIR, enabling fine-grained argument tracking across call
  boundaries.

- **Python frontend**: full data-flow edge extraction with `ScalarKind` inference
  (`int`, `float`, `str`, `bool`, `None`, `bytes`) for call arguments with known
  shapes. Verified via seeded-fault fixture suite.

- **Clojure frontend**: data-flow edge extraction with per-slot expression nodes
  for call arguments. Fixed `num_lit` shape inference (Clojure grammar uses `num_lit`
  not `int_lit`/`float_lit`). Fixed early-return bug that skipped expression nodes
  for calls to builtins not in the graph (e.g., `+`, `str`, `concat`).

- **Julia frontend**: data-flow edge extraction. Fixed argument extraction to
  dig into the `argument_list` child (Julia grammar nests arguments inside
  `argument_list`, not as direct `call_expression` children).

- **Cross-language stress-test suite**: 8 realistic fixtures across Python, Clojure,
  and Julia (HTTP servers, CLI tools, async examples, data pipelines) verifying
  extraction succeeds without panic.

- **Edge-case corpus**: 23 edge-case fixtures across Rust, Python, Clojure, and Julia
  (empty, comments-only, syntax errors, macros, generics, async, unsafe, Unicode,
  const eval, enormous chains).

- **Benchmark suite**: 4 benchmark tests (100, 1k, 10k, 50k lines) with timing
  assertions. Results documented in `docs/verification/benchmarks-1.md`.

- **Seeded-fault E2E suite**: cross-language fixtures verifying data-flow edge
  structure (slot edges + expression nodes) for all 3 frontends.

### Changed

- **genesis adoption**: upgraded from genesis v0.2.0 to v0.4.0, adopting the `doctor`,
  `feedback`, `cli`, `status`, `scaffold`, `guide`, and `discovery` modules with
  `CliVerbosity`/`CliFormat`/`Output`/`Verbosity` shared infrastructure.

- **ScalarKind shape model**: refined from opaque `Shape` to `ScalarKind` enum
  (`Int`, `Float`, `String`, `Bool`, `Unit`, `Char`, `Bytes`) for finer-grained
  composition analysis.

- **Composition analysis**: kind-aware scalar matching, Vec↔slice aliasing, unified
  `Scalar`↔`Ref(Scalar)` matching, redesigned slot-boundary check using `arg_shape`
  instead of `caller.codomain` — eliminating ~99.8% of false positives.

- **Test-code filtering**: `is_test` node flag added to CIR; test-code findings
  filtered out in default analysis mode.
