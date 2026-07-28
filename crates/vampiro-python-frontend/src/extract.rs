//! Python CIR extraction logic.
//!
//! Walks the tree-sitter CST and emits CIR nodes, edges, shapes, and effects.

use std::path::Path;
use tree_sitter::Node;
use vampiro_cir::{
    CirEdge, CirGraph, CirNode, EffectChannel, EffectResolution, Provenance, Shape, SourceSpan,
    StableId, TrustProvenance,
};

/// Extract a CIR graph from a tree-sitter parsed Python module.
pub fn extract_graph(root: Node, source: &str, path: &Path) -> CirGraph {
    let file_path = path.to_string_lossy().to_string();
    let mut graph = CirGraph::new(&file_path);
    let mut node_counter: u64 = 0;
    let mut edge_counter: u64 = 0;

    // Walk the module children
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        process_node(
            child,
            source,
            &file_path,
            &mut graph,
            &mut node_counter,
            &mut edge_counter,
            0,
        );
    }

    graph
}

/// Process a single tree-sitter node, recursively extracting CIR nodes and edges.
fn process_node(
    node: Node,
    source: &str,
    file_path: &str,
    graph: &mut CirGraph,
    node_counter: &mut u64,
    edge_counter: &mut u64,
    call_depth: u32,
) {
    match node.kind() {
        "function_definition" => {
            let is_async = node
                .child(0)
                .is_some_and(|c| c.kind() == "async" || c.kind() == "ASYNC");
            process_function_definition(
                node,
                source,
                file_path,
                graph,
                node_counter,
                edge_counter,
                call_depth,
                is_async,
            );
        }
        "decorated_definition" => {
            // Decorated definition wraps a function/class with decorators
            // Note: `async def` is NOT a decorated_definition in tree-sitter-python
            // (the `async` keyword is an anonymous child of function_definition).
            for child in node.children(&mut node.walk()) {
                if child.kind() == "function_definition" {
                    let is_async = child
                        .child(0)
                        .is_some_and(|c| c.kind() == "async" || c.kind() == "ASYNC");
                    process_function_definition(
                        child,
                        source,
                        file_path,
                        graph,
                        node_counter,
                        edge_counter,
                        call_depth,
                        is_async,
                    );
                } else if child.kind() == "class_definition" {
                    let _ = process_class_definition(
                        child,
                        source,
                        file_path,
                        graph,
                        node_counter,
                        edge_counter,
                        call_depth,
                    );
                }
            }
        }
        "class_definition" => {
            let _ = process_class_definition(
                node,
                source,
                file_path,
                graph,
                node_counter,
                edge_counter,
                call_depth,
            );
        }
        "lambda" => {
            process_lambda(node, source, file_path, graph, node_counter);
        }
        "module" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                process_node(
                    child,
                    source,
                    file_path,
                    graph,
                    node_counter,
                    edge_counter,
                    call_depth,
                );
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                process_node(
                    child,
                    source,
                    file_path,
                    graph,
                    node_counter,
                    edge_counter,
                    call_depth,
                );
            }
        }
    }
}

/// Process a lambda expression, extracting a CIR node.
fn process_lambda(
    node: Node,
    source: &str,
    file_path: &str,
    graph: &mut CirGraph,
    node_counter: &mut u64,
) {
    // Lambdas are typically assigned: `add = lambda x, y: x + y`
    // We name them based on the assignment target if available.
    let name = "<lambda>".to_string();
    let span = node_span(node, file_path);
    let id = StableId::new(format!("py:{}:lambda_{}", file_path, *node_counter));

    // Check parameters for domain shape
    let domain = extract_lambda_domain(node, source);

    let cir_node = CirNode {
        id,
        domain,
        codomain: Shape::Scalar,
        effect: EffectChannel::Plain,
        span,
        name: Some(name),
        trust_provenance: TrustProvenance::default(),
    };

    graph.add_node(cir_node);
    *node_counter += 1;
}

/// Extract the domain shape from a lambda's parameters.
fn extract_lambda_domain(node: Node, _source: &str) -> Shape {
    let params = node.child_by_field_name("parameters");
    match params {
        Some(p) => {
            // Count parameters
            let mut count = 0;
            let mut cursor = p.walk();
            for child in p.children(&mut cursor) {
                if child.kind() == "identifier" || child.kind() == "lambda_parameters" {
                    count += 1;
                }
            }
            if count <= 1 {
                Shape::Scalar
            } else {
                Shape::Record(vec![Shape::Scalar; count])
            }
        }
        None => Shape::Scalar,
    }
}

