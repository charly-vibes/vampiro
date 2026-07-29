//! Clojure visibility extraction for Vampiro.
//!
//! Extracts visibility levels from Clojure source:
//! - `defn` → Public
//! - `defn-` → Private

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use vampiro_cir::StableId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
    Public,
    Private,
}

impl Visibility {
    pub fn from_form_name(form_name: &str) -> Self {
        if form_name.ends_with('-') {
            Visibility::Private
        } else {
            Visibility::Public
        }
    }
}

pub fn extract_visibility(_source: &str) -> HashMap<StableId, Visibility> {
    HashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defn_is_public() {
        assert_eq!(Visibility::from_form_name("defn"), Visibility::Public);
    }

    #[test]
    fn defn_with_dash_is_private() {
        assert_eq!(Visibility::from_form_name("defn-"), Visibility::Private);
    }

    #[test]
    fn visibility_serialization() {
        let vis = Visibility::Public;
        let json = serde_json::to_string(&vis).unwrap();
        assert_eq!(json, "\"public\"");
    }
}
