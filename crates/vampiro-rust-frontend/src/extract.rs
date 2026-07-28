//! CIR extraction from syn's AST.
//!
//! Walks a `syn::File` and produces a `CirGraph` with nodes for
//! function/closure declarations and edges for call sites.
//! Also extracts visibility metadata and facade (re-export) entries.

use std::collections::HashMap;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use vampiro_cir::{
    CirEdge, CirGraph, CirNode, EffectChannel, EffectResolution, Provenance, Shape, SourceSpan,
    StableId, Totality, UnwrapEvidence, UnwrapKind,
};

use crate::visibility::{FacadeDecl, FacadeEntry, Visibility};

/// Result of extracting a CIR graph and metadata from a syn file.
pub struct ExtractionResult {
    /// The extracted CIR graph.
    pub graph: CirGraph,
    /// Facade declarations (re-exports) at each module level.
    #[allow(dead_code)]
    pub facades: Vec<FacadeDecl>,
    /// Visibility map: node ID -> visibility level.
    #[allow(dead_code)]
    pub visibility: HashMap<StableId, Visibility>,
}

/// Extract a `CirGraph` and metadata from a parsed syn file.
pub fn extract_graph(syntax: &syn::File, path: &Path) -> ExtractionResult {
    let mut extractor = Extractor {
        graph: CirGraph::new(path.to_string_lossy()),
        nodes: HashMap::new(),
        current_function: None,
        path: path.to_path_buf(),
        module_stack: Vec::new(),
        facades: Vec::new(),
        visibility: HashMap::new(),
        doc_hidden_stack: Vec::new(),
    };
    visit::visit_file(&mut extractor, syntax);
    ExtractionResult {
        graph: extractor.graph,
        facades: extractor.facades,
        visibility: extractor.visibility,
    }
}

/// The extraction visitor.
struct Extractor {
    graph: CirGraph,
    /// Map from function name to node stable ID.
    nodes: HashMap<String, StableId>,
    /// The current function being visited (for edge source resolution).
    current_function: Option<String>,
    path: std::path::PathBuf,
    /// Module path stack (e.g., [] for root, ["foo"] for `mod foo`).
    module_stack: Vec<String>,
    /// Facade entries found during extraction.
    facades: Vec<FacadeDecl>,
    /// Visibility per node ID.
    visibility: HashMap<StableId, Visibility>,
    /// Stack of #[doc(hidden)] state (for nested items).
    #[allow(dead_code)]
    doc_hidden_stack: Vec<bool>,
}

