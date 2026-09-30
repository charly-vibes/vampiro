//! Python CIR extraction logic.
//!
//! Walks the tree-sitter CST and emits CIR nodes, edges, shapes, and effects.

use std::collections::HashMap;
use std::path::Path;
use tree_sitter::Node;
use vampiro_cir::{
    CirEdge, CirGraph, CirNode, EffectChannel, EffectResolution, NodeKind, Provenance, ScalarKind,
    Shape, SourceSpan, StableId, TrustProvenance,
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
            &mut HashMap::new(),
        );
    }

    graph
}

/// Process a single tree-sitter node, recursively extracting CIR nodes and edges.
#[allow(clippy::too_many_arguments)]
fn process_node(
    node: Node,
    source: &str,
    file_path: &str,
    graph: &mut CirGraph,
    node_counter: &mut u64,
    edge_counter: &mut u64,
    call_depth: u32,
    local_shapes: &mut HashMap<String, Shape>,
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
                None,
                local_shapes,
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
                        None,
                        local_shapes,
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
                        local_shapes,
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
                local_shapes,
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
                    local_shapes,
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
                    local_shapes,
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
        codomain: Shape::Scalar(ScalarKind::Unit),
        effect: EffectChannel::Plain,
        span,
        name: Some(name),
        trust_provenance: TrustProvenance::default(),
        is_test: false,
        kind: NodeKind::Declaration,
        containing_function: None,
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
                Shape::Scalar(ScalarKind::Unit)
            } else {
                Shape::Record(vec![Shape::Scalar(ScalarKind::Unit); count])
            }
        }
        None => Shape::Scalar(ScalarKind::Unit),
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
    class_name: Option<&str>,
    local_shapes: &mut HashMap<String, Shape>,
) {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| node_text(n, source))
        .unwrap_or_else(|| "<anonymous>".to_string());

    let span = node_span(node, file_path);
    let qualified = match class_name {
        Some(cls) => format!("{}:{}", cls, name),
        None => name.clone(),
    };
    let id = StableId::new(format!("py:{}:{}", file_path, qualified));

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
        is_test: false,
        kind: NodeKind::Declaration,
        containing_function: None,
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
            local_shapes,
            node_counter,
        );
    }
}

#[allow(clippy::too_many_arguments)]
/// Process a class definition node, extracting a CIR node and its methods.
fn process_class_definition(
    node: Node,
    source: &str,
    file_path: &str,
    graph: &mut CirGraph,
    node_counter: &mut u64,
    edge_counter: &mut u64,
    call_depth: u32,
    local_shapes: &mut HashMap<String, Shape>,
) -> Option<StableId> {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| node_text(n, source))
        .unwrap_or_else(|| "<anonymous>".to_string());

    let span = node_span(node, file_path);
    let cls_name = name.clone();
    let id = StableId::new(format!("py:{}:{}", file_path, cls_name));

    let cir_node = CirNode {
        id: id.clone(),
        domain: Shape::Scalar(ScalarKind::Unit),
        codomain: Shape::Scalar(ScalarKind::Unit),
        effect: EffectChannel::Plain,
        span,
        name: Some(name),
        trust_provenance: TrustProvenance::default(),
        is_test: false,
        kind: NodeKind::Declaration,
        containing_function: None,
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
                        Some(&cls_name),
                        local_shapes,
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
                                Some(&cls_name),
                                local_shapes,
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
                        local_shapes,
                        node_counter,
                    );
                }
            }
        }
    }

    Some(id)
}

/// Process a body node, extracting call edges.
#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
fn process_body_for_calls(
    node: Node,
    source: &str,
    file_path: &str,
    caller_id: &StableId,
    graph: &mut CirGraph,
    edge_counter: &mut u64,
    call_depth: u32,
    local_shapes: &mut HashMap<String, Shape>,
    node_counter: &mut u64,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "expression_statement" => {
                // Check for assignment expressions: x = expr
                let mut child_cursor = child.walk();
                for expr_child in child.children(&mut child_cursor) {
                    if expr_child.kind() == "assignment" {
                        process_assignment(
                            expr_child,
                            source,
                            file_path,
                            caller_id,
                            graph,
                            edge_counter,
                            call_depth,
                            local_shapes,
                            node_counter,
                        );
                    } else {
                        process_call_expression(
                            expr_child,
                            source,
                            file_path,
                            caller_id,
                            graph,
                            node_counter,
                            edge_counter,
                            call_depth,
                            local_shapes,
                        );
                    }
                }
            }
            _ => {
                process_call_expression(
                    child,
                    source,
                    file_path,
                    caller_id,
                    graph,
                    node_counter,
                    edge_counter,
                    call_depth,
                    local_shapes,
                );
            }
        }
    }
}

