// Seeded stress fixture — DATA-FLOW SEAM (slot-boundary check).
//
// `parse_amount` returns `Option<f64>` (Parameterized{Option,[Scalar]}) but
// its result is passed as an argument to `apply_discount`, which at slot 0
// expects `(f64, f64)` (Record[Scalar, Scalar]). The containing function
// `total` returns `()` (Scalar) so the return-boundary check is skipped by
// the void-guard, isolating the slot-boundary check.
//
// The slot-boundary check compares `total.codomain` (Scalar = ()) against
// `apply_discount.domain_slot(0)` (Record[Scalar, Scalar]) and finds a
// mismatch — a composition break at the data-flow boundary.

fn parse_amount(input: &str) -> Option<f64> {
    input.parse().ok()
}

fn apply_discount(amount: (f64, f64), rate: f64) -> f64 {
    amount.0 * amount.1 * rate
}

pub fn total(input: &str) {
    apply_discount(parse_amount(input), 0.9);
}