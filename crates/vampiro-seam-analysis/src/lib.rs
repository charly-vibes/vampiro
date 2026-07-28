//! Language-neutral seam analysis for Vampiro.
//!
//! Consumes a [`CirGraph`] (and, eventually, visibility/facade/law/lifecycle
//! contract data) and emits spec-conformant findings along exactly the four
//! EARS axes: `composition`, `modularity`, `optionality`, `robustness`.
//!
//! This crate owns the **normalized finding contract** (REQ-4, v1.3.0): the
//! closed axis set, the `LOW`/`MEDIUM`/`HIGH` severity vocabulary, the
//! per-rule default severities, and the rule-specific evidence payloads. The
//! contract is formally published (with a compatibility fixture) at task
//! `0vb.4.6`; until then it lives here as the in-progress analysis surface.
//!
//! # Current scope
//!
//! Only the **composition tracer** (REQ-7, REQ-23) is implemented. The
//! modularity, optionality, and robustness tracers are delivered by tasks
//! `0vb.4.3`–`0vb.4.5`.

use vampiro_cir::CirGraph;

pub mod composition;
pub mod finding;

pub use composition::{unify_shapes, CompositionAnalyzer, Unification};
pub use finding::{Axis, Evidence, Finding, Severity};

/// Run the currently-implemented analysis slices over `graph` and return the
/// findings.
///
/// Only composition is implemented today; modularity/optionality/robustness
/// are added by later tasks in `add-core-seam-analysis`.
pub fn analyze(graph: &CirGraph) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(CompositionAnalyzer::new().analyze(graph));
    findings
}