/// Process a function definition node, extracting a CIR node and its body.
#[allow(clippy::too_many_arguments)]
fn process_function_definition(
    node: Node,
    source: &str,
    file_path: &str,
    graph: &mut CirGraph,
    node_counter: &mut u64,
    edge_counter: &mut u64,
    call_depth: u32,
    is_async: bool,
) {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| node_text(n, source))
        .unwrap_or_else(|| "<anonymous>".to_string());

    let span = node_span(node, file_path);
    let id = StableId::new(format!("py:{}:{}", file_path, name));

    // Determine effect from body
    let effect = detect_function_effect(node, source, is_async);

    // Determine shapes from type hints
    let domain = extract_domain_shape(node, source);
    let codomain = extract_codomain_shape(node, source);

    let cir_node = CirNode {
        id: id.clone(),
        domain,
        codomain,
        effect,
        span: span.clone(),
        name: Some(name.clone()),
        trust_provenance: TrustProvenance::default(),
    };

    graph.add_node(cir_node);
    *node_counter += 1;

    // Process body for calls
    let body = node.child_by_field_name("body");
    if let Some(body_node) = body {
        process_body_for_calls(
            body_node,
            source,
            file_path,
            &id,
            graph,
            edge_counter,
            call_depth + 1,
        );
    }
}

/// Process a class definition node, extracting a CIR node and its methods.
fn process_class_definition(
    node: Node,
    source: &str,
    file_path: &str,
    graph: &mut CirGraph,
    node_counter: &mut u64,
    edge_counter: &mut u64,
    call_depth: u32,
) -> Option<StableId> {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| node_text(n, source))
        .unwrap_or_else(|| "<anonymous>".to_string());

    let span = node_span(node, file_path);
    let id = StableId::new(format!("py:{}:{}", file_path, name));

    let cir_node = CirNode {
        id: id.clone(),
        domain: Shape::Scalar,
        codomain: Shape::Scalar,
        effect: EffectChannel::Plain,
        span,
        name: Some(name),
        trust_provenance: TrustProvenance::default(),
    };

    graph.add_node(cir_node);
    *node_counter += 1;

    // Process body for method definitions
    let body = node.child_by_field_name("body");
    if let Some(body_node) = body {
        let mut cursor = body_node.walk();
        for child in body_node.children(&mut cursor) {
            match child.kind() {
                "function_definition" => {
                    let is_async = child
                        .child(0)
                        .is_some_and(|c| c.kind() == "async" || c.kind() == "ASYNC");
                    process_function_definition(
                        child,
                        source,
                        file_path,
                        graph,
                        node_counter,
                        edge_counter,
                        call_depth,
                        is_async,
                    );
                }
                "decorated_definition" => {
                    for c in child.children(&mut child.walk()) {
                        if c.kind() == "function_definition" {
                            let is_async = c
                                .child(0)
                                .is_some_and(|cc| cc.kind() == "async" || cc.kind() == "ASYNC");
                            process_function_definition(
                                c,
                                source,
                                file_path,
                                graph,
                                node_counter,
                                edge_counter,
                                call_depth,
                                is_async,
                            );
                        }
                    }
                }
                _ => {
                    process_body_for_calls(
                        child,
                        source,
                        file_path,
                        &id,
                        graph,
                        edge_counter,
                        call_depth + 1,
                    );
                }
            }
        }
    }

    Some(id)
}

/// Process a body node, extracting call edges.
fn process_body_for_calls(
    node: Node,
    source: &str,
    file_path: &str,
    caller_id: &StableId,
    graph: &mut CirGraph,
    edge_counter: &mut u64,
    call_depth: u32,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        process_call_expression(
            child,
            source,
            file_path,
            caller_id,
            graph,
            edge_counter,
            call_depth,
        );
    }
}

/// Process a call expression, extracting a CIR edge.
fn process_call_expression(
    node: Node,
    source: &str,
    file_path: &str,
    caller_id: &StableId,
    graph: &mut CirGraph,
    edge_counter: &mut u64,
    call_depth: u32,
) {
    match node.kind() {
        "call" => {
            let function = node.child_by_field_name("function");
            if let Some(func_node) = function {
                let callee_name = extract_call_name(func_node, source);
                let callee_id = StableId::new(format!("py:{}:{}", file_path, callee_name));

                let span = node_span(node, file_path);
                let provenance = if call_depth <= 3 {
                    Provenance::Direct
                } else if call_depth <= 10 {
                    Provenance::WithinH { hops: call_depth }
                } else {
                    Provenance::OverBound {
                        max_hops: 10,
                        actual: call_depth,
                        traced_hops: vec![],
                    }
                };

                let edge = CirEdge {
                    id: StableId::new(format!("py:edge:{}", caller_id)),
                    source: caller_id.clone(),
                    target: callee_id,
                    resolution: EffectResolution::Propagated,
                    unwrap_evidence: None,
                    provenance,
                    span,
                    discard_spans: vec![],
                    trust_provenance: TrustProvenance::default(),
                };

                graph.add_edge(edge);
                *edge_counter += 1;
            }

            // Recurse into arguments for nested calls
            let arguments = node.child_by_field_name("arguments");
            if let Some(args) = arguments {
                let mut cursor = args.walk();
                for arg in args.children(&mut cursor) {
                    process_call_expression(
                        arg,
                        source,
                        file_path,
                        caller_id,
                        graph,
                        edge_counter,
                        call_depth + 1,
                    );
                }
            }
        }
        "attribute" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                process_call_expression(
                    child,
                    source,
                    file_path,
                    caller_id,
                    graph,
                    edge_counter,
                    call_depth,
                );
            }
        }
        "await" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                process_call_expression(
                    child,
                    source,
                    file_path,
                    caller_id,
                    graph,
                    edge_counter,
                    call_depth,
                );
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                process_call_expression(
                    child,
                    source,
                    file_path,
                    caller_id,
                    graph,
                    edge_counter,
                    call_depth,
                );
            }
        }
    }
}

