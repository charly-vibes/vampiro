// Seeded stress fixture — COMPOSITION BREAK (REQ-7).
//
// `aggregate` declares a return type of `(u32, u32)` (codomain
// `Record[Scalar, Scalar]`) but its tail expression calls `source_value`,
// which returns `Option<u32>` (codomain `Parameterized{Option, [Scalar]}`).
// The callee codomain does not unify with the caller codomain — a
// composition break at the return boundary (vampiro-224.9: the return-
// boundary check fires for return-position calls, so the break is a
// genuine TP, not statement-position noise).
//
// NOTE: the soundness assertion for this defect is pinned via a hand-built
// CIR graph in the test (mirroring the testaruda seeded-fault pattern). This
// source file documents the defect pattern the graph represents.

fn source_value() -> Option<u32> {
    Some(0)
}

pub fn aggregate() -> (u32, u32) {
    source_value()
}
