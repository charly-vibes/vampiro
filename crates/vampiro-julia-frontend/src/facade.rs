//! Julia facade metadata extraction (module re-exports).
//!
//! Extracts facade declarations from Julia `export` statements.

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

/// Extract facade metadata from a tree-sitter parsed Julia source.
pub fn extract_facade_metadata(root: Node, source: &str, path: &Path) -> FacadeMetadata {
    let file_path = path.to_string_lossy().to_string();
    let mut metadata = FacadeMetadata::new(&file_path);

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "module_definition" {
            let mut cc = child.walk();
            for c in child.children(&mut cc) {
                if c.kind() == "export_statement" {
                    let mut ccc = c.walk();
                    for ident in c.children(&mut ccc) {
                        if ident.kind() == "identifier" {
                            if let Some(name) = node_text(ident, source) {
                                metadata.facades.push(FacadeDecl {
                                    name,
                                    source_module: String::new(),
                                    kind: "export".to_string(),
                                    span: node_span(c, &file_path),
                                });
                            }
                        }
                    }
                } else if c.kind() == "expression_statement" {
                    let mut ccc = c.walk();
                    for expr in c.children(&mut ccc) {
                        if expr.kind() == "call_expression" {
                            let mut cccc = expr.walk();
                            let mut is_export = false;
                            let mut names = Vec::new();
                            for ident in expr.children(&mut cccc) {
                                if ident.kind() == "identifier" {
                                    if let Some(text) = node_text(ident, source) {
                                        if text == "export" {
                                            is_export = true;
                                        } else if is_export {
                                            names.push(text);
                                        }
                                    }
                                }
                            }
                            if is_export {
                                for name in names {
                                    metadata.facades.push(FacadeDecl {
                                        name,
                                        source_module: String::new(),
                                        kind: "re_export".to_string(),
                                        span: node_span(expr, &file_path),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    metadata
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
            .set_language(&tree_sitter_julia::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = unsafe { std::mem::transmute::<Node<'_>, Node<'static>>(tree.root_node()) };
        std::mem::forget(tree);
        TestParse { root }
    }

    #[test]
    fn empty_source() {
        let tp = parse_source("");
        let meta = extract_facade_metadata(tp.root, "", Path::new("lib.jl"));
        assert!(meta.facades.is_empty());
    }

    #[test]
    fn version_constant() {
        assert_eq!(FACADE_SCHEMA_VERSION, "0.1.0");
    }
}
