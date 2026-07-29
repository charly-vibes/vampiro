// Seeded stress fixture — REDUNDANCY MISMATCH (REQ-11, REQ-C7).
//
// `use_data` is a consumer node receiving data from two branch sources with
// different codomain shapes: `primary_source_fetch` returns `(f64, String)`
// (`Record[Scalar, Scalar]`) while `cache_get` returns `Option<f64>`
// (`Parameterized{Option, [Scalar]}`). No explicit adapter node reconciles
// the two shapes before they reach `use_data`.
//
// NOTE: a consumer with >=2 inbound edges from differently-shaped sources is
// not naturally produced by single-file frontend extraction. The soundness
// assertion is therefore pinned via a hand-built CIR graph in the test. This
// source file documents the defect pattern.

fn primary_source_fetch(id: u32) -> (f64, String) {
    let _ = id;
    (42.0, "ok".into())
}

fn cache_get(id: u32) -> Option<f64> {
    let _ = id;
    None
}

pub fn use_data(rec: (f64, String)) -> f64 {
    rec.0
}
