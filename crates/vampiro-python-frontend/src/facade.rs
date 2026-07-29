//! Python facade metadata extraction (__init__.py re-exports).
//!
//! Extracts facade declarations from Python `__init__.py` files — re-exported
//! names from imported submodules. This follows the same pattern as the Rust
//! frontend's facade extraction but adapted for Python's import system.

use serde::{Deserialize, Serialize};
use std::path::Path;
use tree_sitter::Node;
use vampiro_cir::SourceSpan;

/// The facade metadata schema version.
pub const FACADE_SCHEMA_VERSION: &str = "0.1.0";

/// A facade declaration — a re-exported symbol from a submodule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FacadeDecl {
    /// The exported name.
    pub name: String,
    /// The source module (e.g. ".core", ".utils").
    pub source_module: String,
    /// The kind of declaration (e.g. "re_export").
    pub kind: String,
    /// Source span of the import statement.
    pub span: SourceSpan,
}

/// All facade metadata extracted from a single Python source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FacadeMetadata {
    /// Schema version.
    pub version: String,
    /// The source file path.
    pub source_file: String,
    /// Facade declarations.
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

/// Extract facade metadata from a tree-sitter parsed Python source.
pub fn extract_facade_metadata(root: Node, source: &str, path: &Path) -> FacadeMetadata {
    let file_path = path.to_string_lossy().to_string();
    let mut metadata = FacadeMetadata::new(&file_path);

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "import_from_statement" => {
                process_import_from(child, source, &file_path, &mut metadata);
            }
            "import_statement" => {
                process_import(child, source, &file_path, &mut metadata);
            }
            _ => {}
        }
    }

    metadata
}

/// Process a `from X import Y` statement.
fn process_import_from(node: Node, source: &str, file_path: &str, metadata: &mut FacadeMetadata) {
    let module = node
        .child_by_field_name("module_name")
        .and_then(|n| node_text(n, source))
        .unwrap_or_default();

    if module.is_empty() {
        return;
    }

    // Tree-sitter-python stores imported names as `dotted_name` children
    // directly under `import_from_statement`. Each is an imported symbol.
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "dotted_name" {
            // Ensure this is an imported name, not the module name.
            // The module_name field child is already captured above.
            if let Some(name) = node_text(child, source) {
                // Only add if the name doesn't contain dots (simple name)
                if !name.contains('.') {
                    metadata.facades.push(FacadeDecl {
                        name,
                        source_module: module.clone(),
                        kind: "re_export".to_string(),
                        span: node_span(node, file_path),
                    });
                }
            }
        }
    }
}

/// Process a `import X` statement at module level.
fn process_import(node: Node, source: &str, file_path: &str, metadata: &mut FacadeMetadata) {
    // For __init__.py files, `import X` at module level is also a re-export.
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "dotted_name" {
            if let Some(name) = node_text(child, source) {
                metadata.facades.push(FacadeDecl {
                    name,
                    source_module: String::new(),
                    kind: "re_export".to_string(),
                    span: node_span(node, file_path),
                });
            }
        }
    }
}

/// Get the text content of a tree-sitter node.
fn node_text(node: Node, source: &str) -> Option<String> {
    let start = node.start_byte();
    let end = node.end_byte();
    if start < end && end <= source.len() {
        Some(source[start..end].to_string())
    } else {
        None
    }
}

/// Create a source span from a tree-sitter node.
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

    /// Parse source and return a root node with an owned tree reference.
    /// The tree is leaked for 'static lifetime — acceptable in test scope.
    struct TestParse {
        root: Node<'static>,
    }

    fn parse_source(source: &str) -> TestParse {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = unsafe {
            // Safety: tree is leaked, root_node borrow is valid for 'static.
            std::mem::transmute::<Node<'_>, Node<'static>>(tree.root_node())
        };
        std::mem::forget(tree);
        TestParse { root }
    }

    #[test]
    fn empty_source_no_facades() {
        let tp = parse_source("");
        let meta = extract_facade_metadata(tp.root, "", Path::new("__init__.py"));
        assert!(meta.facades.is_empty());
    }

    #[test]
    fn from_import_detects_re_exports() {
        let source = "from .core import run, configure\nfrom .utils import Helper, format_result";
        let tp = parse_source(source);
        let meta = extract_facade_metadata(tp.root, source, Path::new("__init__.py"));
        assert_eq!(meta.facades.len(), 4);
        assert_eq!(meta.facades[0].name, "run");
        assert_eq!(meta.facades[0].source_module, ".core");
        assert_eq!(meta.facades[0].kind, "re_export");
        assert_eq!(meta.facades[1].name, "configure");
        assert_eq!(meta.facades[1].source_module, ".core");
        assert_eq!(meta.facades[2].name, "Helper");
        assert_eq!(meta.facades[2].source_module, ".utils");
        assert_eq!(meta.facades[3].name, "format_result");
        assert_eq!(meta.facades[3].source_module, ".utils");
    }

    #[test]
    fn simple_import_at_module_level() {
        let source = "import os\nimport sys";
        let tp = parse_source(source);
        let meta = extract_facade_metadata(tp.root, source, Path::new("__init__.py"));
        assert_eq!(meta.facades.len(), 2);
        assert_eq!(meta.facades[0].name, "os");
        assert_eq!(meta.facades[1].name, "sys");
    }

    #[test]
    fn mixed_imports() {
        let source = "import os\nfrom .types import Result, Config\nimport sys";
        let tp = parse_source(source);
        let meta = extract_facade_metadata(tp.root, source, Path::new("__init__.py"));
        assert_eq!(meta.facades.len(), 4);
    }

    #[test]
    fn version_constant_is_set() {
        assert_eq!(FACADE_SCHEMA_VERSION, "0.1.0");
    }

    #[test]
    fn metadata_has_source_file() {
        let tp = parse_source("");
        let meta = extract_facade_metadata(tp.root, "", Path::new("__init__.py"));
        assert_eq!(meta.source_file, "__init__.py");
    }

    #[test]
    fn span_is_correct() {
        let source = "from .core import run";
        let tp = parse_source(source);
        let meta = extract_facade_metadata(tp.root, source, Path::new("__init__.py"));
        assert_eq!(meta.facades.len(), 1);
        let span = &meta.facades[0].span;
        assert_eq!(span.file, "__init__.py");
        assert_eq!(span.start_line, 1);
        assert_eq!(span.start_column, 1);
    }

    #[test]
    fn non_init_file_still_processed() {
        let source = "from .utils import helper";
        let tp = parse_source(source);
        let meta = extract_facade_metadata(tp.root, source, Path::new("lib.py"));
        assert_eq!(meta.facades.len(), 1);
        assert_eq!(meta.facades[0].name, "helper");
    }
}