/// Detect the effect channel of a function based on its body contents.
fn detect_function_effect(node: Node, _source: &str, is_async: bool) -> EffectChannel {
    let body = node.child_by_field_name("body");
    let body_node = match body {
        Some(b) => b,
        None => return EffectChannel::Plain,
    };

    let mut flags = EffectFlags {
        has_async: is_async,
        ..Default::default()
    };

    scan_body_for_effects(body_node, &mut flags);

    // Build the effect channel (outermost first)
    let mut effect = EffectChannel::Plain;
    if flags.has_stream {
        effect = EffectChannel::Recursive(Box::new(EffectChannel::Stream));
    }
    if flags.has_async {
        effect = EffectChannel::Recursive(Box::new(EffectChannel::Async));
    }
    if flags.has_result {
        effect = EffectChannel::Recursive(Box::new(EffectChannel::Result));
    }
    if flags.has_resource {
        effect = EffectChannel::Recursive(Box::new(EffectChannel::Throws));
    }
    if flags.has_option {
        effect = EffectChannel::Recursive(Box::new(EffectChannel::Option));
    }

    effect
}

/// Scan a body node for effect keywords (yield, await, try, with).
/// Tracks detected effect channel flags during body scanning.
#[derive(Default)]
struct EffectFlags {
    has_async: bool,
    has_stream: bool,
    has_result: bool,
    has_resource: bool,
    has_option: bool,
}

fn scan_body_for_effects(node: Node, flags: &mut EffectFlags) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "yield" => flags.has_stream = true,
            "await" => flags.has_async = true,
            "try_statement" => flags.has_result = true,
            "with_statement" => flags.has_resource = true,
            _ => {}
        }

        // Recurse into all non-leaf children to find nested effects
        if child.child_count() > 0 {
            scan_body_for_effects(child, flags);
        }
    }
}

/// Extract the name of a call target from a call expression's function node.
fn extract_call_name(func_node: Node, source: &str) -> String {
    match func_node.kind() {
        "identifier" => node_text(func_node, source).unwrap_or_else(|| "<unknown>".to_string()),
        "attribute" => {
            let attr = func_node
                .child_by_field_name("attribute")
                .and_then(|n| node_text(n, source))
                .unwrap_or_else(|| "<unknown>".to_string());
            let object = func_node
                .child_by_field_name("object")
                .and_then(|n| node_text(n, source))
                .unwrap_or_else(|| "?".to_string());
            format!("{}.{}", object, attr)
        }
        _ => node_text(func_node, source).unwrap_or_else(|| "<expression>".to_string()),
    }
}

/// Extract the domain shape from a function's parameter type hints.
fn extract_domain_shape(node: Node, source: &str) -> Shape {
    let parameters = node.child_by_field_name("parameters");
    let params = match parameters {
        Some(p) => p,
        None => return Shape::Scalar,
    };

    let mut fields = Vec::new();
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                fields.push(Shape::Scalar);
            }
            "typed_parameter" => {
                let type_node = child.child_by_field_name("type");
                match type_node {
                    Some(t) => fields.push(type_hint_to_shape(t, source)),
                    None => fields.push(Shape::Scalar),
                }
            }
            "default_parameter" => {
                fields.push(Shape::Scalar);
            }
            "typed_default_parameter" => {
                let type_node = child.child_by_field_name("type");
                match type_node {
                    Some(t) => fields.push(type_hint_to_shape(t, source)),
                    None => fields.push(Shape::Scalar),
                }
            }
            _ => {}
        }
    }

    if fields.len() <= 1 {
        fields.into_iter().next().unwrap_or(Shape::Scalar)
    } else {
        Shape::Record(fields)
    }
}

/// Extract the codomain shape from a function's return type hint.
fn extract_codomain_shape(node: Node, source: &str) -> Shape {
    let return_type = node.child_by_field_name("return_type");
    match return_type {
        Some(t) => type_hint_to_shape(t, source),
        None => Shape::Scalar,
    }
}

