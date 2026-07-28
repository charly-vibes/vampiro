// Negative fixture for the effect-handling tracer (REQ-9, REQ-C4).
//
// `parse_raw` returns `Option<f64>` and `lookup_price` returns `Result<f64, String>`.
// The caller discards both effects at the call site, producing swallowed edges.
//
// - `_ = parse_raw(...)` — the Option effect is swallowed (line N).
// - `price? = lookup_price(...)` — the Result effect would be unwrapped via `?`,
//   but the second call at line M discards it via `_`.
// - `unwrap()` on an Option is a force/panic unwrap, treated as partial
//   (swallowed) unless every summand has an intentional branch.
//
// The top-level `total` returns `f64` so its edges are checked.

fn parse_raw(raw: &str) -> Option<f64> {
    let _ = raw;
    None
}

fn lookup_price(id: u32) -> Result<f64, String> {
    let _ = id;
    Ok(42.0)
}

fn force_unwrap(val: Option<f64>) -> f64 {
    val.unwrap()
}

pub fn total(raw: &str, id: u32) -> f64 {
    // Discard the Option from parse_raw (swallowed effect).
    let _ = parse_raw(raw);
    // Discard the Result from lookup_price (swallowed effect).
    let _ = lookup_price(id);
    // force-unwrap is a separate call with Force+Partial semantics.
    force_unwrap(parse_raw(raw))
}