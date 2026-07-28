//! Rust visibility and facade extraction for Vampiro.
//!
//! This module provides types and extraction logic for:
//! - Visibility levels (public, crate, super, restricted, private)
//! - Facade metadata (crate-root re-exports)
//! - Module ancestry tracking
//!
//! Visibility and effect idiom tables are independently versioned per REQ-V1–V2.

use serde::{Deserialize, Serialize};

/// The visibility level of a Rust declaration.
///
/// Maps to Rust's `pub` forms. Independently versioned from effect idioms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
    /// `pub` — visible to all crates.
    Public,
    /// `pub(crate)` — visible within the current crate.
    Crate,
    /// `pub(super)` — visible within the parent module.
    Super,
    /// `pub(in path)` — visible within a specific path.
    Restricted(String),
    /// No `pub` — private (inherited visibility).
    Private,
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Visibility::Public => f.write_str("pub"),
            Visibility::Crate => f.write_str("pub(crate)"),
            Visibility::Super => f.write_str("pub(super)"),
            Visibility::Restricted(path) => write!(f, "pub(in {path})"),
            Visibility::Private => f.write_str("private"),
        }
    }
}

impl Visibility {
    /// The version of the visibility idiom table.
    pub const TABLE_VERSION: &'static str = "0.1.0";

    /// Returns `true` if this visibility is public to external crates.
    pub fn is_public(&self) -> bool {
        matches!(self, Visibility::Public)
    }

    /// Returns `true` if this visibility is at least crate-visible.
    pub fn is_at_least_crate(&self) -> bool {
        matches!(
            self,
            Visibility::Public | Visibility::Crate | Visibility::Super | Visibility::Restricted(_)
        )
    }
}

impl From<&syn::Visibility> for Visibility {
    fn from(vis: &syn::Visibility) -> Self {
        match vis {
            syn::Visibility::Public(_) => Visibility::Public,
            syn::Visibility::Restricted(restricted) => {
                // Extract the path from pub(in path), pub(crate), pub(super), pub(self)
                if restricted.in_token.is_some() {
                    // pub(in path)
                    let path = restricted
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    Visibility::Restricted(path)
                } else {
                    // pub(crate), pub(super), pub(self)
                    let ident = restricted
                        .path
                        .get_ident()
                        .map(|i| i.to_string())
                        .unwrap_or_else(|| "self".to_string());
                    match ident.as_str() {
                        "crate" => Visibility::Crate,
                        "super" => Visibility::Super,
                        "self" => Visibility::Private, // pub(self) = private
                        _ => Visibility::Restricted(ident),
                    }
                }
            }
            syn::Visibility::Inherited => Visibility::Private,
        }
    }
}

/// A facade entry: a re-exported item at the crate root or module boundary.
///
/// Facades are the public API surface of a crate. Tracking them enables
/// analysis of what a crate intentionally exposes vs. internal implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FacadeEntry {
    /// The name of the re-exported item.
    pub name: String,
    /// The path to the original definition.
    pub original_path: String,
    /// Whether this is a wildcard re-export (`pub use module::*`).
    pub is_wildcard: bool,
    /// The visibility of the re-export.
    pub visibility: Visibility,
    /// The source span of the re-export.
    pub span: SourceSpan,
    /// Whether the re-export is marked `#[doc(hidden)]`.
    pub doc_hidden: bool,
}

/// A facade declaration: the set of re-exports at a module level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FacadeDecl {
    /// The version of the facade metadata schema.
    pub version: String,
    /// The module path (e.g., `""` for crate root, `"foo::bar"` for nested).
    pub module_path: String,
    /// All re-export entries at this level.
    pub entries: Vec<FacadeEntry>,
}

impl FacadeDecl {
    /// Create a new facade declaration for a module path.
    pub fn new(module_path: impl Into<String>) -> Self {
        FacadeDecl {
            version: "0.1.0".into(),
            module_path: module_path.into(),
            entries: Vec::new(),
        }
    }

