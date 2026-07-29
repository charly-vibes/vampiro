//! Julia visibility extraction for Vampiro.
//!
//! All Julia functions are public by default (unlike Rust or Python).
//! `export` declarations are handled by facade metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use vampiro_cir::StableId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
    Public,
    Private,
}

pub fn extract_visibility(_source: &str) -> HashMap<StableId, Visibility> {
    HashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_serialization() {
        let vis = Visibility::Public;
        let json = serde_json::to_string(&vis).unwrap();
        assert_eq!(json, "\"public\"");
    }
}