impl Extractor {
    /// Check if an attribute list includes `#[doc(hidden)]`.
    fn has_doc_hidden(&self, attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|attr| {
            attr.path().is_ident("doc")
                && attr
                    .meta
                    .require_list()
                    .is_ok_and(|meta| meta.tokens.to_string().contains("hidden"))
        })
    }

    /// Get the current module path as a string.
    fn current_module_path(&self) -> String {
        self.module_stack.join("::")
    }

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
                    Shape::Scalar
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
                    EffectChannel::Plain
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

        if func.sig.asyncness.is_some() {
            effect = EffectChannel::Recursive(Box::new(effect));
        }

        let span = self.make_span(&func.sig.ident, &self.path.to_string_lossy());

        let vis = Visibility::from(&func.vis);
        let node = CirNode {
            id: id.clone(),
            domain,
            codomain,
            effect,
            span,
            name: Some(name.clone()),
        };

        self.graph.add_node(node);
        self.nodes.insert(name.clone(), id.clone());
        self.visibility.insert(id, vis);

        let prev = self.current_function.replace(name);

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
                syn::FnArg::Receiver(_) => Shape::Scalar,
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
            syn::ReturnType::Default => Shape::Scalar,
            syn::ReturnType::Type(_, ty) => self.extract_shape(ty),
        }
    }

    /// Extract a `pub use` item as a facade entry.
    fn extract_use_item(&mut self, item: &syn::ItemUse) {
        let vis = Visibility::from(&item.vis);
        let doc_hidden = self.has_doc_hidden(&item.attrs);
        let module_path = self.current_module_path();
        let span = item.span();

        let tree = item.tree.clone();

        let facade_idx = self
            .facades
            .iter()
            .position(|f| f.module_path == module_path);

        let entries = Self::extract_use_tree_entries(
            &tree,
            &vis,
            doc_hidden,
            &module_path,
            &span,
            &self.path,
        );

        if let Some(idx) = facade_idx {
            for entry in entries {
                self.facades[idx].add_entry(entry);
            }
        } else {
            let mut facade = FacadeDecl::new(&module_path);
            for entry in entries {
                facade.add_entry(entry);
            }
            self.facades.push(facade);
        }
    }

    /// Recursively extract re-export entries from a use tree (free function, no borrow issues).
    fn extract_use_tree_entries(
        tree: &syn::UseTree,
        vis: &Visibility,
        doc_hidden: bool,
        module_path: &str,
        span: &proc_macro2::Span,
        file_path: &Path,
    ) -> Vec<FacadeEntry> {
        let mut entries = Vec::new();
        Self::collect_use_entries(
            tree,
            vis,
            doc_hidden,
            module_path,
            span,
            file_path,
            &mut entries,
        );
        entries
    }

    fn collect_use_entries(
        tree: &syn::UseTree,
        vis: &Visibility,
        doc_hidden: bool,
        module_path: &str,
        span: &proc_macro2::Span,
        file_path: &Path,
        entries: &mut Vec<FacadeEntry>,
    ) {
        match tree {
            syn::UseTree::Path(use_path) => {
                Self::collect_use_entries(
                    &use_path.tree,
                    vis,
                    doc_hidden,
                    module_path,
                    span,
                    file_path,
                    entries,
                );
            }
            syn::UseTree::Name(use_name) => {
                let name = use_name.ident.to_string();
                let original_path = if module_path.is_empty() {
                    name.clone()
                } else {
                    format!("{module_path}::{name}")
                };
                let start = span.start();
                let end = span.end();
                entries.push(FacadeEntry {
                    name,
                    original_path,
                    is_wildcard: false,
                    visibility: vis.clone(),
                    span: SourceSpan {
                        file: file_path.to_string_lossy().to_string(),
                        start_line: start.line,
                        start_column: start.column + 1,
                        end_line: end.line,
                        end_column: end.column + 1,
                    },
                    doc_hidden,
                });
            }
            syn::UseTree::Rename(use_rename) => {
                let name = use_rename.rename.to_string();
                let original_path = use_rename.ident.to_string();
                let start = span.start();
                let end = span.end();
                entries.push(FacadeEntry {
                    name,
                    original_path,
                    is_wildcard: false,
                    visibility: vis.clone(),
                    span: SourceSpan {
                        file: file_path.to_string_lossy().to_string(),
                        start_line: start.line,
                        start_column: start.column + 1,
                        end_line: end.line,
                        end_column: end.column + 1,
                    },
                    doc_hidden,
                });
            }
            syn::UseTree::Glob(_) => {
                let start = span.start();
                let end = span.end();
                entries.push(FacadeEntry {
                    name: "*".into(),
                    original_path: module_path.to_string(),
                    is_wildcard: true,
                    visibility: vis.clone(),
                    span: SourceSpan {
                        file: file_path.to_string_lossy().to_string(),
                        start_line: start.line,
                        start_column: start.column + 1,
                        end_line: end.line,
                        end_column: end.column + 1,
                    },
                    doc_hidden,
                });
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    Self::collect_use_entries(
                        item,
                        vis,
                        doc_hidden,
                        module_path,
                        span,
                        file_path,
                        entries,
                    );
                }
            }
        }
    }

    /// Extract an edge from a function call expression.
    fn extract_call(&mut self, func: &syn::Expr, span: proc_macro2::Span) {
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
            _ => return,
        };

        if Self::is_builtin(&callee_name) {
            return;
        }

        let edge_id = self.make_id(&format!("call_{callee_name}"), &span);
        if self.graph.edges.iter().any(|e| e.id == edge_id) {
            return;
        }

        let source_id = match self
            .current_function
            .as_ref()
            .and_then(|name| self.nodes.get(name))
        {
            Some(id) => id.clone(),
            None => return,
        };

        let target_id = match self.nodes.get(&callee_name) {
            Some(id) => id.clone(),
            None => return,
        };

        let source_span = self.make_span_from_span(span, &self.path.to_string_lossy());
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

