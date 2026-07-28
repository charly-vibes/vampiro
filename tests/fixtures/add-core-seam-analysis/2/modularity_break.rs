// Negative fixture for the modularity tracer (REQ-V4 over-exposure,
// REQ-V7 facade-leak).
//
// `pub fn _helper` is `pub` (enforced-open) at L3, marked internal-by-convention
// via its leading-underscore name and absence from the crate-root facade →
// REQ-V4 over-exposure finding.
//
// `pub use internal::raw_helper` re-exports a `pub(crate)` (L2) function at
// the crate-root L4 facade → REQ-V7 facade-leak finding.
//
// This fixture is consumed by
// `crates/vampiro-seam-analysis/tests/modularity_e2e.rs`.

pub fn _helper() -> u32 {
    42
}

pub mod internal {
    pub(crate) fn raw_helper() -> u32 {
        0
    }
}

pub use internal::raw_helper;

pub fn public_api() -> u32 {
    _helper()
}
