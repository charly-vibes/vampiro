// Negative fixture for the redundancy tracer (REQ-11, REQ-C7).
//
// `primary_source.fetch` returns `FullRecord` while `cache.get` returns
// `PartialRecord | None`. Both feed into `use(data)` which expects a single
// shape. No adapter reconciles the two shapes before `use(data)`.
//
// This is the Python redundancy example from the EARS spec:
//   try:
//       data = primary_source.fetch(id)      # -> FullRecord
//   except SourceUnavailable:
//       data = cache.get(id)                 # -> PartialRecord | None
//   use(data)

fn primary_source_fetch(id: u32) -> (f64, String) {
    let _ = id;
    (42.0, "ok".into())
}

fn cache_get(id: u32) -> Option<f64> {
    let _ = id;
    None
}

pub fn use_data(raw: (f64, String)) -> f64 {
    raw.0
}

pub fn get_data(id: u32) -> f64 {
    let primary = primary_source_fetch(id);
    let fallback = cache_get(id);
    // Only one branch feeds into use_data — the redundancy is between
    // which path's output reaches use_data.
    if primary.0 > 0.0 {
        use_data(primary)
    } else if let Some(val) = fallback {
        val
    } else {
        0.0
    }
}