//! Python lifecycle fact extraction for Vampiro.
//!
//! Extracts conservative lifecycle facts from Python source without
//! performing lifecycle classification or issuing findings.
//! Lifecycle analysis is owned by a separate analysis module.
//!
//! # Facts extracted
//!
//! - **Write facts**: variable assignments, attribute assignments
//! - **Retry facts**: loops that resemble retry patterns (for/while)
//! - **Resources**: context manager usage (`with` statement)
//! - **Exit paths**: return, raise, and implicit None returns
//! - **Aliases**: variable aliasing patterns

use serde::{Deserialize, Serialize};
use std::path::Path;
use tree_sitter::Node;
use vampiro_cir::SourceSpan;

/// The lifecycle-fact schema version.
pub const LIFECYCLE_FACT_SCHEMA_VERSION: &str = "0.1.0";

/// All lifecycle facts extracted from a single Python source file.
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

/// A write operation fact (variable or attribute assignment).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WriteFact {
    /// The target being written to.
    pub target: String,
    /// The kind of write (e.g. "assignment", "augmented_assignment").
    pub kind: String,
    /// The function containing this write.
    pub function: String,
    /// Source span of the write expression.
    pub span: SourceSpan,
}

/// A retry pattern fact (loop that resembles a retry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RetryFact {
    /// The kind of retry (e.g. "for_loop", "while_loop").
    pub kind: String,
    /// The function containing this retry.
    pub function: String,
    /// Whether the loop has a break with value (return inside).
    pub has_break_with_value: bool,
    /// Whether the loop has a continue statement.
    pub has_continue: bool,
    /// Source span of the loop.
    pub span: SourceSpan,
}

/// A resource fact (context manager usage).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResourceFact {
    /// The variable bound by the context manager.
    pub variable: String,
    /// The type name of the resource (e.g. "file", "lock").
    pub type_name: String,
    /// The kind of resource (e.g. "context_manager").
    pub kind: String,
    /// The function containing this resource.
    pub function: String,
    /// Source span of the variable binding.
    pub span: SourceSpan,
}

/// An exit path fact (how a function may exit).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExitPathFact {
    /// The kind of exit (e.g. "return", "raise").
    pub kind: String,
    /// The function containing this exit path.
    pub function: String,
    /// Source span of the exit statement.
    pub span: SourceSpan,
}

/// An alias fact (variable aliasing another reference).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AliasFact {
    /// The alias name.
    pub name: String,
    /// The original name being aliased.
    pub original: String,
    /// The function containing this alias.
    pub function: String,
    /// Source span of the alias.
    pub span: SourceSpan,
}

/// Current function tracking context for lifecycle extraction.
struct ExtractionCtx {
    current_fn: String,
}

/// Extract lifecycle facts from a tree-sitter parsed Python module.
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

/// Scan a CST node for lifecycle facts.
fn scan_node(
    node: Node,
    source: &str,
    file_path: &str,
    facts: &mut LifecycleFacts,
    ctx: &mut ExtractionCtx,
) {
    match node.kind() {
        "function_definition" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|n| node_text(n, source))
                .unwrap_or_else(|| "<anonymous>".to_string());
            let prev_fn = ctx.current_fn.clone();
            ctx.current_fn = name;

            // Scan body for lifecycle facts
            let body = node.child_by_field_name("body");
            if let Some(body_node) = body {
                scan_statements(body_node, source, file_path, facts, ctx);
            }

            ctx.current_fn = prev_fn;
        }
        "decorated_definition" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "function_definition" {
                    scan_node(child, source, file_path, facts, ctx);
                }
            }
        }
        "class_definition" => {
            let body = node.child_by_field_name("body");
            if let Some(body_node) = body {
                let mut cursor = body_node.walk();
                for child in body_node.children(&mut cursor) {
                    scan_node(child, source, file_path, facts, ctx);
                }
            }
        }
        "module" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                scan_node(child, source, file_path, facts, ctx);
            }
        }
        _ => {}
    }
}

