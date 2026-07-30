//! Julia CIR extraction logic.
//!
//! Walks the tree-sitter CST and emits CIR nodes, edges, shapes, and effects.
//! Note: tree-sitter-julia 0.23.1 does not use named fields for most nodes.
//! Children must be accessed by index.

use std::collections::HashMap;
use std::path::Path;
use tree_sitter::Node;
use vampiro_cir::{
    CirEdge, CirGraph, CirNode, EffectChannel, EffectResolution, NodeKind, Provenance, ScalarKind,
    Shape, SourceSpan, StableId, TrustProvenance,
};

/// Extract a CIR graph from a tree-sitter parsed Julia module.
pub fn extract_graph(root: Node, source: &str, path: &Path) -> CirGraph {
    let file_path = path.to_string_lossy().to_string();
    let mut graph = CirGraph::new(&file_path);
    let mut node_counter: u64 = 0;
    let mut edge_counter: u64 = 0;
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
            None,
            &mut HashMap::new(),
        );
    }
    graph
}

#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
fn process_node(
    node: Node,
    source: &str,
    file_path: &str,
    graph: &mut CirGraph,
    node_counter: &mut u64,
    edge_counter: &mut u64,
    call_depth: u32,
    binding_name: Option<&str>,
    local_shapes: &mut HashMap<String, Shape>,
) {
    match node.kind() {
        "function_definition" | "macro_definition" => {
            // Find the name: after `function` keyword, before `(` or `::`
            let name =
                find_decl_name(node, source).unwrap_or(binding_name.unwrap_or("<anonymous>"));
            let span = node_span(node, file_path);
            // Disambiguate anonymous functions with a counter.
            let id = if name == "<anonymous>" {
                StableId::new(format!("jl:{}:{}:fn_{}", file_path, name, *node_counter))
            } else {
                StableId::new(format!("jl:{}:{}", file_path, name))
            };
            let effect = detect_julia_effect(node, source);
            graph.add_node(CirNode {
                id: id.clone(),
                domain: Shape::Scalar(ScalarKind::Unit),
                codomain: Shape::Scalar(ScalarKind::Unit),
                effect,
                span,
                name: Some(name.to_string()),
                trust_provenance: TrustProvenance::default(),
                is_test: false,
                kind: NodeKind::Declaration,
                containing_function: None,
            });
            *node_counter += 1;
            // Extract call edges from body
            let mut c = node.walk();
            for child in node.children(&mut c) {
                extract_call_edges(
                    child,
                    source,
                    file_path,
                    &id,
                    graph,
                    node_counter,
                    edge_counter,
                    call_depth + 1,
                    local_shapes,
                );
            }
        }
        "struct_definition" | "primitive_definition" | "abstract_definition" => {
            let name = find_decl_name(node, source).unwrap_or("<anonymous>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!("jl:{}:{}", file_path, name));
            graph.add_node(CirNode {
                id,
                domain: Shape::Scalar(ScalarKind::Unit),
                codomain: Shape::Scalar(ScalarKind::Unit),
                effect: EffectChannel::Plain,
                span,
                name: Some(name.to_string()),
                trust_provenance: TrustProvenance::default(),
                is_test: false,
                kind: NodeKind::Declaration,
                containing_function: None,
            });
            *node_counter += 1;
        }
        "module_definition" => {
            let name = node
                .child_by_field_name("name")
                .and_then(|n| node_text(n, source))
                .unwrap_or_else(|| "<anonymous>".to_string());
            let span = node_span(node, file_path);
            let id = StableId::new(format!("jl:{}:{}", file_path, name));
            graph.add_node(CirNode {
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
            });
            *node_counter += 1;
            // Recurse into module body
            let mut c = node.walk();
            for child in node.children(&mut c) {
                process_node(
                    child,
                    source,
                    file_path,
                    graph,
                    node_counter,
                    edge_counter,
                    call_depth + 1,
                    None,
                    local_shapes,
                );
            }
        }
        "arrow_function_expression" => {
            let name = binding_name.unwrap_or("<lambda>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!(
                "jl:{}:{}:lambda_{}",
                file_path, name, *node_counter
            ));
            graph.add_node(CirNode {
                id,
                domain: Shape::Scalar(ScalarKind::Unit),
                codomain: Shape::Scalar(ScalarKind::Unit),
                effect: EffectChannel::Plain,
                span,
                name: Some(name.to_string()),
                trust_provenance: TrustProvenance::default(),
                is_test: false,
                kind: NodeKind::Declaration,
                containing_function: None,
            });
            *node_counter += 1;
        }
        "assignment" => {
            // Track local variable shapes: x = expr
            if let Some(left) = node.child(0) {
                if let Some(var_name) = node_text(left, source) {
                    if let Some(right) = node.child(2) {
                        // Infer the shape of the right-hand side
                        if let Some(shape) = extract_expr_shape(right, source, graph) {
                            local_shapes.insert(var_name.clone(), shape);
                        }
                        // Process the right-hand side for calls and nested declarations
                        process_node(
                            right,
                            source,
                            file_path,
                            graph,
                            node_counter,
                            edge_counter,
                            call_depth + 1,
                            Some(&var_name),
                            local_shapes,
                        );
                    }
                }
            }
        }
        "call_expression" | "broadcast_call_expression" | "macrocall_expression" => {
            // Handled by extract_call_edges — but also recurse for nested calls
            let mut c = node.walk();
            for child in node.children(&mut c) {
                process_node(
                    child,
                    source,
                    file_path,
                    graph,
                    node_counter,
                    edge_counter,
                    call_depth + 1,
                    None,
                    local_shapes,
                );
            }
        }
        "try_statement" => {
            let name = binding_name.unwrap_or("<try>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!("jl:{}:{}:try_{}", file_path, name, *node_counter));
            graph.add_node(CirNode {
                id: id.clone(),
                domain: Shape::Scalar(ScalarKind::Unit),
                codomain: Shape::Scalar(ScalarKind::Unit),
                effect: EffectChannel::Recursive(Box::new(EffectChannel::Result)),
                span,
                name: Some(name.to_string()),
                trust_provenance: TrustProvenance::default(),
                is_test: false,
                kind: NodeKind::Declaration,
                containing_function: None,
            });
            *node_counter += 1;
            let mut c = node.walk();
            for child in node.children(&mut c) {
                extract_call_edges(
                    child,
                    source,
                    file_path,
                    &id,
                    graph,
                    node_counter,
                    edge_counter,
                    call_depth + 1,
                    local_shapes,
                );
            }
        }
        "compound_statement" | "source_file" => {
            let mut c = node.walk();
            for child in node.children(&mut c) {
                process_node(
                    child,
                    source,
                    file_path,
                    graph,
                    node_counter,
                    edge_counter,
                    call_depth,
                    None,
                    local_shapes,
                );
            }
        }
        _ => {
            let mut c = node.walk();
            for child in node.children(&mut c) {
                process_node(
                    child,
                    source,
                    file_path,
                    graph,
                    node_counter,
                    edge_counter,
                    call_depth,
                    None,
                    local_shapes,
                );
            }
        }
    }
}

/// Find the name of a declaration (function, struct, etc.) by looking for
/// the first identifier child after the keyword.
fn find_decl_name<'a>(node: Node, source: &'a str) -> Option<&'a str> {
    // Check direct children for identifier
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return node_text_as_str(child, source);
        }
        // Check inside signature or type_head (for function_definition, struct_definition)
        if child.kind() == "signature" || child.kind() == "type_head" {
            if let Some(name) = find_name_inside(child, source) {
                return Some(name);
            }
        }
    }
    None
}

