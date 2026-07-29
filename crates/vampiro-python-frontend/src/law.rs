//! Python law runner-input extraction for Vampiro.
//!
//! Extracts Python-specific metadata that law verification will use:
//! - Function declarations with type annotations → tagged functions
//! - Serializable parameters (primitive types)
//! - Generator references (functions with `yield`)
//!
//! Extraction is done via tree-sitter CST walking, using the same pattern
//! as the main extraction module.

use serde::{Deserialize, Serialize};
use std::path::Path;
use tree_sitter::Node;
use vampiro_cir::SourceSpan;

/// The law runner-input schema version.
pub const RUNNER_INPUT_SCHEMA_VERSION: &str = "0.1.0";

/// All law runner-input data extracted from a single Python source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LawRunnerInput {
    /// The schema version of this runner-input data.
    pub version: String,
    /// The source file path.
    pub source_file: String,
    /// Functions with type annotations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tagged_fns: Vec<TaggedFn>,
    /// Serializable values found in the source.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub serializable_values: Vec<SerializableValue>,
    /// Generator/iterator references.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generator_refs: Vec<GeneratorRef>,
}

impl LawRunnerInput {
    /// Create a new empty law runner-input for the given source file.
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

/// A function tagged with type annotations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TaggedFn {
    /// The function name.
    pub name: String,
    /// Tags found on this function.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Parameters of this function.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<FnParam>,
    /// Return type annotation (empty string if absent).
    pub return_type: String,
    /// Source span of the function definition.
    pub span: SourceSpan,
}

/// A function parameter with type information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FnParam {
    /// The parameter name.
    pub name: String,
    /// The type name string (e.g. "int", "str", "list[str]").
    pub type_name: String,
    /// Whether the type is serializable for law verification.
    pub is_serializable: bool,
}

/// A serializable value found in the source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SerializableValue {
    /// The variable name.
    pub name: String,
    /// The type name.
    pub type_name: String,
    /// The source span.
    pub span: SourceSpan,
}

/// A generator or iterator reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GeneratorRef {
    /// The generator name.
    pub name: String,
    /// The kind of generator (e.g. "generator", "iterator").
    pub kind: String,
    /// The source span.
    pub span: SourceSpan,
}

/// Check if a type name represents a serializable type.
fn is_serializable_type(type_name: &str) -> bool {
    matches!(
        type_name.trim(),
        "int"
            | "float"
            | "str"
            | "bool"
            | "bytes"
            | "None"
            | "Optional[int]"
            | "Optional[str]"
            | "Optional[float]"
            | "Optional[bool]"
    )
}

/// Extract law runner-input from a tree-sitter parsed Python module.
pub fn extract_law_input(root: Node, source: &str, path: &Path) -> LawRunnerInput {
    let file_path = path.to_string_lossy().to_string();
    let mut input = LawRunnerInput::new(&file_path);

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        collect_functions(child, source, &file_path, &mut input);
    }

    input
}

/// Collect function declarations from the CST, extracting tagged functions
/// and generator references.
fn collect_functions(node: Node, source: &str, file_path: &str, input: &mut LawRunnerInput) {
    match node.kind() {
        "function_definition" => {
            process_function(node, source, file_path, input);
        }
        "decorated_definition" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "function_definition" {
                    process_function(child, source, file_path, input);
                }
            }
        }
        "class_definition" => {
            // Recurse into class body for methods.
            let body = node.child_by_field_name("body");
            if let Some(body_node) = body {
                let mut cursor = body_node.walk();
                for child in body_node.children(&mut cursor) {
                    collect_functions(child, source, file_path, input);
                }
            }
        }
        "module" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_functions(child, source, file_path, input);
            }
        }
        _ => {}
    }
}

