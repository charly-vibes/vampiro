//! Benchmark tests for vampiro (vampiro-tmf.6).
//!
//! Generates Rust source files of increasing size, times `vampiro check`
//! on each, and records results.
//!
//! Run: `cargo test --test benchmarks -- --nocapture`

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// Resolve the workspace root from this crate's manifest dir.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Absolute path to the `vampiro` binary.
fn vampiro_bin() -> &'static str {
    env!("CARGO_BIN_EXE_vampiro")
}

/// Output directory for generated benchmark files.
fn bench_dir() -> PathBuf {
    workspace_root().join("target/bench-fixtures")
}

/// Generate a Rust source file with `n_fns` functions by writing directly to disk.
fn generate_rust_source(n_fns: usize, path: &Path) {
    let mut f = std::fs::File::create(path)
        .unwrap_or_else(|e| panic!("failed to create {}: {e}", path.display()));

    writeln!(f, "// Auto-generated benchmark fixture — do not edit").unwrap();
    writeln!(f, "#![allow(unused)]\n").unwrap();
    writeln!(f, "struct Point {{ x: i32, y: i32 }}").unwrap();
    writeln!(f, "trait Transform {{ fn apply(&self, p: Point) -> Point; }}\n").unwrap();
    writeln!(f, "struct Identity;").unwrap();
    writeln!(f, "impl Transform for Identity {{").unwrap();
    writeln!(f, "    fn apply(&self, p: Point) -> Point {{ p }}").unwrap();
    writeln!(f, "}}\n").unwrap();

    // Generate function chain — write directly to file (no intermediate String)
    for i in 0..n_fns {
        if i < n_fns - 1 {
            write!(f, "fn f{i:06}() -> u32 {{ f{:06}() + 1 }}\n", i + 1).unwrap();
        } else {
            write!(f, "fn f{i:06}() -> u32 {{ 0 }}\n").unwrap();
        }
    }

    writeln!(f, "\nfn main() {{").unwrap();
    writeln!(f, "    let _ = f000000();").unwrap();
    writeln!(f, "    let _ = Identity.apply(Point {{ x: 1, y: 2 }});").unwrap();
    writeln!(f, "}}").unwrap();
}

/// Count approximate source lines.
fn count_lines(path: &Path) -> usize {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    content.lines().count()
}

/// Format a duration in milliseconds.
fn ms(d: std::time::Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Format a duration in seconds.
fn secs(d: std::time::Duration) -> f64 {
    d.as_secs_f64()
}

/// Run `vampiro check --path <file> --full --mode guidance` and measure timing.
fn bench_vampiro(path: &Path) -> (std::time::Duration, bool) {
    let start = Instant::now();
    let output = Command::new(vampiro_bin())
        .arg("check")
        .arg("--path")
        .arg(path.to_string_lossy().as_ref())
        .arg("--full")
        .arg("--mode")
        .arg("guidance")
        .output()
        .expect("vampiro check subprocess failed");
    let elapsed = start.elapsed();
    let succeeded = output.status.success();
    (elapsed, succeeded)
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

#[test]
fn bench_100_lines() {
    let path = bench_dir().join("bench_100.rs");
    generate_rust_source(90, &path);
    let lines = count_lines(&path);
    println!("  lines: {lines}");

    let (duration, ok) = bench_vampiro(&path);
    println!("  time:  {:.2}ms", ms(duration));
    assert!(ok, "vampiro check failed on {} line file", lines);
}

#[test]
fn bench_1k_lines() {
    let path = bench_dir().join("bench_1k.rs");
    generate_rust_source(990, &path);
    let lines = count_lines(&path);
    println!("  lines: {lines}");

    let (duration, ok) = bench_vampiro(&path);
    println!("  time:  {:.0}ms ({:.2}s)", ms(duration), secs(duration));
    assert!(ok, "vampiro check failed on {} line file", lines);
}

#[test]
fn bench_10k_lines() {
    let path = bench_dir().join("bench_10k.rs");
    generate_rust_source(9990, &path);
    let lines = count_lines(&path);
    println!("  lines: {lines}");

    let (duration, ok) = bench_vampiro(&path);
    println!("  time:  {:.2}s", secs(duration));
    assert!(ok, "vampiro check failed on {} line file", lines);
}

#[test]
#[ignore]
fn bench_50k_lines() {
    let path = bench_dir().join("bench_50k.rs");
    let gen_start = Instant::now();
    generate_rust_source(49990, &path);
    let gen_elapsed = gen_start.elapsed();
    println!("  gen:   {:.2}s", secs(gen_elapsed));

    let lines = count_lines(&path);
    println!("  lines: {lines}");

    let (duration, ok) = bench_vampiro(&path);
    println!("  time:  {:.2}s", secs(duration));
    assert!(ok, "vampiro check failed on {} line file", lines);
}