// Seeded stress fixture — COMPOSITION BREAK (REQ-7).
//
// `aggregate` declares a return type of `(u32, u32)` (codomain
// `Record[Scalar, Scalar]`) but calls `source_value`, which returns
// `Option<u32>` (codomain `Parameterized{Option, [Scalar]}`). The callee
// codomain does not unify with the caller codomain — a composition break at
// the return boundary.
//
// NOTE: the soundness assertion for this defect is pinned via a hand-built
// CIR graph in the test (mirroring the testaruda seeded-fault pattern). This
// source file documents the defect pattern the graph represents.

fn source_value() -> Option<u32> {
    Some(0)
}

pub fn aggregate() -> (u32, u32) {
    let _v = source_value();
    (0, 0)
}