/// Process an assignment expression `x = expr` to track local variable shapes.
#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
fn process_assignment(
    node: Node,
    source: &str,
    file_path: &str,
    caller_id: &StableId,
    graph: &mut CirGraph,
    edge_counter: &mut u64,
    call_depth: u32,
    local_shapes: &mut HashMap<String, Shape>,
    node_counter: &mut u64,
) {
    // Check for simple `x = expr` pattern
    if let Some(left) = node.child_by_field_name("left") {
        if let Some(right) = node.child_by_field_name("right") {
            if left.kind() == "identifier" {
                if let Some(var_name) = node_text(left, source) {
                    // Infer the shape of the right-hand side
                    if let Some(shape) = extract_expr_shape(right, source, graph) {
                        local_shapes.insert(var_name, shape);
                    }
                }
            }

            // Process call expressions in the right-hand side
            process_call_expression(
                right,
                source,
                file_path,
                caller_id,
                graph,
                node_counter,
                edge_counter,
                call_depth,
                local_shapes,
            );

            // Process call expressions in the left-hand side (e.g., obj.attr = val)
            process_call_expression(
                left,
                source,
                file_path,
                caller_id,
                graph,
                node_counter,
                edge_counter,
                call_depth,
                local_shapes,
            );
        }
    }
}

/// Determine whether a call expression is in return position: its value
/// flows directly to the enclosing function's `return` statement
/// (vampiro-224.9 mirrored to the Python frontend). Python has no
/// tail-expression return — a trailing unreturned expression yields None
/// implicitly, so only `return` operands are return position.
fn is_return_position(mut node: Node) -> bool {
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "return_statement" => return true,
            // Value consumed by an enclosing construct — argument of another
            // call, stored/accumulated, indexed, container element, loop or
            // condition context, default value: it does not flow to return.
            "call"
            | "argument_list"
            | "assignment"
            | "augmented_assignment"
            | "named_expression"
            | "subscript"
            | "list"
            | "set"
            | "dictionary"
            | "list_comprehension"
            | "set_comprehension"
            | "dictionary_comprehension"
            | "for_statement"
            | "while_statement"
            | "if_statement"
            | "with_statement"
            | "assert_statement"
            | "yield"
            | "default_parameter"
            | "typed_default_parameter"
            | "lambda" => return false,
            // Value passes through (binary operators, attribute access,
            // parenthesized forms, conditional arms): keep walking up.
            _ => {}
        }
        node = parent;
    }
    false
}

