//! Julia law runner-input extraction for Vampiro.
//!
//! Extracts function declarations with their parameters as tagged functions.
//!
//! Julia grammar:
//! - function_definition → signature → call_expression → identifier (name)
//! - signature → argument_list → identifiers (params)

use serde::{Deserialize, Serialize};
use std::path::Path;
use tree_sitter::Node;
use vampiro_cir::SourceSpan;

pub const RUNNER_INPUT_SCHEMA_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LawRunnerInput {
    pub version: String,
    pub source_file: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tagged_fns: Vec<TaggedFn>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub serializable_values: Vec<SerializableValue>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generator_refs: Vec<GeneratorRef>,
}

impl LawRunnerInput {
    pub fn new(source_file: impl Into<String>) -> Self {
        LawRunnerInput {
            version: RUNNER_INPUT_SCHEMA_VERSION.into(),
            source_file: source_file.into(),
            tagged_fns: Vec::new(),
            serializable_values: Vec::new(),
            generator_refs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TaggedFn {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<FnParam>,
    pub return_type: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FnParam {
    pub name: String,
    pub type_name: String,
    pub is_serializable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SerializableValue {
    pub name: String,
    pub type_name: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GeneratorRef {
    pub name: String,
    pub kind: String,
    pub span: SourceSpan,
}

/// Extract law runner-input from a tree-sitter parsed Julia source.
pub fn extract_law_input(root: Node, source: &str, path: &Path) -> LawRunnerInput {
    let file_path = path.to_string_lossy().to_string();
    let mut input = LawRunnerInput::new(&file_path);

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        process_top_level(child, source, &file_path, &mut input);
    }

    input
}

fn process_top_level(node: Node, source: &str, file_path: &str, input: &mut LawRunnerInput) {
    match node.kind() {
        "function_definition" => process_function(node, source, file_path, input),
        "module_definition" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                process_top_level(child, source, file_path, input);
            }
        }
        _ => {}
    }
}

fn process_function(node: Node, source: &str, file_path: &str, input: &mut LawRunnerInput) {
    let name = extract_function_name(node, source).unwrap_or_default();
    if name.is_empty() {
        return;
    }

    let span = node_span(node, file_path);
    let params = extract_params(node, source);

    input.tagged_fns.push(TaggedFn {
        name: name.clone(),
        tags: Vec::new(),
        params,
        return_type: String::new(),
        span,
    });

    // No generator detection for Julia v1 (Channel is too complex to detect statically)
}

fn extract_function_name(node: Node, source: &str) -> Option<String> {
    // function_definition → signature → call_expression → identifier
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "signature" {
            let mut cc = child.walk();
            for c in child.children(&mut cc) {
                if c.kind() == "call_expression" {
                    // First identifier child is the function name
                    let mut ccc = c.walk();
                    for ident in c.children(&mut ccc) {
                        if ident.kind() == "identifier" {
                            return node_text(ident, source);
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_params(node: Node, source: &str) -> Vec<FnParam> {
    // function_definition → signature → call_expression → argument_list → identifiers
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "signature" {
            let mut cc = child.walk();
            for c in child.children(&mut cc) {
                if c.kind() == "call_expression" {
                    let mut ccc = c.walk();
                    for arg in c.children(&mut ccc) {
                        if arg.kind() == "argument_list" {
                            let mut params = Vec::new();
                            let mut cccc = arg.walk();
                            for p in arg.children(&mut cccc) {
                                if p.kind() == "identifier" {
                                    if let Some(name) = node_text(p, source) {
                                        params.push(FnParam {
                                            name,
                                            type_name: String::new(),
                                            is_serializable: false,
                                        });
                                    }
                                }
                            }
                            return params;
                        }
                    }
                }
            }
        }
    }
    Vec::new()
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
        let input = extract_law_input(tp.root, "", Path::new("empty.jl"));
        assert!(input.tagged_fns.is_empty());
    }

    #[test]
    fn simple_function() {
        let source = "function add(a, b)\n    return a + b\nend";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("lib.jl"));
        assert_eq!(input.tagged_fns.len(), 1);
        assert_eq!(input.tagged_fns[0].name, "add");
        assert_eq!(input.tagged_fns[0].params.len(), 2);
    }

    #[test]
    fn multiple_functions() {
        let source = "function a(x) x end\nfunction b(y) y end";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("lib.jl"));
        assert_eq!(input.tagged_fns.len(), 2);
    }

    #[test]
    fn version_constant() {
        assert_eq!(RUNNER_INPUT_SCHEMA_VERSION, "0.1.0");
    }
}
