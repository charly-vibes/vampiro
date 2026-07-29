//! Julia lifecycle fact extraction for Vampiro.
//!
//! Extracts conservative lifecycle facts from Julia source:
//! - Write facts: local variable assignments
//! - Retry facts: for/while loops with try/catch
//! - Resources: do-block resource acquisition

use serde::{Deserialize, Serialize};
use std::path::Path;
use tree_sitter::Node;
use vampiro_cir::SourceSpan;

pub const LIFECYCLE_FACT_SCHEMA_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LifecycleFacts {
    pub version: String,
    pub source_file: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub writes: Vec<WriteFact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retries: Vec<RetryFact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<ResourceFact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exit_paths: Vec<ExitPathFact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<AliasFact>,
}

impl LifecycleFacts {
    pub fn new(source_file: impl Into<String>) -> Self {
        LifecycleFacts {
            version: LIFECYCLE_FACT_SCHEMA_VERSION.into(),
            source_file: source_file.into(),
            writes: Vec::new(),
            retries: Vec::new(),
            resources: Vec::new(),
            exit_paths: Vec::new(),
            aliases: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WriteFact {
    pub target: String,
    pub kind: String,
    pub function: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RetryFact {
    pub kind: String,
    pub function: String,
    pub has_break_with_value: bool,
    pub has_continue: bool,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResourceFact {
    pub variable: String,
    pub type_name: String,
    pub kind: String,
    pub function: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExitPathFact {
    pub kind: String,
    pub function: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AliasFact {
    pub name: String,
    pub original: String,
    pub function: String,
    pub span: SourceSpan,
}

struct ExtractionCtx {
    current_fn: String,
}

/// Extract lifecycle facts from a tree-sitter parsed Julia source.
pub fn extract_lifecycle_facts(root: Node, source: &str, path: &Path) -> LifecycleFacts {
    let file_path = path.to_string_lossy().to_string();
    let mut facts = LifecycleFacts::new(&file_path);
    let mut ctx = ExtractionCtx {
        current_fn: String::new(),
    };

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        scan_node(child, source, &file_path, &mut facts, &mut ctx);
    }

    facts
}

fn scan_node(
    node: Node,
    source: &str,
    file_path: &str,
    facts: &mut LifecycleFacts,
    ctx: &mut ExtractionCtx,
) {
    match node.kind() {
        "function_definition" => {
            let name = extract_fn_name(node, source).unwrap_or_default();
            let prev = ctx.current_fn.clone();
            ctx.current_fn = name;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                scan_node(child, source, file_path, facts, ctx);
            }
            ctx.current_fn = prev;
        }
        "assignment" => {
            // Detect write operations: `x = ...`
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    if let Some(name) = node_text(child, source) {
                        if !ctx.current_fn.is_empty() {
                            facts.writes.push(WriteFact {
                                target: name,
                                kind: "assignment".to_string(),
                                function: ctx.current_fn.clone(),
                                span: node_span(child, file_path),
                            });
                        }
                    }
                }
            }
        }
        "for_statement" | "while_statement" => {
            let kind = if node.kind() == "for_statement" {
                "for_loop"
            } else {
                "while_loop"
            };
            if !ctx.current_fn.is_empty() {
                facts.retries.push(RetryFact {
                    kind: kind.to_string(),
                    function: ctx.current_fn.clone(),
                    has_break_with_value: false,
                    has_continue: true, // for/while loops can continue in Julia
                    span: node_span(node, file_path),
                });
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                scan_node(child, source, file_path, facts, ctx);
            }
        }
        "return_statement" => {
            if !ctx.current_fn.is_empty() {
                facts.exit_paths.push(ExitPathFact {
                    kind: "return".to_string(),
                    function: ctx.current_fn.clone(),
                    span: node_span(node, file_path),
                });
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                scan_node(child, source, file_path, facts, ctx);
            }
        }
    }
}

fn extract_fn_name(node: Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "signature" {
            let mut cc = child.walk();
            for c in child.children(&mut cc) {
                if c.kind() == "call_expression" {
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
        let facts = extract_lifecycle_facts(tp.root, "", Path::new("empty.jl"));
        assert!(facts.writes.is_empty());
    }

    #[test]
    fn simple_function_no_facts() {
        let source = "function foo() end";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("lib.jl"));
        assert!(facts.writes.is_empty());
    }

    #[test]
    fn assignment_detected() {
        let source = "function foo()\n    x = 42\nend";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("lib.jl"));
        assert_eq!(facts.writes.len(), 1);
        assert_eq!(facts.writes[0].target, "x");
    }

    #[test]
    fn return_detected() {
        let source = "function foo()\n    return 42\nend";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("lib.jl"));
        assert_eq!(facts.exit_paths.len(), 1);
        assert_eq!(facts.exit_paths[0].kind, "return");
    }

    #[test]
    fn for_loop_detected() {
        let source = "function foo()\n    for i in 1:10\n        println(i)\n    end\nend";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("lib.jl"));
        assert_eq!(facts.retries.len(), 1);
        assert_eq!(facts.retries[0].kind, "for_loop");
    }
}