/// Process a call expression, extracting a CIR edge with per-slot data-flow.
#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
fn process_call_expression(
    node: Node,
    source: &str,
    file_path: &str,
    caller_id: &StableId,
    graph: &mut CirGraph,
    node_counter: &mut u64,
    edge_counter: &mut u64,
    call_depth: u32,
    local_shapes: &mut HashMap<String, Shape>,
) {
    match node.kind() {
        "call" => {
            let function = node.child_by_field_name("function");
            if let Some(func_node) = function {
                let callee_name = extract_call_name(func_node, source);
                let callee_id = StableId::new(format!("py:{}:{}", file_path, callee_name));

                let span = node_span(node, file_path);

                if graph.node_by_id(&callee_id).is_some() {
                    // Only add edge if the target node exists in the graph.
                    // Builtins (e.g. getattr) are not extracted as nodes.
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

                    // Emit a single declaration->declaration edge for the return-boundary
                    // check (no slot). vampiro-224.9 mirrored to Python: the edge carries
                    // return_position only when the call's value flows to the enclosing
                    // function's `return` (Python has no tail-expression return).
                    let edge = CirEdge {
                        id: StableId::new(format!("py:edge:{}", *edge_counter)),
                        source: caller_id.clone(),
                        target: callee_id.clone(),
                        resolution: EffectResolution::Propagated,
                        unwrap_evidence: None,
                        provenance: provenance.clone(),
                        span: span.clone(),
                        discard_spans: vec![],
                        trust_provenance: TrustProvenance::default(),
                        slot: None,
                        arg_shape: None,
                        return_position: is_return_position(node),
                    };

                    graph.add_edge(edge);
                    *edge_counter += 1;

                    // Emit expression->declaration edges for each argument with a known
                    // shape. These are the data-flow edges: the composition analyzer
                    // compares the expression's shape against the callee's domain slot.
                    let arguments = node.child_by_field_name("arguments");
                    if let Some(args) = arguments {
                        let mut arg_cursor = args.walk();
                        let arg_nodes: Vec<Node> = args.children(&mut arg_cursor).collect();
                        let mut slot_index: u32 = 0;
                        for arg in arg_nodes.iter() {
                            if arg.is_named() {
                                if let Some(shape) = extract_expr_shape(*arg, source, graph) {
                                    let expr_id = emit_expression_node(
                                        shape,
                                        *arg,
                                        source,
                                        file_path,
                                        graph,
                                        node_counter,
                                        caller_id,
                                    );
                                    let expr_edge = CirEdge {
                                        id: StableId::new(format!(
                                            "py:edge:expr_{}",
                                            *edge_counter
                                        )),
                                        source: expr_id,
                                        target: callee_id.clone(),
                                        resolution: EffectResolution::Propagated,
                                        unwrap_evidence: None,
                                        provenance: provenance.clone(),
                                        span: node_span(*arg, file_path),
                                        discard_spans: vec![],
                                        trust_provenance: TrustProvenance::default(),
                                        slot: Some(slot_index),
                                        arg_shape: None,
                                        // Data-flow edges feed the per-slot domain check,
                                        // not the return-boundary check.
                                        return_position: false,
                                    };
                                    graph.add_edge(expr_edge);
                                    *edge_counter += 1;
                                }
                                slot_index += 1;
                            }
                        }
                    }
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
                            node_counter,
                            edge_counter,
                            call_depth + 1,
                            local_shapes,
                        );
                    }
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
                    node_counter,
                    edge_counter,
                    call_depth,
                    local_shapes,
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
                    node_counter,
                    edge_counter,
                    call_depth,
                    local_shapes,
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
                    node_counter,
                    edge_counter,
                    call_depth,
                    local_shapes,
                );
            }
        }
    }
}

/// Emit an expression node for an intermediate expression value.
///
/// Creates a `CirNode` with `kind: Expression`, domain=codomain=shape,
/// and `containing_function` set to the given declaration ID.
fn emit_expression_node(
    shape: Shape,
    node: Node,
    _source: &str,
    file_path: &str,
    graph: &mut CirGraph,
    node_counter: &mut u64,
    containing_fn: &StableId,
) -> StableId {
    let id = StableId::new(format!("py:expr:{}:{}", file_path, *node_counter));
    let expr_node = CirNode {
        id: id.clone(),
        domain: shape.clone(),
        codomain: shape,
        effect: EffectChannel::Plain,
        span: node_span(node, file_path),
        name: None,
        trust_provenance: TrustProvenance::default(),
        is_test: false,
        kind: NodeKind::Expression,
        containing_function: Some(containing_fn.clone()),
    };
    graph.add_node(expr_node);
    *node_counter += 1;
    id
}

