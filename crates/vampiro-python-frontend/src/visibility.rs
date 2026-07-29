//! Python visibility extraction for Vampiro.
//!
//! Extracts visibility levels from Python source based on naming conventions:
//! - Public: no leading underscore
//! - Protected: single leading underscore `_`
//! - Private: double leading underscore `__`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use vampiro_cir::StableId;

/// Visibility level for Python declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
    /// Public — no leading underscore.
    Public,
    /// Protected — single leading underscore `_`.
    Protected,
    /// Private — double leading underscore `__`.
    Private,
}

impl Visibility {
    /// Infer visibility from a Python identifier name.
    pub fn from_name(name: &str) -> Self {
        if name.starts_with("__") && name.ends_with("__") && name.len() > 4 {
            // Dunder methods like __init__, __str__ are protocol hooks,
            // not name-mangled. They are considered public.
            Visibility::Public
        } else if name.starts_with("__") {
            // Name-mangled private attribute: starts with __ but doesn't
            // end with __ (e.g. __private, __impl).
            Visibility::Private
        } else if name.starts_with('_') {
            Visibility::Protected
        } else {
            Visibility::Public
        }
    }
}

#[allow(dead_code)]
/// Extract visibility for all named nodes in a CIR graph.
pub fn extract_visibility(graph: &vampiro_cir::CirGraph) -> HashMap<StableId, Visibility> {
    let mut vis = HashMap::new();
    for node in &graph.nodes {
        if let Some(name) = &node.name {
            vis.insert(node.id.clone(), Visibility::from_name(name));
        }
    }
    vis
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_name() {
        assert_eq!(Visibility::from_name("foo"), Visibility::Public);
        assert_eq!(Visibility::from_name("Helper"), Visibility::Public);
        assert_eq!(Visibility::from_name("process_data"), Visibility::Public);
    }

    #[test]
    fn protected_name() {
        assert_eq!(Visibility::from_name("_internal"), Visibility::Protected);
        assert_eq!(Visibility::from_name("_helper"), Visibility::Protected);
    }

    #[test]
    fn private_name() {
        assert_eq!(Visibility::from_name("__private"), Visibility::Private);
        assert_eq!(Visibility::from_name("__impl"), Visibility::Private);
    }

    #[test]
    fn dunder_method_is_public() {
        // __init__, __str__, etc. are protocol hooks, not name-mangled.
        assert_eq!(Visibility::from_name("__init__"), Visibility::Public);
        assert_eq!(Visibility::from_name("__str__"), Visibility::Public);
        assert_eq!(Visibility::from_name("__repr__"), Visibility::Public);
    }

    #[test]
    fn private_name_is_not_dunder() {
        // __private starts with __ but doesn't end with __ — name-mangled.
        assert_eq!(Visibility::from_name("__private"), Visibility::Private);
        assert_eq!(Visibility::from_name("__impl"), Visibility::Private);
    }

    #[test]
    fn single_underscore_is_protected() {
        assert_eq!(Visibility::from_name("_"), Visibility::Protected);
    }

    #[test]
    fn visibility_serialization() {
        let vis = Visibility::Public;
        let json = serde_json::to_string(&vis).unwrap();
        assert_eq!(json, "\"public\"");
    }

    #[test]
    fn visibility_deserialization() {
        let vis: Visibility = serde_json::from_str("\"private\"").unwrap();
        assert_eq!(vis, Visibility::Private);
    }
}
