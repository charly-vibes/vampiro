//! Finding contract — re-exported from `vampiro-seam-analysis`.
//!
//! The EARS-conformant finding types (REQ-4, v1.3.0) live in
//! `vampiro-seam-analysis::finding`. This module re-exports them so the CLI
//! uses the single, authoritative finding contract rather than a duplicate.
//!
//! The closed axis set is `{composition, modularity, optionality, robustness}`
//! and the closed severity vocabulary is `{low, medium, high}`.

pub use vampiro_seam_analysis::finding::{
    Axis, Diagnostic, Evidence, Finding, LineRange, Severity,
};
