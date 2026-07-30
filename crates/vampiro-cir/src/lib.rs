//! Vampiro Composition IR (CIR) — the single boundary between syntax plugins
//! and language-neutral analysis.
//!
//! CIR types represent nodes (callables/declarations) and edges (call sites)
//! extracted from source files by front-end plugins. The graph is
//! language-neutral and carries effect channels, shapes, provenance, and
//! stable identities.
//!
//! # Frontend contract
//!
//! Every frontend implements the [`Frontend`] trait. The platform verifies
//! conformance before any graph is used for analysis.
//!
//! # Resource limits
//!
//! - Effect channels: max [`MAX_EFFECT_DEPTH`](effect::MAX_EFFECT_DEPTH) nesting
//! - Shapes: max [`MAX_SHAPE_DEPTH`](shape::MAX_SHAPE_DEPTH) nesting
//!
//! Graphs exceeding these limits are rejected at construction time.

pub mod category;
pub mod cir;
pub mod effect;
pub mod error;
pub mod frontend;
pub mod provenance;
pub mod shape;
pub mod visibility;

pub use category::{
    validate_category, validate_filtration, CategoryDecl, FiltrationDecl, FiltrationLevel,
    MorphismDecl, MorphismId, ValidatedCategory, ValidationError,
};
pub use cir::{CirEdge, CirGraph, CirNode, NodeKind};
pub use effect::{EffectChannel, EffectResolution, Totality, UnwrapEvidence, UnwrapKind};
pub use error::CirError;
pub use frontend::Frontend;
pub use provenance::{
    DiscardSpan, Provenance, SourceSpan, StableId, TrustProvenance, ValidationObservation,
};
pub use shape::{ScalarKind, Shape};
pub use visibility::{BoundaryKind, FacadeReexport, LatticeLevel, VisibilityFact, VisibilityFacts};