/// Infer the shape of a Python expression from a tree-sitter AST node.
///
/// Returns `None` when the shape cannot be determined statically.
fn extract_expr_shape(node: Node, source: &str, graph: &CirGraph) -> Option<Shape> {
    match node.kind() {
        // Integer literal -> Scalar(Int)
        "integer" => Some(Shape::Scalar(ScalarKind::Int)),
        // Float literal -> Scalar(Float)
        "float" => Some(Shape::Scalar(ScalarKind::Float)),
        // String literal -> Scalar(String)
        "string" | "string_content" => Some(Shape::Scalar(ScalarKind::String)),
        // Boolean literal -> Scalar(Bool)
        "true" => Some(Shape::Scalar(ScalarKind::Bool)),
        "false" => Some(Shape::Scalar(ScalarKind::Bool)),
        // None literal -> Scalar(Unit)
        "none" => Some(Shape::Scalar(ScalarKind::Unit)),
        // Call expression: use the callee's codomain if resolvable
        "call" => {
            let function = node.child_by_field_name("function");
            if let Some(func_node) = function {
                let callee_name = extract_call_name(func_node, source);
                let callee_id = StableId::new(format!("py:{}:{}", graph.source_file, callee_name));
                if let Some(callee_node) = graph.node_by_id(&callee_id) {
                    return Some(callee_node.codomain.clone());
                }
            }
            None
        }
        // Identifier reference: not resolved here (caller handles via local_shapes)
        "identifier" => None,
        // Everything else: opaque
        _ => None,
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
        None => return Shape::Scalar(ScalarKind::Unit),
    };

    let mut fields = Vec::new();
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                fields.push(Shape::Scalar(ScalarKind::Unit));
            }
            "typed_parameter" => {
                let type_node = child.child_by_field_name("type");
                match type_node {
                    Some(t) => fields.push(type_hint_to_shape(t, source)),
                    None => fields.push(Shape::Scalar(ScalarKind::Unit)),
                }
            }
            "default_parameter" => {
                fields.push(Shape::Scalar(ScalarKind::Unit));
            }
            "typed_default_parameter" => {
                let type_node = child.child_by_field_name("type");
                match type_node {
                    Some(t) => fields.push(type_hint_to_shape(t, source)),
                    None => fields.push(Shape::Scalar(ScalarKind::Unit)),
                }
            }
            _ => {}
        }
    }

    if fields.len() <= 1 {
        fields
            .into_iter()
            .next()
            .unwrap_or(Shape::Scalar(ScalarKind::Unit))
    } else {
        Shape::Record(fields)
    }
}

/// Extract the codomain shape from a function's return type hint.
fn extract_codomain_shape(node: Node, source: &str) -> Shape {
    let return_type = node.child_by_field_name("return_type");
    match return_type {
        Some(t) => type_hint_to_shape(t, source),
        None => Shape::Scalar(ScalarKind::Unit),
    }
}

/// Map a Python type name to a ScalarKind variant.
fn python_type_to_scalar(name: &str) -> Option<ScalarKind> {
    match name {
        "int" => Some(ScalarKind::Int),
        "float" => Some(ScalarKind::Float),
        "str" => Some(ScalarKind::String),
        "bool" => Some(ScalarKind::Bool),
        _ => None,
    }
}