/// Visit functions and extract CIR nodes, visibility, and facades.
impl<'ast> Visit<'ast> for Extractor {
    fn visit_item_fn(&mut self, func: &'ast syn::ItemFn) {
        self.extract_function(func, &func.attrs);
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.extract_use_item(item);
        visit::visit_item_use(self, item);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let name = item.ident.to_string();
        self.module_stack.push(name);
        if let Some((_, items)) = &item.content {
            for child in items {
                visit::visit_item(self, child);
            }
        }
        self.module_stack.pop();
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        let span = call.func.as_ref().span();
        self.extract_call(&call.func, span);
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let span = call.span();
        self.extract_call(&syn::Expr::MethodCall(call.clone()), span);
        visit::visit_expr_method_call(self, call);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visibility::Visibility;

    #[test]
    fn extract_simple_function_no_params() {
        let source = "fn hello() -> i32 { 42 }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(result.graph.nodes.len(), 1);
        let node = &result.graph.nodes[0];
        assert_eq!(node.name.as_deref(), Some("hello"));
        assert_eq!(node.domain, Shape::Scalar);
        assert_eq!(node.codomain, Shape::Scalar);
        assert_eq!(node.effect, EffectChannel::Plain);
    }

    #[test]
    fn extract_function_with_params() {
        let source = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(result.graph.nodes.len(), 1);
        let node = &result.graph.nodes[0];
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
        let result = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(result.graph.nodes.len(), 1);
        let node = &result.graph.nodes[0];
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
        let result = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(result.graph.nodes.len(), 1);
        let node = &result.graph.nodes[0];
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
        let source = "fn helper() -> i32 { 42 }\nfn main() -> i32 { helper() }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(result.graph.nodes.len(), 2);
        assert_eq!(result.graph.edges.len(), 1);
        let edge = &result.graph.edges[0];
        assert_eq!(edge.resolution, EffectResolution::Propagated);
    }

    #[test]
    fn extract_fully_qualified_call_skipped() {
        let source = "fn helper() -> i32 { 42 }\nfn main() -> i32 { crate::helper() }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(result.graph.nodes.len(), 2);
        assert_eq!(result.graph.edges.len(), 0);
    }

    #[test]
    fn extract_shape_ref() {
        let source = "fn foo(x: &str) { }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(
            result.graph.nodes[0].domain,
            Shape::Ref(Box::new(Shape::Scalar))
        );
    }

    #[test]
    fn extract_shape_parameterized() {
        let source = "fn foo(x: Vec<i32>) { }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(
            result.graph.nodes[0].domain,
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
        let result = extract_graph(&syntax, Path::new("test.rs"));
        let span = &result.graph.nodes[0].span;
        assert_eq!(span.file, "test.rs");
        assert!(span.start_line > 0);
    }

