//! Clojure lifecycle fact extraction for Vampiro.
//!
//! Extracts conservative lifecycle facts from Clojure source:
//! - Write facts: binding and assignment patterns
//! - Retry facts: loop/recur patterns
//! - Resources: with-open usage
//! - Exit paths: function return values

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

/// Extract lifecycle facts from a tree-sitter parsed Clojure source.
pub fn extract_lifecycle_facts(root: Node, source: &str, path: &Path) -> LifecycleFacts {
    let file_path = path.to_string_lossy().to_string();
    let mut facts = LifecycleFacts::new(&file_path);
    let mut ctx = ExtractionCtx {
        current_fn: String::new(),
    };

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        scan_form(child, source, &file_path, &mut facts, &mut ctx);
    }

    facts
}

fn scan_form(
    node: Node,
    source: &str,
    file_path: &str,
    facts: &mut LifecycleFacts,
    ctx: &mut ExtractionCtx,
) {
    if node.kind() != "list_lit" {
        return;
    }

    let first = node.child(1);
    let first_name = first
        .and_then(|n| n.child_by_field_name("name"))
        .and_then(|n| node_text(n, source));

    match first_name.as_deref() {
        Some("defn") | Some("defn-") => {
            let name = node
                .child(2)
                .and_then(|n| n.child_by_field_name("name"))
                .and_then(|n| node_text(n, source))
                .unwrap_or_default();
            let prev = ctx.current_fn.clone();
            ctx.current_fn = name;
            // Scan all children for sub-forms
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                scan_form(child, source, file_path, facts, ctx);
            }
            ctx.current_fn = prev;
        }
        Some("with-open") => {
            // with-open [binding init] body
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "vec_lit" {
                    let mut cc = child.walk();
                    for p in child.children(&mut cc) {
                        if p.kind() == "sym_lit" {
                            if let Some(name) = p
                                .child_by_field_name("name")
                                .and_then(|n| node_text(n, source))
                            {
                                facts.resources.push(ResourceFact {
                                    variable: name,
                                    type_name: "file".to_string(),
                                    kind: "context_manager".to_string(),
                                    function: ctx.current_fn.clone(),
                                    span: node_span(p, file_path),
                                });
                            }
                            // After sym_lit, next is the init expr — skip it
                            break;
                        }
                    }
                }
            }
            // Recurse into body
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                scan_form(child, source, file_path, facts, ctx);
            }
        }
        Some("loop") => {
            // loop [bindings] body
            if !ctx.current_fn.is_empty() {
                facts.retries.push(RetryFact {
                    kind: "loop".to_string(),
                    function: ctx.current_fn.clone(),
                    has_break_with_value: form_contains_symbol(node, source, "recur"),
                    has_continue: false,
                    span: node_span(node, file_path),
                });
            }
            // Recurse into body
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                scan_form(child, source, file_path, facts, ctx);
            }
        }
        Some("let") => {
            // let [bindings...] body — bindings are writes
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "vec_lit" {
                    let mut cc = child.walk();
                    for p in child.children(&mut cc) {
                        if p.kind() == "sym_lit" {
                            if let Some(name) = p
                                .child_by_field_name("name")
                                .and_then(|n| node_text(n, source))
                            {
                                if !ctx.current_fn.is_empty() {
                                    facts.writes.push(WriteFact {
                                        target: name,
                                        kind: "binding".to_string(),
                                        function: ctx.current_fn.clone(),
                                        span: node_span(p, file_path),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            // Recurse into body
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                scan_form(child, source, file_path, facts, ctx);
            }
        }
        _ => {
            // Recurse into children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                scan_form(child, source, file_path, facts, ctx);
            }
        }
    }
}

fn form_contains_symbol(node: Node, source: &str, target: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "sym_lit" {
            if let Some(name) = child
                .child_by_field_name("name")
                .and_then(|n| node_text(n, source))
            {
                if name == target {
                    return true;
                }
            }
        }
        if child.child_count() > 0 && form_contains_symbol(child, source, target) {
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
        let facts = extract_lifecycle_facts(tp.root, "", Path::new("empty.clj"));
        assert!(facts.writes.is_empty());
    }

    #[test]
    fn with_open_detected() {
        let source = "(defn read-f [path] (with-open [r (io/reader path)] (doall (line-seq r))))";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("core.clj"));
        assert_eq!(facts.resources.len(), 1);
        assert_eq!(facts.resources[0].variable, "r");
    }

    #[test]
    fn loop_detected() {
        let source = "(defn retry [f n] (loop [i n] (when (pos? i) (f) (recur (dec i)))))";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("core.clj"));
        assert_eq!(facts.retries.len(), 1);
        assert_eq!(facts.retries[0].kind, "loop");
    }

    #[test]
    fn let_binding_detected() {
        let source = "(defn foo [] (let [x 42 y 10] (+ x y)))";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("core.clj"));
        assert!(facts.writes.len() >= 2);
        assert_eq!(facts.writes[0].target, "x");
        assert_eq!(facts.writes[1].target, "y");
    }
}
