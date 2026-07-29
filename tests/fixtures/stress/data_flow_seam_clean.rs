// Seeded stress fixture — DATA-FLOW SEAM CLEAN BASELINE.
//
// Same structure as data_flow_seam.rs but with matching types: `total` returns
// `()` (Scalar) and `apply_discount` expects `()` at slot 0 → slot-boundary
// match. Zero findings expected.

fn parse_amount(input: &str) -> Option<f64> {
    input.parse().ok()
}

fn apply_discount(_amount: (), rate: f64) -> f64 {
    rate
}

pub fn total(input: &str) {
    let _ = parse_amount(input);
    apply_discount((), 0.9);
}