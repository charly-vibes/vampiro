// Seeded stress fixture — CLEAN BASELINE (precision).
//
// No composition breaks, no over-exposure, no swallowed effects, no
// redundancy mismatches. Vampiro MUST report zero findings on this file.
//
// Consumed by `crates/vampiro-seam-analysis/tests/stress_seeded_fixtures.rs`
// (`fixtures_are_precise`).

pub fn add(a: f64, b: f64) -> f64 {
    a + b
}

pub fn total(qty: u32, unit: f64) -> f64 {
    add(qty as f64, unit)
}
