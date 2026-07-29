# Benchmark Results: `vampiro check` on generated Rust source

**Date:** 2026-07-29
**Ticket:** vampiro-tmf.6
**Pipeline:** `vampiro check --path <file> --full --mode guidance` (RustFrontend → VisibilityFacts → CompositionAnalyzer + EffectHandlingAnalyzer + RedundancyAnalyzer + ModularityAnalyzer → findings output)
**Methodology:** Generate Rust source files of increasing size with a chain of functions (each calling the next), plus struct definitions, a trait impl, and an entry point. Time the full `vampiro check` pipeline from subprocess start to exit.

## Results

| File size | Lines | Wall time | vs target | Status |
|-----------|------:|----------:|-----------|--------|
| 100 lines | 106   | 10 ms     | —         | ✅ Pass |
| 1k lines  | 1,006 | 271 ms    | —         | ✅ Pass |
| 10k lines | 10,006 | 21.3 s ❌ | <5s      | ❌ Target not met |
| 50k lines | 50,006 | 565 s ❌ | —         | ❌ Far exceeds graceful degradation (<30s) |
| 100k lines| —     | timed out | <30s      | ❌ Cannot complete |

## Analysis

The Rust frontend scales poorly with file size. Performance appears to be
worse than linear (likely O(n²) or higher due to the call-graph construction
algorithm). The breakdown:

- **Parse (tree-sitter):** fast — sub-second even for 50k lines
- **CIR graph construction:** bottlenecks on large numbers of function nodes
  and call edges
- **Analysis phase (composition, effect handling, redundancy):** further
  multiplies the cost on the large graph

## Recommendations

1. **Profile the extraction pipeline** — identify whether the bottleneck is
   node ID generation (string formatting), edge construction, or graph
   traversal. File a P2 perf bug.
2. **Avoid running `--full` on files >5k lines** in CI/dogfood pipelines.
   Use `--mode guidance` (parse-only) for large files.
3. **Consider streaming extraction** for large files — process function by
   function instead of building a complete graph in memory.
4. **Benchmark only on demand** — the `bench_50k_lines` test is marked
   `#[ignore]` and must be run explicitly:  
   `cargo test --test benchmarks -- bench_50k_lines -- --ignored --nocapture`

## Benchmark infrastructure

- Test file: `crates/vampiro-cli/tests/benchmarks.rs`
- Generated fixtures: `target/bench-fixtures/` (auto-generated, not committed)
- Run all small benchmarcks: `cargo test --test benchmarks -- bench_100_lines bench_1k_lines bench_10k_lines -- --nocapture`