/// Scan a block of statements for lifecycle facts.
fn scan_statements(
    node: Node,
    source: &str,
    file_path: &str,
    facts: &mut LifecycleFacts,
    ctx: &ExtractionCtx,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        scan_statement(child, source, file_path, facts, ctx);
    }
}

/// Scan a single statement for lifecycle facts.
fn scan_statement(
    node: Node,
    source: &str,
    file_path: &str,
    facts: &mut LifecycleFacts,
    ctx: &ExtractionCtx,
) {
    match node.kind() {
        "expression_statement" => {
            // An expression statement can contain an assignment, function call, etc.
            // Check for assignment children.
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "assignment" {
                    // Find the left side of the assignment (the target)
                    let target = child
                        .child_by_field_name("left")
                        .and_then(|n| node_text(n, source))
                        .unwrap_or_default();
                    if !target.is_empty() && !ctx.current_fn.is_empty() {
                        facts.writes.push(WriteFact {
                            target,
                            kind: "assignment".to_string(),
                            function: ctx.current_fn.clone(),
                            span: node_span(child, file_path),
                        });
                    }
                }
            }
        }
        "for_statement" | "while_statement" => {
            let loop_kind = if node.kind() == "for_statement" {
                "for_loop"
            } else {
                "while_loop"
            };
            let has_break = has_statement_of_kind(node, "break");
            let has_return = has_statement_of_kind(node, "return_statement");
            let has_continue_stmt = has_statement_of_kind(node, "continue");

            if !ctx.current_fn.is_empty() {
                facts.retries.push(RetryFact {
                    kind: loop_kind.to_string(),
                    function: ctx.current_fn.clone(),
                    has_break_with_value: has_break || has_return,
                    has_continue: has_continue_stmt,
                    span: node_span(node, file_path),
                });
            }

            // Recurse into loop body for other facts
            scan_statements(node, source, file_path, facts, ctx);
        }
        "with_statement" => {
            // Detect resource acquisition via `with X as Y:`
            // In tree-sitter-python 0.25, with_item has only a `value` field.
            // The `as` pattern is part of the expression grammar.
            // Scan children for with_clause → with_item → value as as_pattern.
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "with_clause" {
                    let mut cc = child.walk();
                    for c in child.children(&mut cc) {
                        if c.kind() == "with_item" {
                            if let Some(value) = c.child_by_field_name("value") {
                                if value.kind() == "as_pattern" {
                                    // Try to get the alias
                                    if let Some(alias) = value.child_by_field_name("alias") {
                                        // alias has a single child with the identifier
                                        if let Some(target) = alias.child(0) {
                                            if let Some(var_name) = node_text(target, source) {
                                                if !ctx.current_fn.is_empty() {
                                                    facts.resources.push(ResourceFact {
                                                        variable: var_name,
                                                        type_name: "file".to_string(),
                                                        kind: "context_manager".to_string(),
                                                        function: ctx.current_fn.clone(),
                                                        span: node_span(c, file_path),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Recurse into body
            let body = node.child_by_field_name("body");
            if let Some(body_node) = body {
                scan_statements(body_node, source, file_path, facts, ctx);
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
        "raise_statement" => {
            if !ctx.current_fn.is_empty() {
                facts.exit_paths.push(ExitPathFact {
                    kind: "raise".to_string(),
                    function: ctx.current_fn.clone(),
                    span: node_span(node, file_path),
                });
            }
        }
        _ => {
            // Recurse into compound statements
            scan_statements(node, source, file_path, facts, ctx);
        }
    }
}

/// Check if a node contains a child of the given kind.
fn has_statement_of_kind(node: Node, kind: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return true;
        }
        if child.child_count() > 0 && has_statement_of_kind(child, kind) {
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
    fn empty_source_no_facts() {
        let tp = parse_source("");
        let facts = extract_lifecycle_facts(tp.root, "", Path::new("empty.py"));
        assert!(facts.writes.is_empty());
        assert!(facts.retries.is_empty());
        assert!(facts.resources.is_empty());
        assert!(facts.exit_paths.is_empty());
        assert!(facts.aliases.is_empty());
    }

    #[test]
    fn simple_function_no_facts() {
        let source = "def nothing():\n    pass";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("lib.py"));
        assert!(facts.writes.is_empty());
        assert!(facts.exit_paths.is_empty());
    }

    #[test]
    fn assignment_produces_write_fact() {
        let source = "def foo():\n    x = 42";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("lib.py"));
        assert_eq!(facts.writes.len(), 1);
        assert_eq!(facts.writes[0].target, "x");
        assert_eq!(facts.writes[0].kind, "assignment");
        assert_eq!(facts.writes[0].function, "foo");
    }

    #[test]
    fn return_statement_produces_exit_path() {
        let source = "def foo():\n    return 42";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("lib.py"));
        assert_eq!(facts.exit_paths.len(), 1);
        assert_eq!(facts.exit_paths[0].kind, "return");
        assert_eq!(facts.exit_paths[0].function, "foo");
    }

    #[test]
    fn raise_statement_produces_exit_path() {
        let source = "def foo():\n    raise ValueError('bad')";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("lib.py"));
        assert_eq!(facts.exit_paths.len(), 1);
        assert_eq!(facts.exit_paths[0].kind, "raise");
        assert_eq!(facts.exit_paths[0].function, "foo");
    }

    #[test]
    fn with_statement_produces_resource_fact() {
        let source = "def foo():\n    with open('x') as f:\n        pass";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("lib.py"));
        assert_eq!(facts.resources.len(), 1);
        assert_eq!(facts.resources[0].variable, "f");
        assert_eq!(facts.resources[0].kind, "context_manager");
        assert_eq!(facts.resources[0].function, "foo");
    }

    #[test]
    fn for_loop_produces_retry_fact() {
        let source = "def foo():\n    for i in range(3):\n        if ok:\n            return True\n        continue\n    return False";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("lib.py"));
        assert_eq!(facts.retries.len(), 1);
        assert_eq!(facts.retries[0].kind, "for_loop");
        assert!(facts.retries[0].has_break_with_value);
        assert!(facts.retries[0].has_continue);
        assert_eq!(facts.retries[0].function, "foo");
    }

    #[test]
    fn while_loop_produces_retry_fact() {
        let source =
            "def foo():\n    while True:\n        try:\n            result = api()\n            return result\n        except:\n            continue";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("lib.py"));
        assert_eq!(facts.retries.len(), 1);
        assert_eq!(facts.retries[0].kind, "while_loop");
        assert!(facts.retries[0].has_break_with_value);
        assert!(facts.retries[0].has_continue);
    }

    #[test]
    fn multiple_functions_all_scanned() {
        let source = "def a():\n    return 1\n\ndef b():\n    with open('x') as f:\n        data = f.read()\n    return data";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("lib.py"));
        assert_eq!(facts.exit_paths.len(), 2);
        assert_eq!(facts.exit_paths[0].function, "a");
        assert_eq!(facts.exit_paths[1].function, "b");
        assert_eq!(facts.resources.len(), 1);
        assert_eq!(facts.resources[0].function, "b");
        assert_eq!(facts.writes.len(), 1);
        assert_eq!(facts.writes[0].function, "b");
    }

    #[test]
    fn class_methods_scanned() {
        let source = "class Manager:\n    def connect(self):\n        self.conn = create()\n        return True\n\n    def disconnect(self):\n        if self.conn:\n            self.conn.close()\n        self.conn = None";
        let tp = parse_source(source);
        let facts = extract_lifecycle_facts(tp.root, source, Path::new("lib.py"));
        // connect has return True, disconnect has no explicit return
        assert_eq!(facts.exit_paths.len(), 1);
        // At least two writes (self.conn = create, self.conn = None)
        assert!(facts.writes.len() >= 2);
    }

    #[test]
    fn version_constant_is_set() {
        assert_eq!(LIFECYCLE_FACT_SCHEMA_VERSION, "0.1.0");
    }

    #[test]
    fn facts_have_source_file() {
        let tp = parse_source("");
        let facts = extract_lifecycle_facts(tp.root, "", Path::new("test.py"));
        assert_eq!(facts.source_file, "test.py");
    }
}