/// Process a single function definition for law runner-input extraction.
fn process_function(node: Node, source: &str, file_path: &str, input: &mut LawRunnerInput) {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| node_text(n, source))
        .unwrap_or_else(|| "<anonymous>".to_string());

    let span = node_span(node, file_path);

    // Extract parameters
    let params = extract_params(node, source);

    // Extract return type
    let return_type = extract_return_type(node, source);

    // Check if this is a generator (has yield)
    let is_generator = has_yield(node);

    // Build the tagged function entry
    let tagged_fn = TaggedFn {
        name: name.clone(),
        tags: Vec::new(),
        params,
        return_type,
        span: span.clone(),
    };

    input.tagged_fns.push(tagged_fn);

    // If generator, add a generator ref
    if is_generator {
        input.generator_refs.push(GeneratorRef {
            name,
            kind: "generator".to_string(),
            span,
        });
    }
}

/// Extract function parameters from a function definition node.
fn extract_params(node: Node, source: &str) -> Vec<FnParam> {
    let parameters = node.child_by_field_name("parameters");
    let params_node = match parameters {
        Some(p) => p,
        None => return Vec::new(),
    };

    let mut params = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                // Simple parameter without type hint (e.g. `def fn(x):`)
                let name = node_text(child, source).unwrap_or_default();
                // Skip `self` and `cls`
                if name == "self" || name == "cls" {
                    continue;
                }
                params.push(FnParam {
                    name,
                    type_name: String::new(),
                    is_serializable: false,
                });
            }
            "typed_parameter" | "typed_default_parameter" => {
                let name = child
                    .child_by_field_name("name")
                    .or_else(|| {
                        // Fallback: take the first identifier child by index
                        for i in 0..child.child_count() {
                            if let Some(n) = child.child(i) {
                                if n.kind() == "identifier" {
                                    return Some(n);
                                }
                            }
                        }
                        None
                    })
                    .and_then(|n| node_text(n, source))
                    .unwrap_or_default();
                // Skip `self` and `cls`
                if name == "self" || name == "cls" {
                    continue;
                }
                let type_node = child.child_by_field_name("type");
                let type_name = match type_node {
                    Some(t) => type_node_text(t, source).unwrap_or_default(),
                    None => String::new(),
                };
                let is_serializable = is_serializable_type(&type_name);
                params.push(FnParam {
                    name,
                    type_name,
                    is_serializable,
                });
            }
            "default_parameter" => {
                let name = child
                    .child_by_field_name("name")
                    .or_else(|| {
                        for i in 0..child.child_count() {
                            if let Some(n) = child.child(i) {
                                if n.kind() == "identifier" {
                                    return Some(n);
                                }
                            }
                        }
                        None
                    })
                    .and_then(|n| node_text(n, source))
                    .unwrap_or_default();
                if name == "self" || name == "cls" {
                    continue;
                }
                params.push(FnParam {
                    name,
                    type_name: String::new(),
                    is_serializable: false,
                });
            }
            _ => {}
        }
    }

    params
}

/// Extract the return type from a function definition node.
fn extract_return_type(node: Node, source: &str) -> String {
    let return_type = node.child_by_field_name("return_type");
    match return_type {
        Some(t) => type_node_text(t, source).unwrap_or_default(),
        None => String::new(),
    }
}

/// Get the text of a type node, including subscript brackets.
fn type_node_text(node: Node, source: &str) -> Option<String> {
    // For subscript types like `list[str]`, we need the full text.
    let start = node.start_byte();
    let end = node.end_byte();
    if start < end && end <= source.len() {
        Some(source[start..end].to_string())
    } else {
        None
    }
}

/// Check if a function body contains a `yield` statement.
fn has_yield(node: Node) -> bool {
    let body = node.child_by_field_name("body");
    let body_node = match body {
        Some(b) => b,
        None => return false,
    };
    scan_for_yield(body_node)
}

