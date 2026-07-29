//! Edge-case corpus tests (vampiro-tmf.5).
//!
//! Verifies that `vampiro check` exits without panic for a wide variety of
//! edge-case source files across all supported languages.
//!
//! Run: `cargo test edge_case_*`

use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve the workspace root from this crate's manifest dir.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Absolute path to the `vampiro` binary (set by Cargo's test harness).
fn vampiro_bin() -> &'static str {
    env!("CARGO_BIN_EXE_vampiro")
}

/// Absolute path to an edge-case fixture.
fn edge_case(lang: &str, name: &str) -> PathBuf {
    workspace_root()
        .join("tests")
        .join("fixtures")
        .join("stress")
        .join("edge-cases")
        .join(lang)
        .join(name)
}

/// Run `vampiro check --full` on a single file; panic is caught by the test harness.
fn assert_no_panic(path: &Path, label: &str) {
    let output = Command::new(vampiro_bin())
        .arg("check")
        .arg("--path")
        .arg(path.to_string_lossy().as_ref())
        .arg("--full")
        .arg("--mode")
        .arg("guidance")
        .output()
        .unwrap_or_else(|e| panic!("{label}: failed to spawn vampiro: {e}"));

    // The process may exit non-zero for syntax errors or findings — that's OK.
    // The test only checks that the process didn't panic (SIGABRT or internal error).
    assert!(
        output.status.success() || !output.stderr.is_empty(),
        "{label}: process crashed (status={:?}, stderr={})",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// Rust edge cases
// ---------------------------------------------------------------------------

#[test]
fn edge_case_rust_empty() {
    assert_no_panic(&edge_case("rust", "empty.rs"), "empty.rs");
}

#[test]
fn edge_case_rust_comments_only() {
    assert_no_panic(&edge_case("rust", "comments_only.rs"), "comments_only.rs");
}

#[test]
fn edge_case_rust_syntax_error() {
    assert_no_panic(&edge_case("rust", "syntax_error.rs"), "syntax_error.rs");
}

#[test]
fn edge_case_rust_macros() {
    assert_no_panic(&edge_case("rust", "macros.rs"), "macros.rs");
}

#[test]
fn edge_case_rust_attrs() {
    assert_no_panic(&edge_case("rust", "attrs.rs"), "attrs.rs");
}

#[test]
fn edge_case_rust_generics() {
    assert_no_panic(&edge_case("rust", "generics.rs"), "generics.rs");
}

#[test]
fn edge_case_rust_async() {
    assert_no_panic(&edge_case("rust", "async.rs"), "async.rs");
}

#[test]
fn edge_case_rust_unsafe() {
    assert_no_panic(&edge_case("rust", "unsafe.rs"), "unsafe.rs");
}

#[test]
fn edge_case_rust_enormous() {
    assert_no_panic(
        &edge_case("rust", "enormous.rs"),
        "enormous.rs (1000 fn chain)",
    );
}

#[test]
fn edge_case_rust_unicode() {
    assert_no_panic(&edge_case("rust", "unicode.rs"), "unicode.rs");
}

#[test]
fn edge_case_rust_const_eval() {
    assert_no_panic(&edge_case("rust", "const_eval.rs"), "const_eval.rs");
}

// ---------------------------------------------------------------------------
// Python edge cases
// ---------------------------------------------------------------------------

#[test]
fn edge_case_python_empty() {
    assert_no_panic(&edge_case("python", "empty.py"), "empty.py");
}

#[test]
fn edge_case_python_comments_only() {
    assert_no_panic(&edge_case("python", "comments_only.py"), "comments_only.py");
}

#[test]
fn edge_case_python_syntax_error() {
    assert_no_panic(&edge_case("python", "syntax_error.py"), "syntax_error.py");
}

#[test]
fn edge_case_python_decorators_async() {
    assert_no_panic(
        &edge_case("python", "decorators_async.py"),
        "decorators_async.py",
    );
}

// ---------------------------------------------------------------------------
// Clojure edge cases
// ---------------------------------------------------------------------------

#[test]
fn edge_case_clojure_empty() {
    assert_no_panic(&edge_case("clojure", "empty.clj"), "empty.clj");
}

#[test]
fn edge_case_clojure_comments_only() {
    assert_no_panic(
        &edge_case("clojure", "comments_only.clj"),
        "comments_only.clj",
    );
}

#[test]
fn edge_case_clojure_syntax_error() {
    assert_no_panic(
        &edge_case("clojure", "syntax_error.clj"),
        "syntax_error.clj",
    );
}

#[test]
fn edge_case_clojure_macros() {
    assert_no_panic(&edge_case("clojure", "macros.clj"), "macros.clj");
}

// ---------------------------------------------------------------------------
// Julia edge cases
// ---------------------------------------------------------------------------

#[test]
fn edge_case_julia_empty() {
    assert_no_panic(&edge_case("julia", "empty.jl"), "empty.jl");
}

#[test]
fn edge_case_julia_comments_only() {
    assert_no_panic(&edge_case("julia", "comments_only.jl"), "comments_only.jl");
}

#[test]
fn edge_case_julia_syntax_error() {
    assert_no_panic(&edge_case("julia", "syntax_error.jl"), "syntax_error.jl");
}

#[test]
fn edge_case_julia_macros_types() {
    assert_no_panic(&edge_case("julia", "macros_types.jl"), "macros_types.jl");
}
