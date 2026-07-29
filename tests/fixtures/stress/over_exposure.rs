// Seeded stress fixture — OVER-EXPOSURE (REQ-V4).
//
// `_internal` is a `pub fn` marked `#[doc(hidden)]` (internal-by-convention)
// at lattice level L3 (EnforcedOpen). It is reachable from outside its
// declaring file via a cross-file caller — the problem is that the item is
// reachable at all, not that a caller reached it improperly.
//
// NOTE: over-exposure requires a cross-file caller (vampiro-6ty), which the
// single-file Rust frontend extraction cannot represent. The soundness
// assertion is therefore pinned via a hand-built CIR graph + visibility facts
// in the test. This source file documents the defect pattern.

#[doc(hidden)]
pub fn _internal() -> u32 {
    42
}
