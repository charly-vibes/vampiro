//! CIR extraction from syn's AST.
//!
//! Walks a `syn::File` and produces a `CirGraph` with nodes for
//! function/closure declarations and edges for call sites.

use std::collections::HashMap;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use vampiro_cir::{
    CirEdge, CirGraph, CirNode, EffectChannel, EffectResolution, Provenance, Shape, SourceSpan,
    StableId, Totality, UnwrapEvidence, UnwrapKind,
};

/// Extract a `CirGraph` from a parsed syn file.
pub fn extract_graph(syntax: &syn::File, path: &Path) -> CirGraph {
    let mut extractor = Extractor {
        graph: CirGraph::new(path.to_string_lossy()),
        nodes: HashMap::new(),
        current_function: None,
        path: path.to_path_buf(),
    };
    visit::visit_file(&mut extractor, syntax);
    extractor.graph
}

/// The extraction visitor.
struct Extractor {
    graph: CirGraph,
    /// Map from function name to node stable ID.
    nodes: HashMap<String, StableId>,
    /// The current function being visited (for edge source resolution).
    current_function: Option<String>,
    path: std::path::PathBuf,
}

impl Extractor {
    /// Extract a shape from a syn type.
    fn extract_shape(&self, ty: &syn::Type) -> Shape {
        match ty {
            syn::Type::Path(type_path) => {
                let last_segment = type_path.path.segments.last();
                match last_segment {
                    Some(seg) => {
                        let ident = seg.ident.to_string();
                        match ident.as_str() {
                            "bool" | "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16"
                            | "u32" | "u64" | "u128" | "f32" | "f64" | "usize" | "isize"
                            | "char" | "String" | "str" => Shape::Scalar,
                            _ => {
                                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                                    let params: Vec<Shape> = args
                                        .args
                                        .iter()
                                        .filter_map(|arg| match arg {
                                            syn::GenericArgument::Type(t) => {
                                                Some(self.extract_shape(t))
                                            }
                                            _ => None,
                                        })
                                        .collect();
                                    if params.is_empty() {
                                        Shape::Scalar
                                    } else {
                                        Shape::Parameterized {
                                            base: ident,
                                            parameters: params,
                                        }
                                    }
                                } else {
                                    Shape::Scalar
                                }
                            }
                        }
                    }
                    None => Shape::Opaque,
                }
            }
            syn::Type::Reference(type_ref) => {
                let inner = self.extract_shape(&type_ref.elem);
                Shape::Ref(Box::new(inner))
            }
            syn::Type::Tuple(tuple) => {
                let elems: Vec<Shape> = tuple.elems.iter().map(|t| self.extract_shape(t)).collect();
                if elems.len() == 1 {
                    elems.into_iter().next().unwrap_or(Shape::Scalar)
                } else if elems.is_empty() {
                    Shape::Scalar // unit type
                } else {
                    Shape::Record(elems)
                }
            }
            syn::Type::Slice(slice) => {
                let inner = self.extract_shape(&slice.elem);
                Shape::Parameterized {
                    base: "slice".into(),
                    parameters: vec![inner],
                }
            }
            syn::Type::Array(arr) => {
                let inner = self.extract_shape(&arr.elem);
                Shape::Parameterized {
                    base: "array".into(),
                    parameters: vec![inner],
                }
            }
            syn::Type::ImplTrait(_) => Shape::Opaque,
            syn::Type::TraitObject(_) => Shape::Opaque,
            syn::Type::Never(_) => Shape::Scalar,
            _ => Shape::Opaque,
        }
    }

    /// Extract the effect channel from a return type.
    fn extract_effect(&self, output: &syn::ReturnType) -> EffectChannel {
        match output {
            syn::ReturnType::Default => EffectChannel::Plain,
            syn::ReturnType::Type(_, ty) => self.extract_effect_from_type(ty),
        }
    }

    /// Recognize effect wrappers in a type.
    fn extract_effect_from_type(&self, ty: &syn::Type) -> EffectChannel {
        match ty {
            syn::Type::Path(type_path) => {
                let last_segment = type_path.path.segments.last();
                match last_segment {
                    Some(seg) => {
                        let ident = seg.ident.to_string();
                        match ident.as_str() {
                            "Result" => EffectChannel::Result,
                            "Option" => EffectChannel::Option,
                            "Vec" | "Box" | "String" | "str" | "bool" | "i8" | "i16" | "i32"
                            | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64" | "usize"
                            | "isize" | "char" => EffectChannel::Plain,
                            _ => {
                                // Check if it's a generic wrapping a known effect
                                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                                    let inner_types: Vec<&syn::Type> = args
                                        .args
                                        .iter()
                                        .filter_map(|arg| match arg {
                                            syn::GenericArgument::Type(t) => Some(t),
                                            _ => None,
                                        })
                                        .collect();
                                    if let Some(first) = inner_types.first() {
                                        let inner = self.extract_effect_from_type(first);
                                        if inner != EffectChannel::Plain {
                                            return EffectChannel::Recursive(Box::new(inner));
                                        }
                                    }
                                }
                                EffectChannel::Unknown
                            }
                        }
                    }
                    None => EffectChannel::Plain,
                }
            }
            syn::Type::Reference(_) => EffectChannel::Plain,
            syn::Type::Tuple(tuple) => {
                if tuple.elems.is_empty() {
                    EffectChannel::Plain // unit
                } else {
                    EffectChannel::Unknown
                }
            }
            _ => EffectChannel::Unknown,
        }
    }

    /// Build a source span from a proc_macro2 span.
    fn make_span_from_span(&self, span: proc_macro2::Span, file: &str) -> SourceSpan {
        let start = span.start();
        let end = span.end();
        SourceSpan {
            file: file.to_string(),
            start_line: start.line,
            start_column: start.column + 1,
            end_line: end.line,
            end_column: end.column + 1,
        }
    }

    /// Build a source span from a spanned item.
    fn make_span(&self, spanned: &impl Spanned, file: &str) -> SourceSpan {
        let span = spanned.span();
        self.make_span_from_span(span, file)
    }

    /// Build a stable ID from a name and span.
    fn make_id(&self, name: &str, span: &proc_macro2::Span) -> StableId {
        let file = self.path.to_string_lossy();
        let line = span.start().line;
        StableId::new(format!("{name}:{file}:{line}"))
    }

    /// Extract nodes from a function declaration.
    fn extract_function(&mut self, func: &syn::ItemFn, _attrs: &[syn::Attribute]) {
        let name = func.sig.ident.to_string();
        let id = self.make_id(&name, &func.sig.ident.span());

        let domain = self.extract_fn_params(&func.sig);
        let codomain = self.extract_return_shape(&func.sig.output);

        let mut effect = self.extract_effect(&func.sig.output);

        // Check for async fn
        if func.sig.asyncness.is_some() {
            effect = EffectChannel::Recursive(Box::new(effect));
        }

        let span = self.make_span(&func.sig.ident, &self.path.to_string_lossy());

        let node = CirNode {
            id: id.clone(),
            domain,
            codomain,
            effect,
            span,
            name: Some(name.clone()),
        };

        self.graph.add_node(node);
        self.nodes.insert(name.clone(), id);

        // Track current function context for edge source resolution
        let prev = self.current_function.replace(name);

        // Visit the body for call extraction
        visit::visit_block(self, &func.block);

        self.current_function = prev;
    }

    /// Extract domain shape from function parameters.
    fn extract_fn_params(&self, sig: &syn::Signature) -> Shape {
        let params: Vec<Shape> = sig
            .inputs
            .iter()
            .map(|param| match param {
                syn::FnArg::Typed(pat_type) => self.extract_shape(&pat_type.ty),
                syn::FnArg::Receiver(_) => Shape::Scalar, // &self
            })
            .collect();

        if params.is_empty() {
            Shape::Scalar
        } else if params.len() == 1 {
            params.into_iter().next().unwrap()
        } else {
            Shape::Record(params)
        }
    }

    /// Extract codomain shape from return type.
    fn extract_return_shape(&self, output: &syn::ReturnType) -> Shape {
        match output {
            syn::ReturnType::Default => Shape::Scalar, // unit
            syn::ReturnType::Type(_, ty) => self.extract_shape(ty),
        }
    }

    /// Known Rust built-in functions/constructors that should not produce edges.
    const BUILTINS: &[&str] = &[
        "Ok",
        "Err",
        "Some",
        "None",
        "format",
        "print",
        "println",
        "eprint",
        "eprintln",
        "write",
        "writeln",
        "assert",
        "assert_eq",
        "assert_ne",
        "debug_assert",
        "debug_assert_eq",
        "debug_assert_ne",
        "unreachable",
        "unimplemented",
        "todo",
        "panic",
        "vec",
        "String::from",
        "ToOwned::to_owned",
    ];

    /// Check if a callee name is a known Rust built-in.
    fn is_builtin(name: &str) -> bool {
        Self::BUILTINS.contains(&name)
    }

    /// Extract an edge from a function call expression.
    fn extract_call(&mut self, func: &syn::Expr, span: proc_macro2::Span) {
        // Get the callee name
        let callee_name = match func {
            syn::Expr::Path(expr_path) => {
                let segments: Vec<String> = expr_path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                segments.join("::")
            }
            syn::Expr::MethodCall(method_call) => method_call.method.to_string(),
            _ => return, // Can't determine callee — skip
        };

        // Skip known Rust built-in functions
        if Self::is_builtin(&callee_name) {
            return;
        }

        // Deduplicate: skip if we already have an edge for this call at this location
        let edge_id = self.make_id(&format!("call_{callee_name}"), &span);
        if self.graph.edges.iter().any(|e| e.id == edge_id) {
            return;
        }

        // Resolve source from current function context
        let source_id = match self
            .current_function
            .as_ref()
            .and_then(|name| self.nodes.get(name))
        {
            Some(id) => id.clone(),
            None => return, // No caller context — skip
        };

        // Only create edges for known callees (functions declared in this source)
        let target_id = match self.nodes.get(&callee_name) {
            Some(id) => id.clone(),
            None => return, // Unknown callee — skip (external/stdlib call)
        };

        let source_span = self.make_span_from_span(span, &self.path.to_string_lossy());

        // Detect unwrap patterns
        let (resolution, unwrap_evidence) = self.detect_unwrap(func);

        let edge = CirEdge {
            id: edge_id,
            source: source_id,
            target: target_id,
            resolution,
            unwrap_evidence,
            provenance: Provenance::Direct,
            span: source_span,
            discard_spans: vec![],
        };

        self.graph.add_edge(edge);
    }

    /// Detect unwrap patterns in a call expression.
    fn detect_unwrap(&self, func: &syn::Expr) -> (EffectResolution, Option<UnwrapEvidence>) {
        if let syn::Expr::MethodCall(method_call) = func {
            let method_name = method_call.method.to_string();
            match method_name.as_str() {
                "unwrap" | "expect" => {
                    return (
                        EffectResolution::Unwrapped,
                        Some(UnwrapEvidence {
                            kind: UnwrapKind::Ordinary,
                            totality: Totality::Total,
                        }),
                    );
                }
                "unwrap_unchecked" => {
                    return (
                        EffectResolution::Swallowed,
                        Some(UnwrapEvidence {
                            kind: UnwrapKind::Force,
                            totality: Totality::Partial,
                        }),
                    );
                }
                _ => {}
            }
        }

        (EffectResolution::Propagated, None)
    }
}

