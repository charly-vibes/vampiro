// Negative fixture for the composition tracer (REQ-7).
//
// `parse_amount` returns `Option<f64>` (codomain shape
// `parameterized{Option,[Scalar]}`) but is fed directly into `apply_discount`'s
// `amount: f64` parameter (`Scalar`). The Option wrapper is not eliminated
// before the value crosses into `apply_discount` — a composition break at the
// seam between `parse_amount` and `apply_discount`.
//
// The top-level `total` returns a tuple `(f64, bool)` (codomain
// `record[scalar, scalar]`) so its edges are checked by the composition
// tracer (void-returning callers with codomain=Scalar are skipped because the
// coarse Shape model cannot distinguish different scalar types).
//
// This fixture is consumed by
// `crates/vampiro-seam-analysis/tests/composition_e2e.rs`.

fn parse_amount(raw: &str) -> Option<f64> {
    let _ = raw;
    None
}

fn apply_discount(amount: f64, pct: f64) -> f64 {
    amount - amount * pct
}

pub fn total(raw: &str, pct: f64) -> (f64, bool) {
    (apply_discount(parse_amount(raw), pct), true)
}