    #[test]
    fn extract_empty_file() {
        let source = "";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("empty.rs"));
        assert_eq!(result.graph.nodes.len(), 0);
        assert_eq!(result.graph.edges.len(), 0);
    }

    #[test]
    fn extract_only_comments() {
        let source = "// just a comment\n/* block */";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("comments.rs"));
        assert_eq!(result.graph.nodes.len(), 0);
        assert_eq!(result.graph.edges.len(), 0);
    }

    #[test]
    fn extract_multiple_functions() {
        let source = "fn a() {}\nfn b() {}\nfn c() {}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(result.graph.nodes.len(), 3);
        assert_eq!(result.graph.nodes[0].name.as_deref(), Some("a"));
        assert_eq!(result.graph.nodes[1].name.as_deref(), Some("b"));
        assert_eq!(result.graph.nodes[2].name.as_deref(), Some("c"));
    }

    #[test]
    fn extract_graph_validates() {
        let source = "fn foo() -> i32 { 42 }\nfn bar() -> i32 { foo() }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"));
        let res = result.graph.validate();
        assert!(res.is_ok(), "graph should be valid: {res:?}");
    }

    // --- Visibility tests ---

    #[test]
    fn extract_public_function_visibility() {
        let source = "pub fn foo() {}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"));
        let id = &result.graph.nodes[0].id;
        let vis = result.visibility.get(id).unwrap();
        assert_eq!(*vis, Visibility::Public);
    }

    #[test]
    fn extract_private_function_visibility() {
        let source = "fn foo() {}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"));
        let id = &result.graph.nodes[0].id;
        let vis = result.visibility.get(id).unwrap();
        assert_eq!(*vis, Visibility::Private);
    }

    #[test]
    fn extract_crate_visibility() {
        let source = "pub(crate) fn foo() {}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"));
        let id = &result.graph.nodes[0].id;
        let vis = result.visibility.get(id).unwrap();
        assert_eq!(*vis, Visibility::Crate);
    }

    #[test]
    fn extract_super_visibility() {
        let source = "pub(super) fn foo() {}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"));
        let id = &result.graph.nodes[0].id;
        let vis = result.visibility.get(id).unwrap();
        assert_eq!(*vis, Visibility::Super);
    }

    #[test]
    fn extract_restricted_visibility() {
        let source = "pub(in foo::bar) fn foo() {}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"));
        let id = &result.graph.nodes[0].id;
        let vis = result.visibility.get(id).unwrap();
        assert_eq!(*vis, Visibility::Restricted("foo::bar".into()));
    }

    // --- Facade (pub use) tests ---

    #[test]
    fn extract_simple_pub_use() {
        let source = "pub use helper;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"));
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.module_path, "");
        assert_eq!(facade.entries.len(), 1);
        assert_eq!(facade.entries[0].name, "helper");
        assert_eq!(facade.entries[0].visibility, Visibility::Public);
        assert!(!facade.entries[0].is_wildcard);
    }

    #[test]
    fn extract_private_use() {
        let source = "use helper;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"));
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.entries.len(), 1);
        assert_eq!(facade.entries[0].visibility, Visibility::Private);
    }

    #[test]
    fn extract_wildcard_use() {
        let source = "pub use module::*;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"));
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.entries.len(), 1);
        assert!(facade.entries[0].is_wildcard);
        assert_eq!(facade.entries[0].name, "*");
    }

    #[test]
    fn extract_renamed_use() {
        let source = "pub use old_name as new_name;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"));
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.entries.len(), 1);
        assert_eq!(facade.entries[0].name, "new_name");
        assert_eq!(facade.entries[0].original_path, "old_name");
    }

    #[test]
    fn extract_use_group() {
        let source = "pub use {foo, bar};";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"));
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.entries.len(), 2);
        assert_eq!(facade.entries[0].name, "foo");
        assert_eq!(facade.entries[1].name, "bar");
    }

    #[test]
    fn extract_use_path() {
        let source = "pub use foo::bar::baz;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"));
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.entries.len(), 1);
        assert_eq!(facade.entries[0].name, "baz");
    }

    #[test]
    fn extract_module_use() {
        let source = "mod inner {\n    pub use helper;\n}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"));
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.module_path, "inner");
        assert_eq!(facade.entries.len(), 1);
        assert_eq!(facade.entries[0].name, "helper");
    }

    #[test]
    fn extract_multiple_uses() {
        let source = "pub use a;\npub use b;\npub use c;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"));
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.entries.len(), 3);
    }

    #[test]
    fn extract_doc_hidden_use() {
        let source = "#[doc(hidden)] pub use internal;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"));
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.entries.len(), 1);
        assert!(facade.entries[0].doc_hidden);
    }

    #[test]
    fn extract_crate_use() {
        let source = "pub(crate) use internal;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"));
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.entries[0].visibility, Visibility::Crate);
    }

    #[test]
    fn extract_mixed_public_and_private() {
        let source = "pub fn public_fn() {}\nfn private_fn() {}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"));
        assert_eq!(result.graph.nodes.len(), 2);

        let pub_id = &result.graph.nodes[0].id;
        let priv_id = &result.graph.nodes[1].id;

        assert_eq!(*result.visibility.get(pub_id).unwrap(), Visibility::Public);
        assert_eq!(
            *result.visibility.get(priv_id).unwrap(),
            Visibility::Private
        );
    }
}