/// Convert a Python type hint node to a CIR shape.
///
/// Unresolvable or unannotated types map to `Shape::Opaque` — the 224.3 rule
/// (unresolvable named types must be Opaque, not Scalar(Unit)) — because the
/// composition unifier excludes top-level opaque from break checking, while a
/// Unit scalar witnesses mismatches against any concrete scalar.
fn type_hint_to_shape(node: Node, source: &str) -> Shape {
    match node.kind() {
        // `type` is the grammar's annotation wrapper — dispatch to its single
        // inner node.
        "type" => {
            let mut cursor = node.walk();
            let mut children = node.children(&mut cursor);
            match (children.next(), children.next()) {
                (Some(inner), None) => type_hint_to_shape(inner, source),
                _ => Shape::Opaque,
            }
        }
        "identifier" => {
            let name = node_text(node, source).unwrap_or_default();
            if let Some(kind) = python_type_to_scalar(&name) {
                Shape::Scalar(kind)
            } else if name == "None" {
                Shape::Scalar(ScalarKind::Unit)
            } else {
                // bytes, Any, custom classes — cannot resolve a shape.
                Shape::Opaque
            }
        }
        "generic_type" => {
            // `<base>[<params>]`: children are the base identifier followed by
            // a `type_parameter` holding the parameter `type` nodes.
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            let base = children
                .iter()
                .find(|c| c.kind() == "identifier")
                .and_then(|c| node_text(*c, source));
            let params: Vec<Shape> = children
                .iter()
                .find(|c| c.kind() == "type_parameter")
                .map(|tp| {
                    let mut pc = tp.walk();
                    tp.children(&mut pc)
                        .filter(|c| c.kind() == "type")
                        .map(|c| type_hint_to_shape(c, source))
                        .collect()
                })
                .unwrap_or_default();
            match base.as_deref() {
                // list[T] / set[T] / frozenset[T] → sequence of T
                Some("list" | "set" | "frozenset") if params.len() == 1 => Shape::Parameterized {
                    base: "Vec".into(),
                    parameters: params,
                },
                // dict[K, V] → map (base matches the Rust frontend's HashMap)
                Some("dict") if params.len() == 2 => Shape::Parameterized {
                    base: "HashMap".into(),
                    parameters: params,
                },
                // tuple[A, B, ...] → product type
                Some("tuple") if !params.is_empty() => Shape::Record(params),
                // Optional[T] → Option[T]
                Some("Optional") if params.len() == 1 => Shape::Parameterized {
                    base: "Option".into(),
                    parameters: params,
                },
                // typing.Union[A, B, ...]
                Some("Union") if !params.is_empty() => Shape::Union(params),
                // Bare containers, partial subscripts (dict[K]), and unknown
                // generic bases (Literal[...], custom generics) — unresolvable.
                _ => Shape::Opaque,
            }
        }
        "binary_operator" | "union_type" => {
            // PEP 604 unions: `A | B`, possibly ending in `None`.
            let arms = union_arms(node, source);
            if arms.is_empty() {
                return Shape::Opaque;
            }
            // `X | None` (either order) → Option[X]
            let unit = Shape::Scalar(ScalarKind::Unit);
            let non_none: Vec<Shape> = arms.iter().filter(|a| **a != unit).cloned().collect();
            if arms.len() == 2 && non_none.len() == 1 {
                Shape::Parameterized {
                    base: "Option".into(),
                    parameters: non_none,
                }
            } else {
                Shape::Union(arms)
            }
        }
        "none" => Shape::Scalar(ScalarKind::Unit),
        // Default-value literal nodes reaching the annotation path — keep the
        // existing degraded mapping (expression shapes are a separate concern).
        "list" | "tuple" | "dictionary" => Shape::Record(vec![Shape::Scalar(ScalarKind::Unit)]),
        _ => Shape::Opaque,
    }
}

