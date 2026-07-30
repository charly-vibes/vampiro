//! Clojure CIR extraction logic.
//!
//! Walks the tree-sitter CST and emits CIR nodes, edges, shapes, and effects.
//! Clojure is homoiconic — everything is a list literal. The first element of
//! a list determines whether it's a declaration (defn, fn, def, etc.) or a
//! function call.

use std::path::Path;
use tree_sitter::Node;
use vampiro_cir::{NodeKind, ScalarKind, 
    CirEdge, CirGraph, CirNode, EffectChannel, EffectResolution, Provenance, Shape, SourceSpan,
    StableId, TrustProvenance,
};

/// Extract a CIR graph from a tree-sitter parsed Clojure source.
pub fn extract_graph(root: Node, source: &str, path: &Path) -> CirGraph {
    let file_path = path.to_string_lossy().to_string();
    let mut graph = CirGraph::new(&file_path);
    let mut node_counter: u64 = 0;
    let mut edge_counter: u64 = 0;

    // Walk the source children (top-level forms)
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        process_form(
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

/// Process a form (top-level expression), optionally with a binding name.
#[allow(clippy::too_many_arguments)]
fn process_form(
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
        "list_lit" => {
            process_list_lit(
                node,
                source,
                file_path,
                graph,
                node_counter,
                edge_counter,
                call_depth,
                binding_name,
            );
        }
        "vec_lit" | "map_lit" | "set_lit" => {
            // Data literals — recurse for nested calls
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                process_form(
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
        "anon_fn_lit" => {
            // #(body) — anonymous function literal
            let name = binding_name.unwrap_or("<#(fn)>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!("clj:{}:{}:anon_{}", file_path, name, *node_counter));

            let cir_node = CirNode {
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
            };

            graph.add_node(cir_node);
            *node_counter += 1;
        }
        _ => {
            // Recurse into children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                process_form(
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

/// Process a list literal, which may be a special form, function call, or data.
#[allow(clippy::too_many_arguments)]
fn process_list_lit(
    node: Node,
    source: &str,
    file_path: &str,
    graph: &mut CirGraph,
    node_counter: &mut u64,
    edge_counter: &mut u64,
    call_depth: u32,
    binding_name: Option<&str>,
) {
    // Get the first named child to determine the operator
    // Index 0 is `(`, index 1 is the operator symbol
    let first = node.child(1);
    let operator = first.and_then(|n| get_symbol_text(n, source));

    match operator {
        // ---- Declaration special forms ----
        Some("defn") => {
            let name = extract_defn_name(node, source).unwrap_or("<anonymous>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!("clj:{}:{}", file_path, name));

            // Detect visibility
            let _is_private = name.ends_with('-');

            // Detect effects in body
            let effect = detect_clojure_effect(node, source);

            let cir_node = CirNode {
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
            };

            graph.add_node(cir_node);
            *node_counter += 1;

            // Process body for calls
            process_body_for_calls(
                node,
                source,
                file_path,
                &id,
                graph,
                edge_counter,
                call_depth + 1,
            );
        }
        Some("defn-") => {
            // Private function definition
            let name = extract_defn_name(node, source).unwrap_or("<anonymous>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!("clj:{}:{}", file_path, name));

            let cir_node = CirNode {
                id: id.clone(),
                domain: Shape::Scalar(ScalarKind::Unit),
                codomain: Shape::Scalar(ScalarKind::Unit),
                effect: EffectChannel::Plain,
                span,
                name: Some(name.to_string()),
                trust_provenance: TrustProvenance::default(),
                is_test: false,
                kind: NodeKind::Declaration,
                containing_function: None,
            };

            graph.add_node(cir_node);
            *node_counter += 1;

            process_body_for_calls(
                node,
                source,
                file_path,
                &id,
                graph,
                edge_counter,
                call_depth + 1,
            );
        }
        Some("fn") => {
            // Anonymous function
            let name = binding_name.unwrap_or("<fn>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!("clj:{}:{}:fn_{}", file_path, name, *node_counter));

            let cir_node = CirNode {
                id: id.clone(),
                domain: Shape::Scalar(ScalarKind::Unit),
                codomain: Shape::Scalar(ScalarKind::Unit),
                effect: EffectChannel::Plain,
                span,
                name: Some(name.to_string()),
                trust_provenance: TrustProvenance::default(),
                is_test: false,
                kind: NodeKind::Declaration,
                containing_function: None,
            };

            graph.add_node(cir_node);
            *node_counter += 1;

            process_body_for_calls(
                node,
                source,
                file_path,
                &id,
                graph,
                edge_counter,
                call_depth + 1,
            );
        }
        Some("def") => {
            // Variable definition — may be bound to a function value
            let name = extract_defn_name(node, source).unwrap_or("<anonymous>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!("clj:{}:{}", file_path, name));

            let effect = detect_clojure_effect(node, source);

            let cir_node = CirNode {
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
            };

            graph.add_node(cir_node);
            *node_counter += 1;

            // Check if the value is a fn/anonymous function
            let value = node.child(3); // (def name value) — value is at index 3 (after (, def, name)
            if let Some(val) = value {
                process_form(
                    val,
                    source,
                    file_path,
                    graph,
                    node_counter,
                    edge_counter,
                    call_depth + 1,
                    Some(name),
                );
            }
        }
        Some("defmulti") | Some("defmethod") | Some("defprotocol") | Some("defrecord")
        | Some("deftype") => {
            let name = extract_defn_name(node, source).unwrap_or("<anonymous>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!("clj:{}:{}", file_path, name));

            let cir_node = CirNode {
                id: id.clone(),
                domain: Shape::Scalar(ScalarKind::Unit),
                codomain: Shape::Scalar(ScalarKind::Unit),
                effect: EffectChannel::Plain,
                span,
                name: Some(name.to_string()),
                trust_provenance: TrustProvenance::default(),
                is_test: false,
                kind: NodeKind::Declaration,
                containing_function: None,
            };

            graph.add_node(cir_node);
            *node_counter += 1;

            process_body_for_calls(
                node,
                source,
                file_path,
                &id,
                graph,
                edge_counter,
                call_depth + 1,
            );
        }
        Some("future") => {
            // (future body) — creates an async computation
            // If bound to a name via def, the node is already created.
            // Otherwise, create an anonymous node.
            let name = binding_name.unwrap_or("<future>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!(
                "clj:{}:{}:future_{}",
                file_path, name, *node_counter
            ));

            let cir_node = CirNode {
                id: id.clone(),
                domain: Shape::Scalar(ScalarKind::Unit),
                codomain: Shape::Scalar(ScalarKind::Unit),
                effect: EffectChannel::Recursive(Box::new(EffectChannel::Async)),
                span,
                name: Some(name.to_string()),
                trust_provenance: TrustProvenance::default(),
                is_test: false,
                kind: NodeKind::Declaration,
                containing_function: None,
            };

            graph.add_node(cir_node);
            *node_counter += 1;

            process_body_for_calls(
                node,
                source,
                file_path,
                &id,
                graph,
                edge_counter,
                call_depth + 1,
            );
        }
        Some("lazy-seq") => {
            let name = binding_name.unwrap_or("<lazy-seq>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!("clj:{}:{}:lazy_{}", file_path, name, *node_counter));

            let cir_node = CirNode {
                id: id.clone(),
                domain: Shape::Scalar(ScalarKind::Unit),
                codomain: Shape::Scalar(ScalarKind::Unit),
                effect: EffectChannel::Recursive(Box::new(EffectChannel::Stream)),
                span,
                name: Some(name.to_string()),
                trust_provenance: TrustProvenance::default(),
                is_test: false,
                kind: NodeKind::Declaration,
                containing_function: None,
            };

            graph.add_node(cir_node);
            *node_counter += 1;

            process_body_for_calls(
                node,
                source,
                file_path,
                &id,
                graph,
                edge_counter,
                call_depth + 1,
            );
        }
        Some("try") => {
            // (try body (catch Exception e ...) ...)
            let name = binding_name.unwrap_or("<try>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!("clj:{}:{}:try_{}", file_path, name, *node_counter));

            let cir_node = CirNode {
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
            };

            graph.add_node(cir_node);
            *node_counter += 1;

            process_body_for_calls(
                node,
                source,
                file_path,
                &id,
                graph,
                edge_counter,
                call_depth + 1,
            );
        }
        Some("with-open") | Some("binding") => {
            let name = binding_name.unwrap_or("<resource>");
            let span = node_span(node, file_path);
            let id = StableId::new(format!(
                "clj:{}:{}:resource_{}",
                file_path, name, *node_counter
            ));

            let cir_node = CirNode {
                id: id.clone(),
                domain: Shape::Scalar(ScalarKind::Unit),
                codomain: Shape::Scalar(ScalarKind::Unit),
                effect: EffectChannel::Recursive(Box::new(EffectChannel::Throws)),
                span,
                name: Some(name.to_string()),
                trust_provenance: TrustProvenance::default(),
                is_test: false,
                kind: NodeKind::Declaration,
                containing_function: None,
            };

            graph.add_node(cir_node);
            *node_counter += 1;

            process_body_for_calls(
                node,
                source,
                file_path,
                &id,
                graph,
                edge_counter,
                call_depth + 1,
            );
        }
        // ---- Function calls ----
        Some(_) => {
            // Any other list with a symbol as first element is a function call
            let callee_name = operator.unwrap_or("<unknown>");
            let _callee_id = StableId::new(format!("clj:{}:{}", file_path, callee_name));

            let _span = node_span(node, file_path);
            let _provenance = if call_depth <= 3 {
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

            // If there's a caller context, add an edge
            // (call edges are handled by the caller's body processing)

            // Recurse into arguments for nested calls
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                process_form(
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
        None => {
            // Empty list or non-symbol first element — recurse
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                process_form(
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

/// Process a body for call edges, extracting edges from inner list forms.
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

/// Extract call edges from a form, recursing into nested structures.
fn extract_call_edges(
    node: Node,
    source: &str,
    file_path: &str,
    caller_id: &StableId,
    graph: &mut CirGraph,
    edge_counter: &mut u64,
    call_depth: u32,
) {
    match node.kind() {
        "list_lit" => {
            let first = node.child(1);
            let operator = first.and_then(|n| get_symbol_text(n, source));

            // Skip special forms that are declarations, not calls
            let is_special_form = matches!(
                operator,
                Some(
                    "defn"
                        | "defn-"
                        | "fn"
                        | "def"
                        | "defmulti"
                        | "defmethod"
                        | "defprotocol"
                        | "defrecord"
                        | "deftype"
                        | "future"
                        | "lazy-seq"
                        | "try"
                        | "catch"
                        | "with-open"
                        | "binding"
                        | "let"
                        | "loop"
                        | "if"
                        | "when"
                        | "cond"
                        | "case"
                        | "do"
                        | "->"
                        | "->>"
                        | "as->"
                        | "some->"
                        | "some->>"
                )
            );

            if !is_special_form {
                // This is a function call — add an edge
                if let Some(op) = operator {
                    let callee_id = StableId::new(format!("clj:{}:{}", file_path, op));
                    let span = node_span(node, file_path);

                    // Skip edges to nodes not in the graph (e.g. macros,
                    // constructors like ->Response, or builtins like go-loop).
                    if graph.node_by_id(&callee_id).is_none() {
                        // Still recurse into arguments for nested calls.
                        let mut cursor = node.walk();
                        for child in node.children(&mut cursor) {
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
                        return;
                    }
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
                        id: StableId::new(format!("clj:edge:{}", *edge_counter)),
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
                    };

                    graph.add_edge(edge);
                    *edge_counter += 1;
                }

                // Recurse into arguments
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
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
                // For special forms, recurse into children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
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
            }
        }
        "vec_lit" | "map_lit" | "set_lit" | "anon_fn_lit" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
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
        _ => {
            // Recurse into children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
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
}

/// Detect the effect channel for a Clojure form.
fn detect_clojure_effect(node: Node, source: &str) -> EffectChannel {
    let mut has_async = false;
    let mut has_stream = false;
    let mut has_result = false;
    let mut has_resource = false;

    scan_clojure_effects(
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

/// Scan a Clojure form for effect-related forms.
#[allow(clippy::too_many_arguments)]
fn scan_clojure_effects(
    node: Node,
    source: &str,
    has_async: &mut bool,
    has_stream: &mut bool,
    has_result: &mut bool,
    has_resource: &mut bool,
) {
    if node.kind() == "list_lit" {
        let first = node.child(1);
        let operator = first.and_then(|n| get_symbol_text(n, source));

        match operator {
            Some("future" | "promise") => *has_async = true,
            Some("lazy-seq") => *has_stream = true,
            Some("try" | "catch") => *has_result = true,
            Some("with-open" | "binding") => *has_resource = true,
            _ => {}
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        scan_clojure_effects(
            child,
            source,
            has_async,
            has_stream,
            has_result,
            has_resource,
        );
    }
}

/// Extract the function name from a defn-like form.
/// (defn name [args] body) → "name"
fn extract_defn_name<'a>(node: Node, source: &'a str) -> Option<&'a str> {
    // The name is the third child (index 2) — after '(' and the operator symbol
    let name_node = node.child(2)?;
    get_symbol_text(name_node, source)
}

/// Get the text of a symbol literal node.
fn get_symbol_text<'a>(node: Node, source: &'a str) -> Option<&'a str> {
    match node.kind() {
        "sym_lit" | "sym_val_lit" => {
            // sym_lit has a child sym_name that contains the actual text
            if let Some(name_node) = node.child(0) {
                if name_node.kind() == "sym_name" {
                    let start = name_node.start_byte();
                    let end = name_node.end_byte();
                    if start < end && end <= source.len() {
                        return Some(&source[start..end]);
                    }
                }
            }
            // Fallback: use the node's own text
            let start = node.start_byte();
            let end = node.end_byte();
            if start < end && end <= source.len() {
                Some(&source[start..end])
            } else {
                None
            }
        }
        "kwd_lit" => {
            let start = node.start_byte();
            let end = node.end_byte();
            if start < end && end <= source.len() {
                Some(&source[start..end])
            } else {
                None
            }
        }
        _ => None,
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
    fn test_get_symbol_text_returns_text() {
        let source = "(defn greet [name] (str \"Hello\" name))";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_clojure::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // First child should be the list_lit for defn
        let list_node = root.child(0).unwrap();
        assert_eq!(list_node.kind(), "list_lit");

        // First child of list should be sym_lit "defn"
        let defn_sym = list_node.child(1).unwrap();
        assert_eq!(get_symbol_text(defn_sym, source), Some("defn"));

        // Second child should be the name "greet"
        let name_sym = list_node.child(2).unwrap();
        assert_eq!(get_symbol_text(name_sym, source), Some("greet"));
    }

    #[test]
    fn test_extract_defn_name() {
        let source = "(defn greet [name] (str \"Hello\" name))";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_clojure::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let list_node = root.child(0).unwrap();
        assert_eq!(extract_defn_name(list_node, source), Some("greet"));
    }
}