/// Convert a Python type hint node to a CIR shape.
fn type_hint_to_shape(node: Node, source: &str) -> Shape {
    match node.kind() {
        "type" => {
            let children: Vec<_> = {
                let mut cursor = node.walk();
                node.children(&mut cursor).collect()
            };
            if children.is_empty() {
                return Shape::Scalar;
            }
            let first = children[0];
            match first.kind() {
                "identifier" => {
                    let name = node_text(first, source).unwrap_or_default();
                    match name.as_str() {
                        "int" | "float" | "str" | "bool" | "bytes" | "None" | "Any" => {
                            Shape::Scalar
                        }
                        "list" | "set" | "frozenset" => {
                            if children.len() > 1 {
                                let inner = &children[1];
                                if inner.kind() == "type" {
                                    let inner_shape = type_hint_to_shape(*inner, source);
                                    Shape::Record(vec![inner_shape])
                                } else {
                                    Shape::Record(vec![Shape::Scalar])
                                }
                            } else {
                                Shape::Record(vec![Shape::Scalar])
                            }
                        }
                        "dict" => Shape::Record(vec![Shape::Scalar, Shape::Scalar]),
                        "tuple" => Shape::Record(vec![Shape::Scalar]),
                        "Optional" => {
                            if children.len() > 1 {
                                let inner = &children[1];
                                if inner.kind() == "type" {
                                    type_hint_to_shape(*inner, source)
                                } else {
                                    Shape::Scalar
                                }
                            } else {
                                Shape::Scalar
                            }
                        }
                        _ => Shape::Scalar,
                    }
                }
                "subscript" => Shape::Record(vec![Shape::Scalar]),
                "union_type" => Shape::Record(vec![Shape::Scalar]),
                _ => Shape::Scalar,
            }
        }
        "union_type" => Shape::Record(vec![Shape::Scalar]),
        "generic_type" => Shape::Record(vec![Shape::Scalar]),
        "list" | "tuple" | "dictionary" => Shape::Record(vec![Shape::Scalar]),
        "none" => Shape::Scalar,
        _ => Shape::Scalar,
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

    #[test]
    fn test_node_text_returns_text() {
        let source = "def greet(): pass";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let func_node = root.child(0).unwrap();
        let name_node = func_node.child_by_field_name("name").unwrap();
        assert_eq!(node_text(name_node, source).as_deref(), Some("greet"));
    }

    #[test]
    fn test_extract_call_name_identifier() {
        let name = extract_call_name_from_str("helper()", "helper");
        assert_eq!(name, "helper");
    }

    #[test]
    fn test_extract_call_name_attribute() {
        let name = extract_call_name_from_str("obj.method()", "obj.method");
        assert_eq!(name, "obj.method");
    }

    #[test]
    fn test_detect_plain_function_effect() {
        let source = "def fn(): pass";
        check_function_effect(source, |e| matches!(e, EffectChannel::Plain));
    }

    #[test]
    fn test_detect_yield_effect() {
        let source = "def fn():\n    yield 1";
        check_function_effect(source, |e| {
            format!("{:?}", e).contains("Stream") || matches!(e, EffectChannel::Recursive(..))
        });
    }

    #[test]
    fn test_detect_async_effect() {
        let source = "async def fn():\n    pass";
        check_function_effect(source, |e| {
            format!("{:?}", e).contains("Async") || matches!(e, EffectChannel::Recursive(..))
        });
    }

    fn extract_call_name_from_str(source: &str, _expected_func: &str) -> String {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "expression_statement" {
                let mut c2 = child.walk();
                for expr in child.children(&mut c2) {
                    if expr.kind() == "call" {
                        let func = expr.child_by_field_name("function").unwrap();
                        return extract_call_name(func, source);
                    }
                }
            }
        }
        "<not found>".to_string()
    }

    fn check_function_effect(source: &str, check: impl Fn(&EffectChannel) -> bool) {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "function_definition" || child.kind() == "decorated_definition" {
                let func_node = if child.kind() == "decorated_definition" {
                    let mut c2 = child.walk();
                    let mut found = None;
                    for c in child.children(&mut c2) {
                        if c.kind() == "function_definition" {
                            found = Some(c);
                            break;
                        }
                    }
                    found
                } else {
                    Some(child)
                };

                if let Some(fn_node) = func_node {
                    let is_async = fn_node
                        .child(0)
                        .is_some_and(|c| c.kind() == "async" || c.kind() == "ASYNC");
                    let effect = detect_function_effect(fn_node, source, is_async);
                    assert!(check(&effect), "effect {:?} didn't match", effect);
                    return;
                }
            }
        }
        panic!("no function_definition found");
    }
}