/// Find a name by looking inside a node for call_expression > identifier pattern.
fn find_name_inside<'a>(node: Node, source: &'a str) -> Option<&'a str> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return node_text_as_str(child, source);
        }
        if child.kind() == "call_expression" || child.kind() == "scoped_identifier" {
            // Look for identifier inside the call expression (e.g., `greet()`)
            let mut c2 = child.walk();
            for inner in child.children(&mut c2) {
                if inner.kind() == "identifier" {
                    return node_text_as_str(inner, source);
                }
            }
        }
    }
    None
}

#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
fn extract_call_edges(
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
    let kind = node.kind();
    if kind == "call_expression"
        || kind == "broadcast_call_expression"
        || kind == "macrocall_expression"
    {
        // The function is the first child (index 0)
        if let Some(func) = node.child(0) {
            if let Some(name) = node_text(func, source) {
                let callee_id = StableId::new(format!("jl:{}:{}", file_path, name));
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

                // Emit a single declaration->declaration edge for the return-boundary
                // check (no slot). This preserves the existing codomain comparison.
                graph.add_edge(CirEdge {
                    id: StableId::new(format!("jl:edge:{}", *edge_counter)),
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
                });
                *edge_counter += 1;

                // Emit expression->declaration edges for each argument with a known
                // shape. In Julia, arguments start at index 1 (after the function at index 0).
                let mut arg_cursor = node.walk();
                let arg_nodes: Vec<Node> = node.children(&mut arg_cursor).collect();
                // arg_nodes[0] = function, arg_nodes[1..] = arguments
                for (i, arg) in arg_nodes.iter().enumerate().skip(1) {
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
                        graph.add_edge(CirEdge {
                            id: StableId::new(format!("jl:edge:expr_{}", *edge_counter)),
                            source: expr_id,
                            target: callee_id.clone(),
                            resolution: EffectResolution::Propagated,
                            unwrap_evidence: None,
                            provenance: provenance.clone(),
                            span: node_span(*arg, file_path),
                            discard_spans: vec![],
                            trust_provenance: TrustProvenance::default(),
                            slot: Some(i as u32 - 1),
                            arg_shape: None,
                        });
                        *edge_counter += 1;
                    }
                }
            }
        }
        let mut c = node.walk();
        for child in node.children(&mut c) {
            extract_call_edges(
                child,
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
    } else {
        let mut c = node.walk();
        for child in node.children(&mut c) {
            extract_call_edges(
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

/// Emit an expression node for an intermediate expression value.
fn emit_expression_node(
    shape: Shape,
    node: Node,
    _source: &str,
    file_path: &str,
    graph: &mut CirGraph,
    node_counter: &mut u64,
    containing_fn: &StableId,
) -> StableId {
    let id = StableId::new(format!(
        "jl:expr:{}:{:?}:{}",
        file_path,
        node.id(),
        *node_counter
    ));
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

/// Infer the shape of a Julia expression from a tree-sitter AST node.
fn extract_expr_shape(node: Node, source: &str, graph: &CirGraph) -> Option<Shape> {
    match node.kind() {
        // Integer literal -> Scalar(Int)
        "integer_literal" => Some(Shape::Scalar(ScalarKind::Int)),
        // Float literal -> Scalar(Float)
        "float_literal" => Some(Shape::Scalar(ScalarKind::Float)),
        // String literal -> Scalar(String)
        "string_literal" | "command_string" | "triple_string_literal" => {
            Some(Shape::Scalar(ScalarKind::String))
        }
        // Boolean literal -> Scalar(Bool)
        "true" => Some(Shape::Scalar(ScalarKind::Bool)),
        "false" => Some(Shape::Scalar(ScalarKind::Bool)),
        // Nothing literal -> Scalar(Unit)
        "nothing_literal" => Some(Shape::Scalar(ScalarKind::Unit)),
        // Char literal -> Scalar(Char)
        "char_literal" => Some(Shape::Scalar(ScalarKind::Char)),
        // Call expression: use the callee's codomain if resolvable
        "call_expression" | "broadcast_call_expression" => {
            if let Some(func) = node.child(0) {
                if let Some(name) = node_text(func, source) {
                    let callee_id = StableId::new(format!("jl:{}:{}", graph.source_file, name));
                    if let Some(callee_node) = graph.node_by_id(&callee_id) {
                        return Some(callee_node.codomain.clone());
                    }
                }
            }
            None
        }
        // Identifier reference: check if tracked via local_shapes
        // (local_shapes not available here — caller handles this)
        "identifier" => None,
        // Everything else: opaque
        _ => None,
    }
}

#[allow(clippy::only_used_in_recursion)]
fn detect_julia_effect(node: Node, source: &str) -> EffectChannel {
    let mut has_async = false;
    let mut has_stream = false;
    let mut has_result = false;
    let mut has_resource = false;
    scan_julia_effects(
        node,
        source,
        &mut has_async,
        &mut has_stream,
        &mut has_result,
        &mut has_resource,
    );
    let mut effect = EffectChannel::Plain;
    if has_stream {
        effect = EffectChannel::Recursive(Box::new(EffectChannel::Stream));
    }
    if has_async {
        effect = EffectChannel::Recursive(Box::new(EffectChannel::Async));
    }
    if has_result {
        effect = EffectChannel::Recursive(Box::new(EffectChannel::Result));
    }
    if has_resource {
        effect = EffectChannel::Recursive(Box::new(EffectChannel::Throws));
    }
    effect
}

#[allow(clippy::only_used_in_recursion, clippy::too_many_arguments)]
fn scan_julia_effects(
    node: Node,
    source: &str,
    has_async: &mut bool,
    has_stream: &mut bool,
    has_result: &mut bool,
    has_resource: &mut bool,
) {
    match node.kind() {
        "try_statement" => *has_result = true,
        "macrocall_expression" => {
            if let Some(func) = node.child(0) {
                if let Some(name) = node_text(func, source) {
                    let name = name.trim();
                    if name == "@async" || name == "@sync" || name == "@spawn" {
                        *has_async = true;
                    }
                }
            }
        }
        _ => {}
    }
    let mut c = node.walk();
    for child in node.children(&mut c) {
        scan_julia_effects(
            child,
            source,
            has_async,
            has_stream,
            has_result,
            has_resource,
        );
    }
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

fn node_text_as_str<'a>(node: Node, source: &'a str) -> Option<&'a str> {
    let start = node.start_byte();
    let end = node.end_byte();
    if start < end && end <= source.len() {
        Some(&source[start..end])
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

    #[test]
    fn test_node_text() {
        let source = "function greet() end";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_julia::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let func = root.child(0).unwrap();
        assert_eq!(func.kind(), "function_definition");
        let name = find_decl_name(func, source);
        assert_eq!(name, Some("greet"));
    }

    #[test]
    fn test_find_decl_name_struct() {
        let source = "struct Point\n    x::Float64\nend";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_julia::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let struct_node = root.child(0).unwrap();
        assert_eq!(struct_node.kind(), "struct_definition");
        let name = find_decl_name(struct_node, source);
        assert_eq!(name, Some("Point"));
    }
}
