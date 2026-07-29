//! Clojure law runner-input extraction for Vampiro.
//!
//! Extracts defn declarations with their parameters as tagged functions.
//!
//! Tree-sitter-clojure child indexing:
//! - list_lit child[0] = `(`, child[1] = first element, child[2] = second, etc.
//! - sym_lit field `name` = the symbol name text (e.g. "defn", "add")

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

/// Extract law runner-input from a tree-sitter parsed Clojure source.
pub fn extract_law_input(root: Node, source: &str, path: &Path) -> LawRunnerInput {
    let file_path = path.to_string_lossy().to_string();
    let mut input = LawRunnerInput::new(&file_path);

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "list_lit" {
            process_defn_form(child, source, &file_path, &mut input);
        }
    }

    input
}

fn process_defn_form(node: Node, source: &str, file_path: &str, input: &mut LawRunnerInput) {
    // child[0] = '(', child[1] = operator symbol, child[2] = name, child[3] = params, ...
    let first_op = node.child(1);
    let op_name = first_op.and_then(|n| get_sym_name(n, source));

    if op_name.as_deref() != Some("defn") && op_name.as_deref() != Some("defn-") {
        return;
    }

    let name_node = node.child(2);
    let name = name_node
        .and_then(|n| get_sym_name(n, source))
        .unwrap_or_default();

    if name.is_empty() {
        return;
    }

    let span = node_span(node, file_path);

    // Find params vector — skip ( and operator and name
    let params = extract_params_from_defn(node, source);

    // Check for lazy-seq in body
    let has_generator = form_contains_sym(node, source, "lazy-seq");

    input.tagged_fns.push(TaggedFn {
        name: name.clone(),
        tags: Vec::new(),
        params,
        return_type: String::new(),
        span: span.clone(),
    });

    if has_generator {
        input.generator_refs.push(GeneratorRef {
            name,
            kind: "generator".to_string(),
            span,
        });
    }
}

fn get_sym_name(node: Node, source: &str) -> Option<String> {
    // sym_lit has a `name` field giving the symbol text
    if node.kind() == "sym_lit" {
        node.child_by_field_name("name")
            .and_then(|n| node_text(n, source))
            .or_else(|| node_text(node, source))
    } else {
        node_text(node, source)
    }
}

fn extract_params_from_defn(node: Node, source: &str) -> Vec<FnParam> {
    // Scan children looking for vec_lit (params vector)
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "vec_lit" {
            let mut params = Vec::new();
            let mut cc = child.walk();
            for p in child.children(&mut cc) {
                if p.kind() == "sym_lit" {
                    if let Some(name) = get_sym_name(p, source) {
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
    Vec::new()
}

fn form_contains_sym(node: Node, source: &str, target: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "sym_lit" {
            if let Some(name) = get_sym_name(child, source) {
                if name == target {
                    return true;
                }
            }
        }
        if child.child_count() > 0 && form_contains_sym(child, source, target) {
            return true;
        }
    }
    false
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
        let input = extract_law_input(tp.root, "", Path::new("empty.clj"));
        assert!(input.tagged_fns.is_empty());
    }

    #[test]
    fn simple_defn() {
        let source = "(defn add [a b] (+ a b))";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("core.clj"));
        assert_eq!(input.tagged_fns.len(), 1);
        assert_eq!(input.tagged_fns[0].name, "add");
        assert_eq!(input.tagged_fns[0].params.len(), 2);
    }

    #[test]
    fn defn_with_generator() {
        let source = "(defn count-up [n] (lazy-seq (cons n (count-up (dec n)))))";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("core.clj"));
        assert_eq!(input.generator_refs.len(), 1);
        assert_eq!(input.generator_refs[0].name, "count-up");
    }

    #[test]
    fn multiple_defns() {
        let source = "(defn a [x] x)\n(defn b [y] y)";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("core.clj"));
        assert_eq!(input.tagged_fns.len(), 2);
    }

    #[test]
    fn version_constant() {
        assert_eq!(RUNNER_INPUT_SCHEMA_VERSION, "0.1.0");
    }

    #[test]
    fn span_is_correct() {
        let source = "(defn foo [x] x)";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("core.clj"));
        assert_eq!(input.tagged_fns.len(), 1);
        let span = &input.tagged_fns[0].span;
        assert_eq!(span.file, "core.clj");
        assert_eq!(span.start_line, 1);
    }
}