/// Flatten a PEP 604 union annotation into its arms (`None` → Unit scalar).
fn union_arms(node: Node, source: &str) -> Vec<Shape> {
    match node.kind() {
        "binary_operator" => {
            let mut arms = Vec::new();
            if let Some(left) = node.child_by_field_name("left") {
                arms.extend(union_arms(left, source));
            }
            if let Some(right) = node.child_by_field_name("right") {
                arms.extend(union_arms(right, source));
            }
            arms
        }
        "union_type" => {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .filter(|c| c.kind() == "type")
                .map(|c| type_hint_to_shape(c, source))
                .collect()
        }
        "none" => vec![Shape::Scalar(ScalarKind::Unit)],
        _ => vec![type_hint_to_shape(node, source)],
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

    // --- Type-hint inference table (vampiro-224.11) ---
    //
    // Subscripted builtins must map to CIR parameterized/record shapes, not
    // degrade to Unit/Record([Unit]). Table mirrors the ScalarKind inference
    // table: one row per annotation form.

    /// Parse `def f() -> <annotation>: pass` and extract the codomain shape.
    fn codomain_of(annotation: &str) -> Shape {
        let source = format!("def f() -> {}:\n    pass\n", annotation);
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(&source, None).unwrap();
        let func = tree.root_node().child(0).unwrap();
        extract_codomain_shape(func, &source)
    }

    /// Parse `def f(x: <annotation>): pass` and extract the domain shape.
    fn domain_of(annotation: &str) -> Shape {
        let source = format!("def f(x: {}):\n    pass\n", annotation);
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(&source, None).unwrap();
        let func = tree.root_node().child(0).unwrap();
        extract_domain_shape(func, &source)
    }

    fn vec_of(inner: Shape) -> Shape {
        Shape::Parameterized {
            base: "Vec".into(),
            parameters: vec![inner],
        }
    }

    fn option_of(inner: Shape) -> Shape {
        Shape::Parameterized {
            base: "Option".into(),
            parameters: vec![inner],
        }
    }

    #[test]
    fn hint_list_str_is_parameterized_vec_of_string() {
        assert_eq!(
            codomain_of("list[str]").normalize(),
            vec_of(Shape::Scalar(ScalarKind::String)).normalize()
        );
    }

    #[test]
    fn hint_list_int_is_parameterized_vec_of_int() {
        assert_eq!(
            domain_of("list[int]").normalize(),
            vec_of(Shape::Scalar(ScalarKind::Int)).normalize()
        );
    }

    #[test]
    fn hint_nested_list_is_recursive() {
        assert_eq!(
            codomain_of("list[list[str]]").normalize(),
            vec_of(vec_of(Shape::Scalar(ScalarKind::String))).normalize()
        );
    }

    #[test]
    fn hint_dict_is_parameterized_map() {
        assert_eq!(
            codomain_of("dict[str, int]").normalize(),
            Shape::Parameterized {
                base: "HashMap".into(),
                parameters: vec![
                    Shape::Scalar(ScalarKind::String),
                    Shape::Scalar(ScalarKind::Int)
                ]
            }
            .normalize()
        );
    }

    #[test]
    fn hint_tuple_is_record_of_arms() {
        assert_eq!(
            codomain_of("tuple[int, str]").normalize(),
            Shape::Record(vec![
                Shape::Scalar(ScalarKind::Int),
                Shape::Scalar(ScalarKind::String)
            ])
            .normalize()
        );
    }

    #[test]
    fn hint_set_is_parameterized_vec() {
        assert_eq!(
            codomain_of("set[int]").normalize(),
            vec_of(Shape::Scalar(ScalarKind::Int)).normalize()
        );
    }

    #[test]
    fn hint_optional_is_option() {
        assert_eq!(
            codomain_of("Optional[str]").normalize(),
            option_of(Shape::Scalar(ScalarKind::String)).normalize()
        );
    }

    #[test]
    fn hint_pep604_optional_is_option() {
        assert_eq!(
            codomain_of("str | None").normalize(),
            option_of(Shape::Scalar(ScalarKind::String)).normalize()
        );
    }

    #[test]
    fn hint_pep604_union_is_union() {
        assert_eq!(
            codomain_of("int | str").normalize(),
            Shape::Union(vec![
                Shape::Scalar(ScalarKind::Int),
                Shape::Scalar(ScalarKind::String)
            ])
            .normalize()
        );
    }

    #[test]
    fn hint_pep604_union_with_none_keeps_unit_arm() {
        assert_eq!(
            codomain_of("int | str | None").normalize(),
            Shape::Union(vec![
                Shape::Scalar(ScalarKind::Int),
                Shape::Scalar(ScalarKind::String),
                Shape::Scalar(ScalarKind::Unit)
            ])
            .normalize()
        );
    }

    #[test]
    fn hint_bare_container_is_opaque_not_unit_record() {
        // `list` without a subscript is untyped — Opaque (unresolvable), not
        // a Record holding a Unit scalar (224.3 rule: unresolvable → Opaque).
        assert_eq!(codomain_of("list"), Shape::Opaque);
        assert_eq!(codomain_of("dict"), Shape::Opaque);
        assert_eq!(codomain_of("tuple"), Shape::Opaque);
    }

    #[test]
    fn hint_any_and_custom_class_are_opaque() {
        assert_eq!(codomain_of("Any"), Shape::Opaque);
        assert_eq!(codomain_of("SectionSync"), Shape::Opaque);
    }

    #[test]
    fn hint_none_annotation_is_unit() {
        assert_eq!(codomain_of("None"), Shape::Scalar(ScalarKind::Unit));
    }

    #[test]
    fn hint_scalars_unchanged() {
        assert_eq!(codomain_of("int"), Shape::Scalar(ScalarKind::Int));
        assert_eq!(codomain_of("str"), Shape::Scalar(ScalarKind::String));
        assert_eq!(codomain_of("float"), Shape::Scalar(ScalarKind::Float));
        assert_eq!(codomain_of("bool"), Shape::Scalar(ScalarKind::Bool));
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