/// Recursively scan a node for yield statements.
fn scan_for_yield(node: Node) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "yield" {
            return true;
        }
        if child.child_count() > 0 && scan_for_yield(child) {
            return true;
        }
    }
    false
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
    fn empty_source_produces_empty_input() {
        let tp = parse_source("");
        let input = extract_law_input(tp.root, "", Path::new("empty.py"));
        assert_eq!(input.version, RUNNER_INPUT_SCHEMA_VERSION);
        assert_eq!(input.source_file, "empty.py");
        assert!(input.tagged_fns.is_empty());
        assert!(input.generator_refs.is_empty());
    }

    #[test]
    fn simple_function_with_type_hints() {
        let source = "def add(a: int, b: int) -> int:\n    return a + b";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("lib.py"));
        assert_eq!(input.tagged_fns.len(), 1);
        let f = &input.tagged_fns[0];
        assert_eq!(f.name, "add");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "a");
        assert_eq!(f.params[0].type_name, "int");
        assert!(f.params[0].is_serializable);
        assert_eq!(f.params[1].name, "b");
        assert_eq!(f.params[1].type_name, "int");
        assert!(f.params[1].is_serializable);
        assert_eq!(f.return_type, "int");
    }

    #[test]
    fn function_without_type_hints() {
        let source = "def greet(name):\n    return f'Hello, {name}!'";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("lib.py"));
        assert_eq!(input.tagged_fns.len(), 1);
        let f = &input.tagged_fns[0];
        assert_eq!(f.name, "greet");
        assert_eq!(f.params.len(), 1);
        assert_eq!(f.params[0].name, "name");
        assert!(f.params[0].type_name.is_empty());
        assert!(!f.params[0].is_serializable);
        assert!(f.return_type.is_empty());
    }

    #[test]
    fn generator_function_has_ref() {
        let source = "def count(n: int):\n    for i in range(n):\n        yield i";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("lib.py"));
        assert_eq!(input.tagged_fns.len(), 1);
        assert_eq!(input.generator_refs.len(), 1);
        assert_eq!(input.generator_refs[0].name, "count");
        assert_eq!(input.generator_refs[0].kind, "generator");
    }

    #[test]
    fn non_generator_function_has_no_ref() {
        let source = "def add(a: int, b: int) -> int:\n    return a + b";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("lib.py"));
        assert!(input.generator_refs.is_empty());
    }

    #[test]
    fn class_methods_are_collected() {
        let source = "class Calc:\n    def add(self, a: int, b: int) -> int:\n        return a + b\n    def sub(self, a: int, b: int) -> int:\n        return a - b";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("lib.py"));
        assert_eq!(input.tagged_fns.len(), 2);
        assert_eq!(input.tagged_fns[0].name, "add");
        assert_eq!(input.tagged_fns[1].name, "sub");
        // `self` should not appear as a parameter
        assert_eq!(input.tagged_fns[0].params.len(), 2);
        assert_eq!(input.tagged_fns[0].params[0].name, "a");
    }

    #[test]
    fn serializable_parameter_detection() {
        let source =
            "def process(a: int, b: str, c: float, d: bool, e: list[str]) -> None:\n    pass";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("lib.py"));
        assert_eq!(input.tagged_fns.len(), 1);
        let params = &input.tagged_fns[0].params;
        assert!(params[0].is_serializable); // int
        assert!(params[1].is_serializable); // str
        assert!(params[2].is_serializable); // float
        assert!(params[3].is_serializable); // bool
        assert!(!params[4].is_serializable); // list[str]
    }

    #[test]
    fn return_type_extraction() {
        let source = "def lookup(id: int) -> Optional[str]:\n    return None";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("lib.py"));
        assert_eq!(input.tagged_fns[0].return_type, "Optional[str]");
    }

    #[test]
    fn span_is_correct() {
        let source = "def foo(a: int) -> bool:\n    return True";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("lib.py"));
        let span = &input.tagged_fns[0].span;
        assert_eq!(span.file, "lib.py");
        assert_eq!(span.start_line, 1);
        assert_eq!(span.end_line, 2);
    }

    #[test]
    fn multiple_functions_all_collected() {
        let source = "def a(): pass\ndef b(): pass\ndef c(): pass";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("lib.py"));
        assert_eq!(input.tagged_fns.len(), 3);
    }

    #[test]
    fn decorated_function_is_processed() {
        let source = "@staticmethod\ndef validate(x: int) -> bool:\n    return x > 0";
        let tp = parse_source(source);
        let input = extract_law_input(tp.root, source, Path::new("lib.py"));
        assert_eq!(input.tagged_fns.len(), 1);
        assert_eq!(input.tagged_fns[0].name, "validate");
    }
}