    /// Add a re-export entry.
    pub fn add_entry(&mut self, entry: FacadeEntry) {
        self.entries.push(entry);
    }
}

use vampiro_cir::SourceSpan;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_from_syn_pub() {
        let syn_vis: syn::Visibility = syn::parse_quote!(pub);
        let vis: Visibility = (&syn_vis).into();
        assert_eq!(vis, Visibility::Public);
    }

    #[test]
    fn visibility_from_syn_crate() {
        let syn_vis: syn::Visibility = syn::parse_quote!(pub(crate));
        let vis: Visibility = (&syn_vis).into();
        assert_eq!(vis, Visibility::Crate);
    }

    #[test]
    fn visibility_from_syn_super() {
        let syn_vis: syn::Visibility = syn::parse_quote!(pub(super));
        let vis: Visibility = (&syn_vis).into();
        assert_eq!(vis, Visibility::Super);
    }

    #[test]
    fn visibility_from_syn_private() {
        let syn_vis: syn::Visibility = syn::Visibility::Inherited;
        let vis: Visibility = (&syn_vis).into();
        assert_eq!(vis, Visibility::Private);
    }

    #[test]
    fn visibility_from_syn_restricted() {
        let syn_vis: syn::Visibility = syn::parse_quote!(pub(in foo::bar));
        let vis: Visibility = (&syn_vis).into();
        assert_eq!(vis, Visibility::Restricted("foo::bar".into()));
    }

    #[test]
    fn visibility_is_public() {
        assert!(Visibility::Public.is_public());
        assert!(!Visibility::Crate.is_public());
        assert!(!Visibility::Private.is_public());
    }

    #[test]
    fn visibility_is_at_least_crate() {
        assert!(Visibility::Public.is_at_least_crate());
        assert!(Visibility::Crate.is_at_least_crate());
        assert!(Visibility::Super.is_at_least_crate());
        assert!(Visibility::Restricted("foo".into()).is_at_least_crate());
        assert!(!Visibility::Private.is_at_least_crate());
    }

    #[test]
    fn visibility_display() {
        assert_eq!(Visibility::Public.to_string(), "pub");
        assert_eq!(Visibility::Crate.to_string(), "pub(crate)");
        assert_eq!(Visibility::Super.to_string(), "pub(super)");
        assert_eq!(
            Visibility::Restricted("foo::bar".into()).to_string(),
            "pub(in foo::bar)"
        );
        assert_eq!(Visibility::Private.to_string(), "private");
    }

    #[test]
    fn facade_decl_creation() {
        let decl = FacadeDecl::new("");
        assert_eq!(decl.version, "0.1.0");
        assert_eq!(decl.module_path, "");
        assert!(decl.entries.is_empty());
    }

    #[test]
    fn facade_decl_add_entry() {
        let mut decl = FacadeDecl::new("foo");
        decl.add_entry(FacadeEntry {
            name: "bar".into(),
            original_path: "foo::bar".into(),
            is_wildcard: false,
            visibility: Visibility::Public,
            span: SourceSpan {
                file: "lib.rs".into(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 10,
            },
            doc_hidden: false,
        });
        assert_eq!(decl.entries.len(), 1);
        assert_eq!(decl.entries[0].name, "bar");
    }

    #[test]
    fn facade_entry_serialization() {
        let entry = FacadeEntry {
            name: "foo".into(),
            original_path: "crate::foo".into(),
            is_wildcard: false,
            visibility: Visibility::Public,
            span: SourceSpan {
                file: "lib.rs".into(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 10,
            },
            doc_hidden: false,
        };
        let json = serde_json::to_string_pretty(&entry).unwrap();
        assert!(json.contains(r#""name""#));
        assert!(json.contains(r#""original-path""#));
        assert!(json.contains(r#""is-wildcard""#));
        assert!(json.contains(r#""visibility""#));
        assert!(json.contains(r#""doc-hidden""#));
    }

    #[test]
    fn visibility_table_version() {
        assert_eq!(Visibility::TABLE_VERSION, "0.1.0");
    }
}