/// Visit functions and extract CIR nodes.
impl<'ast> Visit<'ast> for Extractor {
    fn visit_item_fn(&mut self, func: &'ast syn::ItemFn) {
        self.extract_function(func, &func.attrs);
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        let span = call.func.as_ref().span();
        self.extract_call(&call.func, span);
        // Continue visiting inside the call arguments
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let span = call.span();
        self.extract_call(&syn::Expr::MethodCall(call.clone()), span);
        // Continue visiting
        visit::visit_expr_method_call(self, call);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_simple_function_no_params() {
        let source = "fn hello() -> i32 { 42 }";
        let syntax = syn::parse_file(source).unwrap();
        let graph = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(graph.nodes.len(), 1);
        let node = &graph.nodes[0];
        assert_eq!(node.name.as_deref(), Some("hello"));
        assert_eq!(node.domain, Shape::Scalar);
        assert_eq!(node.codomain, Shape::Scalar);
        assert_eq!(node.effect, EffectChannel::Plain);
    }

    #[test]
    fn extract_function_with_params() {
        let source = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let syntax = syn::parse_file(source).unwrap();
        let graph = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(graph.nodes.len(), 1);
        let node = &graph.nodes[0];
        assert_eq!(node.name.as_deref(), Some("add"));
        assert_eq!(
            node.domain,
            Shape::Record(vec![Shape::Scalar, Shape::Scalar])
        );
        assert_eq!(node.codomain, Shape::Scalar);
    }

    #[test]
    fn extract_async_function() {
        let source = "async fn fetch() -> String { \"data\".to_string() }";
        let syntax = syn::parse_file(source).unwrap();
        let graph = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(graph.nodes.len(), 1);
        let node = &graph.nodes[0];
        assert_eq!(node.name.as_deref(), Some("fetch"));
        assert_eq!(
            node.effect,
            EffectChannel::Recursive(Box::new(EffectChannel::Plain))
        );
    }

    #[test]
    fn extract_function_with_result() {
        let source = "fn parse(input: &str) -> Result<i32, Error> { Ok(42) }";
        let syntax = syn::parse_file(source).unwrap();
        let graph = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(graph.nodes.len(), 1);
        let node = &graph.nodes[0];
        assert_eq!(node.name.as_deref(), Some("parse"));
        assert_eq!(node.effect, EffectChannel::Result);
        assert_eq!(node.domain, Shape::Ref(Box::new(Shape::Scalar)));
        assert_eq!(
            node.codomain,
            Shape::Parameterized {
                base: "Result".into(),
                parameters: vec![Shape::Scalar, Shape::Scalar],
            }
        );
    }

    #[test]
    fn extract_function_call() {
        let source = r#"
fn helper() -> i32 { 42 }
fn main() -> i32 { helper() }
"#;
        let syntax = syn::parse_file(source).unwrap();
        let graph = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        let edge = &graph.edges[0];
        assert_eq!(edge.resolution, EffectResolution::Propagated);
    }

    #[test]
    fn extract_fully_qualified_call_skipped() {
        // Fully qualified calls (crate::helper) are not matched to simple names (helper)
        // in the initial implementation. This is acceptable — unqualified calls are the
        // common case, and cross-module resolution is a future enhancement.
        let source = r#"
fn helper() -> i32 { 42 }
fn main() -> i32 { crate::helper() }
"#;
        let syntax = syn::parse_file(source).unwrap();
        let graph = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(graph.nodes.len(), 2);
        // No edge because 'crate::helper' != 'helper'
        assert_eq!(graph.edges.len(), 0);
    }

    #[test]
    fn extract_shape_ref() {
        let source = "fn foo(x: &str) { }";
        let syntax = syn::parse_file(source).unwrap();
        let graph = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].domain, Shape::Ref(Box::new(Shape::Scalar)));
    }

    #[test]
    fn extract_shape_parameterized() {
        let source = "fn foo(x: Vec<i32>) { }";
        let syntax = syn::parse_file(source).unwrap();
        let graph = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(
            graph.nodes[0].domain,
            Shape::Parameterized {
                base: "Vec".into(),
                parameters: vec![Shape::Scalar],
            }
        );
    }

    #[test]
    fn extract_span_information() {
        let source = "fn foo() { }";
        let syntax = syn::parse_file(source).unwrap();
        let graph = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(graph.nodes.len(), 1);
        let span = &graph.nodes[0].span;
        assert_eq!(span.file, "test.rs");
        assert!(span.start_line > 0);
    }

    #[test]
    fn extract_empty_file() {
        let source = "";
        let syntax = syn::parse_file(source).unwrap();
        let graph = extract_graph(&syntax, Path::new("empty.rs"));
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
    }

    #[test]
    fn extract_only_comments() {
        let source = "// just a comment\n/* block */";
        let syntax = syn::parse_file(source).unwrap();
        let graph = extract_graph(&syntax, Path::new("comments.rs"));
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
    }

    #[test]
    fn extract_multiple_functions() {
        let source = r#"
fn a() {}
fn b() {}
fn c() {}
"#;
        let syntax = syn::parse_file(source).unwrap();
        let graph = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.nodes[0].name.as_deref(), Some("a"));
        assert_eq!(graph.nodes[1].name.as_deref(), Some("b"));
        assert_eq!(graph.nodes[2].name.as_deref(), Some("c"));
    }

    #[test]
    fn extract_graph_validates() {
        let source = r#"
fn foo() -> i32 { 42 }
fn bar() -> i32 { foo() }
"#;
        let syntax = syn::parse_file(source).unwrap();
        let graph = extract_graph(&syntax, Path::new("test.rs"));
        let result = graph.validate();
        assert!(result.is_ok(), "graph should be valid: {result:?}");
    }
}
