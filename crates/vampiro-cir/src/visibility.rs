//! Language-neutral visibility model (Addendum V, REQ-V1–V2).
//!
//! Front-ends classify every declaration into the visibility lattice and
//! boundary kind using that language's visibility idiom table. The analysis
//! layer consumes these facts to check modularity (REQ-8, REQ-V3–V4, REQ-V7,
//! REQ-C5) without depending on any particular language's syntax.

use crate::StableId;
use serde::{Deserialize, Serialize};

/// The visibility lattice levels (Addendum V, per-language table).
///
/// Ordered from least to most accessible. `L1Half` represents the `L1.5`
/// intermediate level (e.g. Rust `pub(super)`, Python name-mangling).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LatticeLevel {
    /// L0: private (lexical/local scope only).
    L0,
    /// L1: module-internal (visible within its own module, not exported).
    L1,
    /// L1.5: parent-module (visible to the parent module only).
    L1Half,
    /// L2: package-internal (visible within its own package/crate, not beyond).
    L2,
    /// L3: public-unstable (reachable from outside, but marked not-for-external-use).
    L3,
    /// L4: public-stable (part of the declared facade).
    L4,
}

impl LatticeLevel {
    /// Returns `true` if this level is part of the declared public facade
    /// (`L4`).
    pub fn is_facade(self) -> bool {
        matches!(self, LatticeLevel::L4)
    }

    /// Returns `true` if this level is hidden below the facade (`< L4`).
    pub fn is_hidden_below_facade(self) -> bool {
        self < LatticeLevel::L4
    }
}

impl std::fmt::Display for LatticeLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LatticeLevel::L0 => f.write_str("L0"),
            LatticeLevel::L1 => f.write_str("L1"),
            LatticeLevel::L1Half => f.write_str("L1.5"),
            LatticeLevel::L2 => f.write_str("L2"),
            LatticeLevel::L3 => f.write_str("L3"),
            LatticeLevel::L4 => f.write_str("L4"),
        }
    }
}

/// How a lattice level is enforced (Addendum V, boundary kind).
///
/// Only Rust has genuine `Enforced` boundaries above L0; Python, Clojure, and
/// Julia are `Advisory` at every level above local scope. `EnforcedOpen`
/// describes Rust `pub` — technically accessible, treated as a facade level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BoundaryKind {
    /// The language/runtime prevents access from outside the boundary.
    Enforced,
    /// The language permits access; only convention discourages it.
    Advisory,
    /// Technically enforced-open (accessible from outside) — used for Rust `pub`
    /// at L3/L4.
    EnforcedOpen,
}

impl std::fmt::Display for BoundaryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoundaryKind::Enforced => f.write_str("enforced"),
            BoundaryKind::Advisory => f.write_str("advisory"),
            BoundaryKind::EnforcedOpen => f.write_str("enforced-open"),
        }
    }
}

/// A per-node visibility classification (REQ-V1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilityFact {
    /// The node this fact applies to.
    pub node: StableId,
    /// The lattice level of this declaration.
    pub level: LatticeLevel,
    /// The boundary kind of this declaration.
    pub boundary: BoundaryKind,
    /// The module path / scope this node lives in (e.g. `"my_crate::module"`).
    pub scope: String,
    /// Whether this node is marked internal-by-convention (`#[doc(hidden)]`,
    /// leading-underscore name, or excluded from the crate root's `pub use`
    /// facade). Drives REQ-V4 over-exposure.
    pub internal_by_convention: bool,
}

/// A facade re-export entry (REQ-V7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacadeReexport {
    /// The facade module path (e.g. `""` for crate root).
    pub facade_scope: String,
    /// The re-exported symbol name.
    pub exported_name: String,
    /// The underlying declaration's node ID.
    pub underlying_node: StableId,
}

/// The complete visibility facts surface consumed by the modularity tracer.
///
/// Built by front-ends (or adapters) from language-specific extraction data.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilityFacts {
    /// The visibility idiom table version (REQ-V2).
    pub version: String,
    /// Per-node visibility classifications.
    pub facts: Vec<VisibilityFact>,
    /// Nesting edges: child scope → parent scope (module ancestry).
    pub nesting: Vec<(String, String)>,
    /// Facade re-exports at each module level.
    pub facades: Vec<FacadeReexport>,
}

impl VisibilityFacts {
    /// Construct an empty facts table with the given idiom-table version.
    pub fn new(version: impl Into<String>) -> Self {
        VisibilityFacts {
            version: version.into(),
            facts: Vec::new(),
            nesting: Vec::new(),
            facades: Vec::new(),
        }
    }

    /// Add a per-node visibility fact.
    pub fn add_fact(&mut self, fact: VisibilityFact) {
        self.facts.push(fact);
    }

    /// Add a nesting edge (child scope → parent scope).
    pub fn add_nesting(&mut self, child: impl Into<String>, parent: impl Into<String>) {
        self.nesting.push((child.into(), parent.into()));
    }

