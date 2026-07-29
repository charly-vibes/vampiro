// Seeded stress fixture — SWALLOWED EFFECT (REQ-9, REQ-C4).
//
// `lookup` returns `Result<f64, String>` (effect channel `result`). The
// caller `report` discards the result via `let _ = lookup(id);`, swallowing
// the error channel without handling it.
//
// NOTE: the Rust frontend does not yet classify edges as `Swallowed` (discard
// detection is a separate enhancement). The soundness assertion is therefore
// pinned via a hand-built CIR graph in the test. This source file documents
// the defect pattern.

fn lookup(id: u32) -> Result<f64, String> {
    let _ = id;
    Ok(0.0)
}

pub fn report(id: u32) -> f64 {
    let _ = lookup(id);
    0.0
}
