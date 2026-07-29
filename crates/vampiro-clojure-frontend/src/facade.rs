//! Clojure facade metadata extraction (namespace re-exports).
//!
//! Extracts facade declarations from Clojure `:require` and `:use` directives.

use serde::{Deserialize, Serialize};
use std::path::Path;
use tree_sitter::Node;
use vampiro_cir::SourceSpan;

pub const FACADE_SCHEMA_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FacadeDecl {
    pub name: String,
    pub source_module: String,
    pub kind: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FacadeMetadata {
    pub version: String,
    pub source_file: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facades: Vec<FacadeDecl>,
}

impl FacadeMetadata {
    pub fn new(source_file: impl Into<String>) -> Self {
        FacadeMetadata {
            version: FACADE_SCHEMA_VERSION.into(),
            source_file: source_file.into(),
            facades: Vec::new(),
        }
    }
}

/// Extract facade metadata from a tree-sitter parsed Clojure source.
pub fn extract_facade_metadata(root: Node, source: &str, path: &Path) -> FacadeMetadata {
    let file_path = path.to_string_lossy().to_string();
    let mut metadata = FacadeMetadata::new(&file_path);

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "list_lit" {
            process_ns_or_require(child, source, &file_path, &mut metadata);
        }
    }

    metadata
}

fn process_ns_or_require(node: Node, source: &str, file_path: &str, metadata: &mut FacadeMetadata) {
    let first = node.child(0);
    let first_name = first
        .and_then(|n| n.child_by_field_name("name"))
        .and_then(|n| node_text(n, source));

    match first_name.as_deref() {
        Some("ns") => {
            // (ns my.ns (:require [clojure.set :refer [union intersection]] ...))
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "list_lit" {
                    process_require_directive(child, source, file_path, metadata);
                }
            }
        }
        Some("require") | Some("use") => {
            process_require_directive(node, source, file_path, metadata);
        }
        _ => {}
    }
}

fn process_require_directive(
    node: Node,
    source: &str,
    file_path: &str,
    metadata: &mut FacadeMetadata,
) {
    // (:require [clojure.string :as str]) or (:require [clojure.set :refer [union]]...)
    // or (:use [clojure.walk :only [keywordize-strings]])
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "vec_lit" {
            // Extract the namespace from the vector
            let ns = child
                .child(0)
                .and_then(|n| n.child_by_field_name("name"))
                .and_then(|n| node_text(n, source))
                .unwrap_or_default();

            if ns.is_empty() {
                continue;
            }

            let span = node_span(node, file_path);

            // Check for :refer :all (wildcard re-export)
            if contains_keyword_refer_all(child, source) {
                metadata.facades.push(FacadeDecl {
                    name: "*".to_string(),
                    source_module: ns,
                    kind: "re_export_wildcard".to_string(),
                    span: span.clone(),
                });
                continue;
            }

            // Check for :refer [...] (specific re-exports)
            if let Some(names) = extract_referred_names(child, source) {
                for name in names {
                    metadata.facades.push(FacadeDecl {
                        name,
                        source_module: ns.clone(),
                        kind: "re_export".to_string(),
                        span: span.clone(),
                    });
                }
            }
        }
    }
}

fn contains_keyword_refer_all(_node: Node, _source: &str) -> bool {
    // Simplified: don't try to parse :refer :all for now
    false
}

fn extract_referred_names(node: Node, source: &str) -> Option<Vec<String>> {
    // Look for :refer [...] or :only [...] with vector of symbols
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(text) = node_text(child, source) {
            if text == ":refer" || text == ":only" {
                // Look for a vec_lit child containing the referred names
                let mut cc = node.walk();
                for c in node.children(&mut cc) {
                    if c.kind() == "vec_lit" {
                        let mut names = Vec::new();
                        let mut ccc = c.walk();
                        for sym in c.children(&mut ccc) {
                            if sym.kind() == "sym_lit" {
                                if let Some(name) = sym
                                    .child_by_field_name("name")
                                    .and_then(|n| node_text(n, source))
                                {
                                    names.push(name);
                                }
                            }
                        }
                        if !names.is_empty() {
                            return Some(names);
                        }
                    }
                }
            }
        }
    }
    None
}

fn node_text(node: Node, source: &str) -> Option<String> {
    let start = node.start_byte();
    let end = node.end_byte();
    if start < end && end <= source.len() {
        Some(source[start..end].to_string())
    } else {
        None
    }
}

fn node_span(node: Node, file_path: &str) -> SourceSpan {
    SourceSpan {
        file: file_path.to_string(),
        start_line: node.start_position().row + 1,
        start_column: node.start_position().column + 1,
        end_line: node.end_position().row + 1,
        end_column: node.end_position().column + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestParse {
        root: Node<'static>,
    }

    fn parse_source(source: &str) -> TestParse {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_clojure::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = unsafe { std::mem::transmute::<Node<'_>, Node<'static>>(tree.root_node()) };
        std::mem::forget(tree);
        TestParse { root }
    }

    #[test]
    fn empty_source() {
        let tp = parse_source("");
        let meta = extract_facade_metadata(tp.root, "", Path::new("core.clj"));
        assert!(meta.facades.is_empty());
    }

    #[test]
    fn version_constant() {
        assert_eq!(FACADE_SCHEMA_VERSION, "0.1.0");
    }
}