    /// Add a facade re-export.
    pub fn add_facade(&mut self, reexport: FacadeReexport) {
        self.facades.push(reexport);
    }

    /// Look up the visibility fact for a node.
    pub fn fact_for(&self, node: &StableId) -> Option<&VisibilityFact> {
        self.facts.iter().find(|f| &f.node == node)
    }

    /// Check if `caller_scope` can reach `target_scope` via nesting ancestors
    /// (i.e. `target_scope` is an ancestor of or equal to `caller_scope`).
    ///
    /// This is the nesting-generator portion of the legitimate subcategory 𝒢
    /// (REQ-C5).
    pub fn nesting_reachable(&self, caller_scope: &str, target_scope: &str) -> bool {
        if caller_scope == target_scope {
            return true;
        }
        // Walk the caller's ancestry; if we hit the target scope, the caller
        // is inside the target's scope.
        let mut current = caller_scope.to_string();
        let mut seen: Vec<String> = Vec::new();
        loop {
            if seen.iter().any(|s| s == &current) {
                return false; // cycle guard
            }
            seen.push(current.clone());
            let parent =
                self.nesting.iter().find_map(
                    |(c, p)| {
                        if c == &current {
                            Some(p.clone())
                        } else {
                            None
                        }
                    },
                );
            match parent {
                Some(p) if p == target_scope => return true,
                Some(p) => {
                    current = p;
                }
                None => return false,
            }
        }
    }

    /// Check if `target_node` is re-exported in a facade reachable from
    /// `caller_scope` via nesting.
    ///
    /// This is the facade/export-generator portion of 𝒢 (REQ-C5).
    pub fn facade_reachable(&self, caller_scope: &str, target_node: &StableId) -> bool {
        self.facades.iter().any(|f| {
            f.underlying_node == *target_node
                && self.nesting_reachable(caller_scope, &f.facade_scope)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(s: &str) -> StableId {
        StableId::new(s)
    }

    #[test]
    fn lattice_ordering() {
        assert!(LatticeLevel::L0 < LatticeLevel::L1);
        assert!(LatticeLevel::L1 < LatticeLevel::L1Half);
        assert!(LatticeLevel::L1Half < LatticeLevel::L2);
        assert!(LatticeLevel::L2 < LatticeLevel::L3);
        assert!(LatticeLevel::L3 < LatticeLevel::L4);
    }

    #[test]
    fn lattice_facade_predicates() {
        assert!(LatticeLevel::L4.is_facade());
        assert!(!LatticeLevel::L3.is_facade());
        assert!(LatticeLevel::L3.is_hidden_below_facade());
        assert!(!LatticeLevel::L4.is_hidden_below_facade());
    }

    #[test]
    fn nesting_reachable_same_scope() {
        let facts = VisibilityFacts::new("0.1.0");
        assert!(facts.nesting_reachable("a::b", "a::b"));
    }

    #[test]
    fn nesting_reachable_ancestor() {
        let mut facts = VisibilityFacts::new("0.1.0");
        facts.add_nesting("a::b", "a");
        facts.add_nesting("a", "");
        assert!(facts.nesting_reachable("a::b", "a"));
        assert!(facts.nesting_reachable("a::b", ""));
        assert!(!facts.nesting_reachable("a", "a::b"));
    }

    #[test]
    fn nesting_reachable_arbitrary_depth() {
        // REQ-C3: arbitrary filtration depth. A deeply nested caller can
        // reach a shallow target scope through many nesting edges.
        let mut facts = VisibilityFacts::new("0.1.0");
        facts.add_nesting("a::b::c::d::e", "a::b::c::d");
        facts.add_nesting("a::b::c::d", "a::b::c");
        facts.add_nesting("a::b::c", "a::b");
        facts.add_nesting("a::b", "a");
        facts.add_nesting("a", "");
        assert!(facts.nesting_reachable("a::b::c::d::e", "a"));
        assert!(facts.nesting_reachable("a::b::c::d::e", "a::b"));
    }

    #[test]
    fn nesting_reachable_cycle_guard() {
        let mut facts = VisibilityFacts::new("0.1.0");
        // Malformed nesting with a cycle — must not hang.
        facts.add_nesting("x", "y");
        facts.add_nesting("y", "x");
        assert!(!facts.nesting_reachable("x", "z"));
    }

    #[test]
    fn facade_reachable_via_nesting() {
        let mut facts = VisibilityFacts::new("0.1.0");
        facts.add_nesting("a::b", "a");
        facts.add_nesting("a", "");
        facts.add_facade(FacadeReexport {
            facade_scope: "".into(),
            exported_name: "foo".into(),
            underlying_node: sid("node-foo"),
        });
        // A caller in a::b can reach the crate-root facade.
        assert!(facts.facade_reachable("a::b", &sid("node-foo")));
        // A caller in a different tree cannot.
        assert!(!facts.facade_reachable("other::tree", &sid("node-foo")));
    }
}
