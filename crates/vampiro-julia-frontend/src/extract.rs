//! Julia CIR extraction logic.
//!
//! Walks the tree-sitter CST and emits CIR nodes, edges, shapes, and effects.
//! Note: tree-sitter-julia 0.23.1 does not use named fields for most nodes.
//! Children must be accessed by index.

use std::path::Path;
use tree_sitter::Node;
use vampiro_cir::{
    CirEdge, CirGraph, CirNode, EffectChannel, EffectResolution, Provenance, Shape, SourceSpan,
    StableId, TrustProvenance,
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
        );
    }
    graph
}

#[allow(clippy::too_many_arguments)]
fn process_node(
    node: Node,
    source: &str,
    file_path: &str,
    graph: &mut CirGraph,
    node_counter: &mut u64,
    edge_counter: &mut u64,
    call_depth: u32,
    binding_name: Option<&str>,
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
                domain: Shape::Scalar,
                codomain: Shape::Scalar,
                effect,
                span,
                name: Some(name.to_string()),
                trust_provenance: TrustProvenance::default(),
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
                    edge_counter,
                    call_depth + 1,
                );
            }
        }
        "struct_definition" | "primitive_definition" | "abstract_definition" => {
            let name = find_decl_name(node, source).unwrap_or("<anonymous>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!("jl:{}:{}", file_path, name));
            graph.add_node(CirNode {
                id,
                domain: Shape::Scalar,
                codomain: Shape::Scalar,
                effect: EffectChannel::Plain,
                span,
                name: Some(name.to_string()),
                trust_provenance: TrustProvenance::default(),
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
                domain: Shape::Scalar,
                codomain: Shape::Scalar,
                effect: EffectChannel::Plain,
                span,
                name: Some(name),
                trust_provenance: TrustProvenance::default(),
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
                domain: Shape::Scalar,
                codomain: Shape::Scalar,
                effect: EffectChannel::Plain,
                span,
                name: Some(name.to_string()),
                trust_provenance: TrustProvenance::default(),
            });
            *node_counter += 1;
        }
        "assignment" => {
            if let Some(left) = node.child(0) {
                if let Some(name) = node_text(left, source) {
                    if let Some(right) = node.child(2) {
                        process_node(
                            right,
                            source,
                            file_path,
                            graph,
                            node_counter,
                            edge_counter,
                            call_depth + 1,
                            Some(&name),
                        );
                    }
                }
            }
        }
        "call_expression" | "broadcast_call_expression" | "macrocall_expression" => {
            // Handled by extract_call_edges
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
                );
            }
        }
        "try_statement" => {
            let name = binding_name.unwrap_or("<try>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!("jl:{}:{}:try_{}", file_path, name, *node_counter));
            graph.add_node(CirNode {
                id: id.clone(),
                domain: Shape::Scalar,
                codomain: Shape::Scalar,
                effect: EffectChannel::Recursive(Box::new(EffectChannel::Result)),
                span,
                name: Some(name.to_string()),
                trust_provenance: TrustProvenance::default(),
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
                    edge_counter,
                    call_depth + 1,
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

fn node_text_as_str<'a>(node: Node, source: &'a str) -> Option<&'a str> {
    let start = node.start_byte();
    let end = node.end_byte();
    if start < end && end <= source.len() {
        Some(&source[start..end])
    } else {
        None
    }
}

fn extract_call_edges(
    node: Node,
    source: &str,
    file_path: &str,
    caller_id: &StableId,
    graph: &mut CirGraph,
    edge_counter: &mut u64,
    call_depth: u32,
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
                graph.add_edge(CirEdge {
                    id: StableId::new(format!("jl:edge:{}", *edge_counter)),
                    source: caller_id.clone(),
                    target: callee_id,
                    resolution: EffectResolution::Propagated,
                    unwrap_evidence: None,
                    provenance,
                    span,
                    discard_spans: vec![],
                    trust_provenance: TrustProvenance::default(),
                    slot: None,
                    arg_shape: None,
                });
                *edge_counter += 1;
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
                edge_counter,
                call_depth + 1,
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
                edge_counter,
                call_depth,
            );
        }
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
