//! Condition-context regression pins (vampiro-224.7).
//!
//! The dogfood-5 R5 class: `if is_leap(year)` flagged `bool` vs `int`.
//! Root cause (pre-224.9): the return-boundary check compared caller vs
//! callee codomains on *every* call edge, so a mid-body condition call
//! (`days_in_month -> is_leap`) compared `is_leap`'s `bool` against
//! `days_in_month`'s `u64` — triage read the site as "expected shape
//! stolen from neighboring match arms". 224.9's return-position gating
//! eliminated the mechanism; conditions never became slot edges.
//!
//! These tests pin the observable contract so it cannot regress: calls in
//! `if` conditions, `match` scrutinees, and `while` conditions produce no
//! composition findings on compiling code. Rust conditions are strictly
//! `bool`; if condition operands are ever promoted to data-flow slots,
//! they must expect `Scalar(Bool)`.

use std::path::Path;

use vampiro_cir::Frontend;
use vampiro_rust_frontend::RustFrontend;
use vampiro_seam_analysis::CompositionAnalyzer;

fn findings(source: &str) -> Vec<String> {
    let graph = RustFrontend
        .extract(source, Path::new("src/lib.rs"))
        .expect("frontend extraction must succeed");
    CompositionAnalyzer::new()
        .analyze(&graph)
        .iter()
        .map(|f| f.rule.clone())
        .collect()
}

/// A `bool`-returning callee in an `if` condition (mid-function and in a
/// tail-position match arm) must not be flagged.
#[test]
fn if_condition_bool_not_flagged() {
    let source = r#"
        fn is_leap(y: u64) -> bool { y % 4 == 0 }

        fn days_in_month(year: u64, month: u8) -> u64 {
            match month {
                1 | 3 => 31,
                2 => {
                    if is_leap(year) {
                        29
                    } else {
                        28
                    }
                }
                _ => 30,
            }
        }
    "#;
    assert!(
        findings(source).is_empty(),
        "if-condition call must not be flagged, got {:?}",
        findings(source)
    );
}

/// The `match` scrutinee context: a bool call driving a match expression
/// must not be flagged (the dont events.rs `epoch_to_parts` shape).
#[test]
fn match_scrutinee_bool_not_flagged() {
    let source = r#"
        fn is_leap(y: u64) -> bool { y % 4 == 0 }

        fn days_of(yd: bool) -> u64 {
            match is_leap(2024) {
                true => 366,
                false => 365,
            }
            .min(if yd { 2 } else { 3 })
        }
    "#;
    assert!(
        findings(source).is_empty(),
        "match-scrutinee call must not be flagged, got {:?}",
        findings(source)
    );
}

/// A `bool`-returning callee in a `while` condition must not be flagged.
#[test]
fn while_condition_bool_not_flagged() {
    let source = r#"
        fn has_next(y: u64) -> bool { y < 3000 }

        fn advance(mut year: u64) -> u64 {
            while has_next(year) {
                year += 1;
            }
            year
        }
    "#;
    assert!(
        findings(source).is_empty(),
        "while-condition call must not be flagged, got {:?}",
        findings(source)
    );
}
