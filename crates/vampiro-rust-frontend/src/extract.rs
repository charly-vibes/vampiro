//! CIR extraction from syn's AST.
//!
//! Walks a `syn::File` and produces a `CirGraph` with nodes for
//! function/closure declarations and edges for call sites.
//! Also extracts visibility metadata and facade (re-export) entries.

use std::collections::HashMap;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use vampiro_cir::{ScalarKind, 
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
///
/// `source` is the original source text, used to compute content-sensitive
/// stable identities (see [`StableId`](vampiro_cir::StableId)).
pub fn extract_graph(syntax: &syn::File, path: &Path, source: &str) -> ExtractionResult {
    // Pre-index source lines once for O(1) line lookups in make_id
    // instead of scanning all lines on every source_slice call.
    let lines_cache: Vec<&str> = source.lines().collect();
    let mut extractor = Extractor {
        graph: CirGraph::new(path.to_string_lossy()),
        nodes: HashMap::new(),
        current_function: None,
        path: path.to_path_buf(),
        module_stack: Vec::new(),
        facades: Vec::new(),
        visibility: HashMap::new(),
        doc_hidden_stack: Vec::new(),
        source,
        lines_cache,
        param_shapes: HashMap::new(),
        is_test_context: false,
        pending_discard: false,
    };
    visit::visit_file(&mut extractor, syntax);
    ExtractionResult {
        graph: extractor.graph,
        facades: extractor.facades,
        visibility: extractor.visibility,
    }
}

/// The extraction visitor.
struct Extractor<'src> {
    graph: CirGraph,
    /// Map from fully-qualified function path to node stable ID.
    ///
    /// Keys are module-qualified (e.g. `"a::b::fn_name"`) so that shadowed
    /// or same-named functions in different modules do not collide.
    nodes: HashMap<String, StableId>,
    /// The fully-qualified path of the function currently being visited
    /// (for edge source resolution).
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
    /// The original source text, used for content-sensitive stable IDs.
    #[allow(dead_code)]
    source: &'src str,
    /// Pre-indexed source lines for O(1) line lookups in make_id.
    lines_cache: Vec<&'src str>,
    /// Parameter name → Shape map for the current function being visited.
    /// Populated when entering a function, used to resolve variable
    /// references in argument expressions for the slot-boundary check.
    param_shapes: HashMap<String, Shape>,
    /// Whether we are currently inside a `#[cfg(test)]` module or a `#[test]` function.
    is_test_context: bool,
    /// Whether the next visited call expression is in a discard context
    /// (e.g., `let _ = expr;` or `expr;` as a statement).
    pending_discard: bool,
}

impl<'src> Extractor<'src> {
    /// Check if an attribute list includes `#[doc(hidden)]`.
    ///
    /// Matches only the attribute form `#[doc(hidden)]`; a doc *comment*
    /// such as `/// hidden features` is not a false positive.
    fn has_doc_hidden(&self, attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|attr| {
            if !attr.path().is_ident("doc") {
                return false;
            }
            match &attr.meta {
                syn::Meta::List(list) => list
                    .parse_args::<syn::Path>()
                    .ok()
                    .map(|p| p.is_ident("hidden"))
                    .unwrap_or(false),
                _ => false,
            }
        })
    }

    /// Check if an attribute list includes `#[cfg(test)]`.
    fn has_cfg_test(&self, attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|attr| {
            if !attr.path().is_ident("cfg") {
                return false;
            }
            match &attr.meta {
                syn::Meta::List(list) => list
                    .parse_args::<syn::Path>()
                    .ok()
                    .map(|p| p.is_ident("test"))
                    .unwrap_or(false),
                _ => false,
            }
        })
    }

    /// Check if an attribute list includes `#[test]`.
    fn has_test_attr(&self, attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|attr| attr.path().is_ident("test"))
    }

    /// The current module path as a `::`-joined string.
    fn current_module_path(&self) -> String {
        self.module_stack.join("::")
    }

    /// The fully-qualified key for a declaration named `name` in the
    /// current module (e.g. `"a::b::fn_name"`, or just `"fn_name"` at root).
    fn current_fq(&self, name: &str) -> String {
        let mp = self.current_module_path();
        if mp.is_empty() {
            name.to_string()
        } else {
            format!("{mp}::{name}")
        }
    }

    /// Resolve a callee name to a node stable ID.
    ///
    /// Tries the name as-is (handles fully-qualified calls like `a::b::f`),
    /// then walks the current module stack from innermost to outermost
    /// (so a bare call `f()` from within `a::b` resolves to `a::b::f` then
    /// `a::f` then the crate-root `f`), then the bare name at crate root.
    fn resolve_node(&self, callee: &str) -> Option<&StableId> {
        if let Some(id) = self.nodes.get(callee) {
            return Some(id);
        }
        let mut acc = self.current_module_path();
        while !acc.is_empty() {
            let candidate = format!("{acc}::{callee}");
            if let Some(id) = self.nodes.get(&candidate) {
                return Some(id);
            }
            acc = match acc.rsplit_once("::") {
                Some((parent, _)) => parent.to_string(),
                None => break,
            };
        }
        self.nodes.get(callee)
    }

    /// Slice the source lines `[start_line, end_line]` (1-indexed, inclusive).
    ///
    /// Uses a pre-built line index for O(k) access where k = end_line - start_line,
    /// avoiding O(source_line_count) scans on every call.
    fn source_slice(&self, start_line: usize, end_line: usize) -> String {
        // 1-indexed to 0-indexed, clamped to line count.
        let start = start_line.saturating_sub(1);
        let end = end_line.min(self.lines_cache.len());
        if start >= end || start >= self.lines_cache.len() {
            return String::new();
        }
        self.lines_cache[start..end].join("\n")
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
                            "bool" => Shape::Scalar(ScalarKind::Bool),
                            "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16"
                            | "u32" | "u64" | "u128" | "usize" | "isize" => Shape::Scalar(ScalarKind::Int),
                            "f32" | "f64" => Shape::Scalar(ScalarKind::Float),
                            "char" => Shape::Scalar(ScalarKind::Char),
                            "String" | "str" => Shape::Scalar(ScalarKind::String),
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
                                        Shape::Scalar(ScalarKind::Unit)
                                    } else {
                                        Shape::Parameterized {
                                            base: ident,
                                            parameters: params,
                                        }
                                    }
                                } else {
                                    Shape::Scalar(ScalarKind::Unit)
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
                    elems.into_iter().next().unwrap_or(Shape::Scalar(ScalarKind::Unit))
                } else if elems.is_empty() {
                    Shape::Scalar(ScalarKind::Unit)
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
            syn::Type::Never(_) => Shape::Bottom,
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

    /// Build a stable ID for a declaration or call site.
    ///
    /// Implements `SHA256(name:file:line:column:content)` truncated to
    /// 128 bits, hex-encoded. The content slice makes the ID content-
    /// sensitive; the column makes two same-line call sites distinct.
    fn make_id(&self, name: &str, span: &proc_macro2::Span) -> StableId {
        use sha2::{Digest, Sha256};
        let file = self.path.to_string_lossy();
        let start = span.start();
        let end = span.end();
        let content = self.source_slice(start.line, end.line);
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        hasher.update(b":");
        hasher.update(file.as_bytes());
        hasher.update(b":");
        hasher.update(start.line.to_string().as_bytes());
        hasher.update(b":");
        hasher.update(start.column.to_string().as_bytes());
        hasher.update(b":");
        hasher.update(content.as_bytes());
        let hash = hasher.finalize();
        // Truncate to 128 bits (16 bytes) and hex-encode.
        let hex: String = hash[..16].iter().map(|b| format!("{b:02x}")).collect();
        StableId::new(hex)
    }

    /// Extract nodes from a function declaration.
    fn extract_function(&mut self, func: &syn::ItemFn, attrs: &[syn::Attribute]) {
        let name = func.sig.ident.to_string();
        let is_test = self.is_test_context || self.has_test_attr(attrs);
        let id = self.make_id(&name, &func.sig.ident.span());
        let fq = self.current_fq(&name);

        let domain = self.extract_fn_params(&func.sig);
        let codomain = self.extract_return_shape(&func.sig.output);

        // Build parameter name → Shape map for argument expression resolution
        self.param_shapes.clear();
        for param in &func.sig.inputs {
            match param {
                syn::FnArg::Typed(pat_type) => {
                    if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                        let name = pat_ident.ident.to_string();
                        let shape = self.extract_shape(&pat_type.ty);
                        self.param_shapes.insert(name, shape);
                    }
                }
                syn::FnArg::Receiver(rec) => {
                    let shape = if rec.reference.is_some() {
                        // &self → Ref(Scalar)
                        Shape::Ref(Box::new(Shape::Scalar(ScalarKind::Unit)))
                    } else {
                        // self (by value) → Scalar
                        Shape::Scalar(ScalarKind::Unit)
                    };
                    self.param_shapes.insert("self".into(), shape);
                }
            }
        }

        let mut effect = self.extract_effect(&func.sig.output);

        if func.sig.asyncness.is_some() {
            effect = EffectChannel::Recursive(Box::new(effect));
        }

        // Use the full function span (including body) so that test-code
        // filtering can detect findings inside test function bodies.
        let span = self.make_span(func, &self.path.to_string_lossy());

        let vis = Visibility::from(&func.vis);
        let node = CirNode {
            id: id.clone(),
            domain,
            codomain,
            effect,
            span,
            name: Some(name.clone()),
            trust_provenance: Default::default(),
            is_test,
        };

        self.graph.add_node(node);
        // Key by fully-qualified path so shadowed same-named functions
        // in different modules do not collide.
        self.nodes.insert(fq.clone(), id.clone());
        self.visibility.insert(id, vis);

        let prev = self.current_function.replace(fq);

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
                syn::FnArg::Receiver(_) => Shape::Scalar(ScalarKind::Unit),
            })
            .collect();

        if params.is_empty() {
            Shape::Scalar(ScalarKind::Unit)
        } else if params.len() == 1 {
            params.into_iter().next().unwrap()
        } else {
            Shape::Record(params)
        }
    }

    /// Extract codomain shape from return type.
    fn extract_return_shape(&self, output: &syn::ReturnType) -> Shape {
        match output {
            syn::ReturnType::Default => Shape::Scalar(ScalarKind::Unit),
            syn::ReturnType::Type(_, ty) => self.extract_shape(ty),
        }
    }

    /// Infer the shape of a call argument expression, if possible.
    ///
    /// Returns `None` when the shape cannot be determined statically.
    /// Frontends use this to populate `CirEdge.arg_shape` for the
    /// slot-boundary check.
    fn extract_expr_shape(&self, expr: &syn::Expr) -> Option<Shape> {
        match expr {
            // Argument is a function call: use the callee's codomain.
            syn::Expr::Call(call) => {
                if let syn::Expr::Path(expr_path) = &*call.func {
                    let callee_name = Self::path_name(expr_path);
                    if let Some(node_id) = self.resolve_node(&callee_name) {
                        if let Some(node) = self.graph.node_by_id(node_id) {
                            return Some(node.codomain.clone());
                        }
                    }
                }
                None
            }
            // Literal arguments — distinguish by literal kind.
            syn::Expr::Lit(lit) => {
                match &lit.lit {
                    syn::Lit::Str(_) => Some(Shape::Scalar(ScalarKind::String)),
                    syn::Lit::ByteStr(_) => Some(Shape::Scalar(ScalarKind::String)),
                    syn::Lit::Int(_) => Some(Shape::Scalar(ScalarKind::Int)),
                    syn::Lit::Float(_) => Some(Shape::Scalar(ScalarKind::Float)),
                    syn::Lit::Bool(_) => Some(Shape::Scalar(ScalarKind::Bool)),
                    syn::Lit::Char(_) => Some(Shape::Scalar(ScalarKind::Char)),
                    syn::Lit::Byte(_) => Some(Shape::Scalar(ScalarKind::Int)),
                    syn::Lit::Verbatim(_) => Some(Shape::Scalar(ScalarKind::Unit)),
                    _ => Some(Shape::Scalar(ScalarKind::Unit)),
                }
            }
            // Tuple expression: unit () is Scalar, else opaque.
            syn::Expr::Tuple(tup) => {
                if tup.elems.is_empty() {
                    Some(Shape::Scalar(ScalarKind::Unit))
                } else {
                    None
                }
            }
            // Block expression { ... } — opaque.
            syn::Expr::Block(_) => None,
            // Struct literal Foo { ... } — opaque.
            syn::Expr::Struct(_) => None,
            // Field access foo.bar — try the base expression.
            syn::Expr::Field(field) => self.extract_expr_shape(&field.base),
            // Path expression (variable/constant reference).
            syn::Expr::Path(expr_path) => {
                let name = Self::path_name(expr_path);
                // Look up in the current function's parameter shapes
                self.param_shapes.get(&name).cloned()
            }
            // Method call — try the receiver.
            syn::Expr::MethodCall(mc) => self.extract_expr_shape(&mc.receiver),
            // Everything else: opaque.
            _ => None,
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
            "",
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
    ///
    /// `prefix` accumulates the source path segments from `UseTree::Path`
    /// nodes so that `pub use foo::bar::baz;` records `original_path ==
    /// "foo::bar::baz"` rather than just `"baz"`.
    fn extract_use_tree_entries(
        tree: &syn::UseTree,
        vis: &Visibility,
        doc_hidden: bool,
        module_path: &str,
        prefix: &str,
        span: &proc_macro2::Span,
        file_path: &Path,
    ) -> Vec<FacadeEntry> {
        let mut entries = Vec::new();
        Self::collect_use_entries(
            tree,
            vis,
            doc_hidden,
            module_path,
            prefix,
            span,
            file_path,
            &mut entries,
        );
        entries
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_use_entries(
        tree: &syn::UseTree,
        vis: &Visibility,
        doc_hidden: bool,
        module_path: &str,
        prefix: &str,
        span: &proc_macro2::Span,
        file_path: &Path,
        entries: &mut Vec<FacadeEntry>,
    ) {
        let join = |prefix: &str, segment: &str| -> String {
            if prefix.is_empty() {
                segment.to_string()
            } else {
                format!("{prefix}::{segment}")
            }
        };
        match tree {
            syn::UseTree::Path(use_path) => {
                let ident = use_path.ident.to_string();
                let new_prefix = join(prefix, &ident);
                Self::collect_use_entries(
                    &use_path.tree,
                    vis,
                    doc_hidden,
                    module_path,
                    &new_prefix,
                    span,
                    file_path,
                    entries,
                );
            }
            syn::UseTree::Name(use_name) => {
                let name = use_name.ident.to_string();
                let original_path = join(prefix, &name);
                let (start, end) = (span.start(), span.end());
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
                let original_path = join(prefix, &use_rename.ident.to_string());
                let (start, end) = (span.start(), span.end());
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
                // The glob source is the accumulated prefix (e.g.
                // `pub use foo::bar::*` → original_path "foo::bar").
                let original_path = if prefix.is_empty() {
                    module_path.to_string()
                } else {
                    prefix.to_string()
                };
                let (start, end) = (span.start(), span.end());
                entries.push(FacadeEntry {
                    name: "*".into(),
                    original_path,
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
                        prefix,
                        span,
                        file_path,
                        entries,
                    );
                }
            }
        }
    }

    /// Record an edge for a call site, if the callee resolves to a known node.
    ///
    /// `resolution`/`unwrap_evidence` are supplied by the caller: `Propagated`/
    /// `None` for ordinary calls, `Unwrapped`/evidence for `?` operands and
    /// `.unwrap()`/`.expect()` receiver edges. Dedup is by stable edge ID, so
    /// the first recorder wins — callers that need to tag an edge with unwrap
    /// evidence must call this before the plain call visitor runs.
    fn add_call_edge(
        &mut self,
        callee_name: &str,
        span: proc_macro2::Span,
        resolution: EffectResolution,
        unwrap_evidence: Option<UnwrapEvidence>,
        slot: Option<u32>,
        arg_shape: Option<Shape>,
    ) {
        if Self::is_builtin(callee_name) {
            return;
        }

        let suffix = match slot {
            Some(s) => format!("call_{callee_name}_slot_{s}"),
            None => format!("call_{callee_name}"),
        };
        let edge_id = self.make_id(&suffix, &span);

        let source_id = match self
            .current_function
            .as_ref()
            .and_then(|fq| self.resolve_node(fq))
        {
            Some(id) => id.clone(),
            None => return,
        };

        let target_id = match self.resolve_node(callee_name) {
            Some(id) => id.clone(),
            None => return,
        };

        let source_span = self.make_span_from_span(span, &self.path.to_string_lossy());
        let edge = CirEdge {
            id: edge_id,
            source: source_id,
            target: target_id,
            resolution,
            unwrap_evidence,
            provenance: Provenance::Direct,
            span: source_span,
            discard_spans: vec![],
            trust_provenance: Default::default(),
            slot,
            arg_shape,
        };

        self.graph.add_edge(edge);
    }

    /// The `::`-joined name of a path expression.
    fn path_name(expr_path: &syn::ExprPath) -> String {
        expr_path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    }

    /// Pre-record an unwrap resolution on the receiver of a method call.
    ///
    /// For `opt().unwrap()`, the effect being resolved is `opt()`'s `Option`,
    /// so the unwrap evidence attaches to the `opt` call edge (not to a
    /// nonexistent `unwrap` node). Pre-recording runs before the receiver's
    /// `visit_expr_call`, so the unwrap-tagged version wins via dedup.
    fn pre_record_receiver_unwrap(
        &mut self,
        receiver: &syn::Expr,
        resolution: EffectResolution,
        evidence: Option<UnwrapEvidence>,
    ) {
        let (name, span) = match receiver {
            syn::Expr::Call(c) => match &*c.func {
                syn::Expr::Path(p) => (Self::path_name(p), p.span()),
                _ => return,
            },
            _ => return,
        };
        self.add_call_edge(&name, span, resolution, evidence, None, None);
    }

    /// Known Rust built-in functions/constructors that should not produce edges.
    const BUILTINS: &'static [&'static str] = &[
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
        "String::new",
        "ToOwned::to_owned",
        "Vec::new",
        "Box::new",
        "Arc::new",
        "Rc::new",
        "Default::default",
        "Into::into",
    ];

    /// Check if a callee name is a known Rust built-in.
    ///
    /// Matches both the fully-qualified name (e.g. `String::from`) and the
    /// last path segment, so `core::assert_eq` and `assert_eq` are both
    /// treated as builtins.
    fn is_builtin(name: &str) -> bool {
        if Self::BUILTINS.contains(&name) {
            return true;
        }
        match name.rsplit("::").next() {
            Some(last) => Self::BUILTINS.contains(&last),
            None => false,
        }
    }

    /// Determine the effect resolution and unwrap evidence for a method call.
    ///
    /// Per the totality matrix in `vampiro_cir::effect`:
    /// - `.unwrap()`/`.expect()` panic on the absent case → force unwrap,
    ///   partial totality (an unhandled branch).
    /// - `.unwrap_unchecked()` is an unchecked force → force unwrap, partial.
    ///
    /// The `?` operator (handled in `visit_expr_try`) is the ordinary/total
    /// case and overrides any method-call resolution via `try_unwrap`.
    fn detect_method_unwrap(method_name: &str) -> (EffectResolution, Option<UnwrapEvidence>) {
        match method_name {
            "unwrap" | "expect" => (
                EffectResolution::Unwrapped,
                Some(UnwrapEvidence {
                    kind: UnwrapKind::Force,
                    totality: Totality::Partial,
                }),
            ),
            "unwrap_unchecked" => (
                EffectResolution::Swallowed,
                Some(UnwrapEvidence {
                    kind: UnwrapKind::Force,
                    totality: Totality::Partial,
                }),
            ),
            _ => (EffectResolution::Propagated, None),
        }
    }
}

/// Visit functions and extract CIR nodes, visibility, and facades.
impl<'src, 'ast> Visit<'ast> for Extractor<'src> {
    #[allow(clippy::borrow_deref_ref)]
    fn visit_stmt(&mut self, stmt: &'ast syn::Stmt) {
        // Detect true discards: expression statements and wildcard locals.
        match stmt {
            syn::Stmt::Expr(expr, semi) => {
                // `expr;` — expression statement with semicolon, result is
                // discarded. A tail expression (no semicolon) is the function's
                // return value and is NOT a discard.
                if semi.is_some()
                    && matches!(&*expr, syn::Expr::Call(_) | syn::Expr::MethodCall(_))
                {
                    self.pending_discard = true;
                }
                visit::visit_stmt(self, stmt);
                self.pending_discard = false;
            }
            syn::Stmt::Local(local) => {
                // `let _ = expr;` — wildcard pattern discards the result.
                let is_wildcard = matches!(&local.pat, syn::Pat::Wild(_));
                if is_wildcard {
                    if let Some(init) = &local.init {
                        if matches!(&*init.expr, syn::Expr::Call(_) | syn::Expr::MethodCall(_)) {
                            self.pending_discard = true;
                        }
                    }
                }
                visit::visit_stmt(self, stmt);
                self.pending_discard = false;
            }
            _ => visit::visit_stmt(self, stmt),
        }
    }

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
        let was_test = self.is_test_context;
        self.is_test_context = was_test || self.has_cfg_test(&item.attrs);
        if let Some((_, items)) = &item.content {
            for child in items {
                visit::visit_item(self, child);
            }
        }
        self.is_test_context = was_test;
        self.module_stack.pop();
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        // Use the discard resolution when in a discard context (true discard
        // like `let _ = expr;` or `expr;`). Pre-recorded resolutions from
        // `pre_record_receiver_unwrap` (e.g., `.unwrap()`) take priority via
        // first-writer-wins dedup in add_edge.
        //
        // Consume the pending_discard flag so nested calls (e.g., arguments
        // or receivers) are NOT also tagged as discarded — only the
        // outermost call expression's result is discarded.
        let is_discard = self.pending_discard;
        self.pending_discard = false;
        let resolution = if is_discard {
            EffectResolution::Swallowed
        } else {
            EffectResolution::Propagated
        };

        // Only direct path calls produce edges; calls through arbitrary
        // expressions have no callee name to resolve.
        if let syn::Expr::Path(expr_path) = &*call.func {
            let callee_name = Self::path_name(expr_path);
            let span = expr_path.span();
            // Emit one edge per argument with slot information so the
            // composition analyzer can compare each argument's shape
            // against the callee's expected domain at that parameter.
            if call.args.is_empty() {
                self.add_call_edge(
                    &callee_name,
                    span,
                    resolution,
                    None,
                    None,
                    None,
                );
            } else {
                for (i, arg) in call.args.iter().enumerate() {
                    let arg_shape = self.extract_expr_shape(arg);
                    self.add_call_edge(
                        &callee_name,
                        span,
                        resolution.clone(),
                        None,
                        Some(i as u32),
                        arg_shape,
                    );
                }
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        // Method-call edges are best-effort: the callee name is the bare
        // method name, and the target is resolved by name only when a
        // same-named function exists. Receiver-type-aware resolution is
        // future work; today this is safe-by-accident because methods like
        // `iter` rarely collide with a defined free function.
        let callee_name = call.method.to_string();
        let span = call.method.span();
        let (resolution, evidence) = Self::detect_method_unwrap(&callee_name);
        // Emit one edge per argument: receiver at slot 0, additional args at
        // slot 1+. For zero-arg methods (iter, clone, etc.), emit a single
        // edge with slot=None for the receiver.
        let total_args = 1 + call.args.len(); // receiver + explicit args
        if total_args == 1 {
            self.add_call_edge(
                &callee_name,
                span,
                resolution.clone(),
                evidence.clone(),
                None,
                None,
            );
        } else {
            // Receiver at slot 0
            self.add_call_edge(&callee_name, span, resolution.clone(), None, Some(0), None);
            for (i, _arg) in call.args.iter().enumerate() {
                self.add_call_edge(
                    &callee_name,
                    span,
                    EffectResolution::Propagated,
                    None,
                    Some((i + 1) as u32),
                    None,
                );
            }
        }
        // For unwrap/expect/unwrap_unchecked, the effect being resolved is
        // the receiver's, so tag the receiver call edge before it is visited.
        if evidence.is_some() {
            self.pre_record_receiver_unwrap(&call.receiver, resolution, evidence);
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_try(&mut self, expr: &'ast syn::ExprTry) {
        // The `?` operator is the ordinary/total unwrap. If its direct
        // operand is a call, tag that call's edge as unwrapped/ordinary/total.
        // We record *before* recursing so the dedup in `add_call_edge` lets
        // the `?`-tagged version win over the plain version the call visitor
        // would otherwise record. The span used must match the call visitor's
        // span so dedup keys agree.
        let ordinary_total = || {
            Some(UnwrapEvidence {
                kind: UnwrapKind::Ordinary,
                totality: Totality::Total,
            })
        };
        match &*expr.expr {
            syn::Expr::Path(expr_path) => {
                let callee_name = Self::path_name(expr_path);
                self.add_call_edge(
                    &callee_name,
                    expr_path.span(),
                    EffectResolution::Unwrapped,
                    ordinary_total(),
                    None,
                    None,
                );
            }
            syn::Expr::Call(inner) => {
                if let syn::Expr::Path(expr_path) = &*inner.func {
                    let callee_name = Self::path_name(expr_path);
                    self.add_call_edge(
                        &callee_name,
                        expr_path.span(),
                        EffectResolution::Unwrapped,
                        ordinary_total(),
                        None,
                        None,
                    );
                }
            }
            syn::Expr::MethodCall(mc) => {
                self.add_call_edge(
                    &mc.method.to_string(),
                    mc.method.span(),
                    EffectResolution::Unwrapped,
                    ordinary_total(),
                    None,
                    None,
                );
            }
            _ => {}
        }
        visit::visit_expr_try(self, expr);
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
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        assert_eq!(result.graph.nodes.len(), 1);
        let node = &result.graph.nodes[0];
        assert_eq!(node.name.as_deref(), Some("hello"));
        assert_eq!(node.domain, Shape::Scalar(ScalarKind::Unit));
        assert_eq!(node.codomain, Shape::Scalar(ScalarKind::Int));
        assert_eq!(node.effect, EffectChannel::Plain);
    }

    #[test]
    fn extract_function_with_params() {
        let source = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        assert_eq!(result.graph.nodes.len(), 1);
        let node = &result.graph.nodes[0];
        assert_eq!(node.name.as_deref(), Some("add"));
        assert_eq!(
            node.domain,
            Shape::Record(vec![Shape::Scalar(ScalarKind::Int), Shape::Scalar(ScalarKind::Int)])
        );
        assert_eq!(node.codomain, Shape::Scalar(ScalarKind::Int));
    }

    #[test]
    fn extract_async_function() {
        let source = "async fn fetch() -> String { \"data\".to_string() }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
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
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        assert_eq!(result.graph.nodes.len(), 1);
        let node = &result.graph.nodes[0];
        assert_eq!(node.name.as_deref(), Some("parse"));
        assert_eq!(node.effect, EffectChannel::Result);
        assert_eq!(node.domain, Shape::Ref(Box::new(Shape::Scalar(ScalarKind::String))));
        assert_eq!(
            node.codomain,
            Shape::Parameterized {
                base: "Result".into(),
                parameters: vec![Shape::Scalar(ScalarKind::Int), Shape::Scalar(ScalarKind::Unit)],
            }
        );
    }

    #[test]
    fn extract_function_call() {
        let source = "fn helper() -> i32 { 42 }\nfn main() -> i32 { helper() }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        assert_eq!(result.graph.nodes.len(), 2);
        assert_eq!(result.graph.edges.len(), 1);
        let edge = &result.graph.edges[0];
        assert_eq!(edge.resolution, EffectResolution::Propagated);
    }

    #[test]
    fn extract_fully_qualified_call_skipped() {
        let source = "fn helper() -> i32 { 42 }\nfn main() -> i32 { crate::helper() }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        assert_eq!(result.graph.nodes.len(), 2);
        assert_eq!(result.graph.edges.len(), 0);
    }

    #[test]
    fn extract_multi_arg_call_produces_slot_edges() {
        let source = "fn add(a: i32, b: i32) -> i32 { a + b }\nfn main() -> i32 { add(1, 2) }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        assert_eq!(result.graph.nodes.len(), 2);
        assert_eq!(result.graph.edges.len(), 2);
        let slot0_edge = result
            .graph
            .edges
            .iter()
            .find(|e| e.slot == Some(0))
            .unwrap();
        let slot1_edge = result
            .graph
            .edges
            .iter()
            .find(|e| e.slot == Some(1))
            .unwrap();
        assert_ne!(
            slot0_edge.id, slot1_edge.id,
            "different slot edges should have different IDs"
        );
        // Both edges should target the same callee
        assert_eq!(
            result
                .graph
                .node_by_id(&slot0_edge.target)
                .unwrap()
                .name
                .as_deref(),
            Some("add")
        );
    }

    #[test]
    fn extract_no_arg_call_has_no_slot() {
        let source = "fn helper() -> i32 { 42 }\nfn main() -> i32 { helper() }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        assert_eq!(result.graph.edges.len(), 1);
        assert_eq!(result.graph.edges[0].slot, None);
    }

    #[test]
    fn extract_shape_ref() {
        let source = "fn foo(x: &str) { }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        assert_eq!(
            result.graph.nodes[0].domain,
            Shape::Ref(Box::new(Shape::Scalar(ScalarKind::String)))
        );
    }

    #[test]
    fn extract_shape_parameterized() {
        let source = "fn foo(x: Vec<i32>) { }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        assert_eq!(
            result.graph.nodes[0].domain,
            Shape::Parameterized {
                base: "Vec".into(),
                parameters: vec![Shape::Scalar(ScalarKind::Int)],
            }
        );
    }

    #[test]
    fn extract_span_information() {
        let source = "fn foo() { }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        let span = &result.graph.nodes[0].span;
        assert_eq!(span.file, "test.rs");
        assert!(span.start_line > 0);
    }

    #[test]
    fn extract_empty_file() {
        let source = "";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("empty.rs"), source);
        assert_eq!(result.graph.nodes.len(), 0);
        assert_eq!(result.graph.edges.len(), 0);
    }

    #[test]
    fn extract_only_comments() {
        let source = "// just a comment\n/* block */";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("comments.rs"), source);
        assert_eq!(result.graph.nodes.len(), 0);
        assert_eq!(result.graph.edges.len(), 0);
    }

    #[test]
    fn extract_multiple_functions() {
        let source = "fn a() {}\nfn b() {}\nfn c() {}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        assert_eq!(result.graph.nodes.len(), 3);
        assert_eq!(result.graph.nodes[0].name.as_deref(), Some("a"));
        assert_eq!(result.graph.nodes[1].name.as_deref(), Some("b"));
        assert_eq!(result.graph.nodes[2].name.as_deref(), Some("c"));
    }

    #[test]
    fn extract_graph_validates() {
        let source = "fn foo() -> i32 { 42 }\nfn bar() -> i32 { foo() }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        let res = result.graph.validate();
        assert!(res.is_ok(), "graph should be valid: {res:?}");
    }

    // --- Visibility tests ---

    #[test]
    fn extract_public_function_visibility() {
        let source = "pub fn foo() {}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        let id = &result.graph.nodes[0].id;
        let vis = result.visibility.get(id).unwrap();
        assert_eq!(*vis, Visibility::Public);
    }

    #[test]
    fn extract_private_function_visibility() {
        let source = "fn foo() {}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        let id = &result.graph.nodes[0].id;
        let vis = result.visibility.get(id).unwrap();
        assert_eq!(*vis, Visibility::Private);
    }

    #[test]
    fn extract_crate_visibility() {
        let source = "pub(crate) fn foo() {}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        let id = &result.graph.nodes[0].id;
        let vis = result.visibility.get(id).unwrap();
        assert_eq!(*vis, Visibility::Crate);
    }

    #[test]
    fn extract_super_visibility() {
        let source = "pub(super) fn foo() {}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        let id = &result.graph.nodes[0].id;
        let vis = result.visibility.get(id).unwrap();
        assert_eq!(*vis, Visibility::Super);
    }

    #[test]
    fn extract_restricted_visibility() {
        let source = "pub(in foo::bar) fn foo() {}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        let id = &result.graph.nodes[0].id;
        let vis = result.visibility.get(id).unwrap();
        assert_eq!(*vis, Visibility::Restricted("foo::bar".into()));
    }

    // --- Facade (pub use) tests ---

    #[test]
    fn extract_simple_pub_use() {
        let source = "pub use helper;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"), source);
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
        let result = extract_graph(&syntax, Path::new("lib.rs"), source);
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.entries.len(), 1);
        assert_eq!(facade.entries[0].visibility, Visibility::Private);
    }

    #[test]
    fn extract_wildcard_use() {
        let source = "pub use module::*;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"), source);
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
        let result = extract_graph(&syntax, Path::new("lib.rs"), source);
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
        let result = extract_graph(&syntax, Path::new("lib.rs"), source);
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
        let result = extract_graph(&syntax, Path::new("lib.rs"), source);
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.entries.len(), 1);
        assert_eq!(facade.entries[0].name, "baz");
    }

    #[test]
    fn extract_module_use() {
        let source = "mod inner {\n    pub use helper;\n}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"), source);
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
        let result = extract_graph(&syntax, Path::new("lib.rs"), source);
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.entries.len(), 3);
    }

    #[test]
    fn extract_doc_hidden_use() {
        let source = "#[doc(hidden)] pub use internal;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"), source);
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.entries.len(), 1);
        assert!(facade.entries[0].doc_hidden);
    }

    #[test]
    fn extract_crate_use() {
        let source = "pub(crate) use internal;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"), source);
        assert_eq!(result.facades.len(), 1);
        let facade = &result.facades[0];
        assert_eq!(facade.entries[0].visibility, Visibility::Crate);
    }

    #[test]
    fn extract_mixed_public_and_private() {
        let source = "pub fn public_fn() {}\nfn private_fn() {}";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        assert_eq!(result.graph.nodes.len(), 2);

        let pub_id = &result.graph.nodes[0].id;
        let priv_id = &result.graph.nodes[1].id;

        assert_eq!(*result.visibility.get(pub_id).unwrap(), Visibility::Public);
        assert_eq!(
            *result.visibility.get(priv_id).unwrap(),
            Visibility::Private
        );
    }

    // --- Regression tests for Rule of 5 fixes ---

    #[test]
    fn unwrap_method_call_is_force_partial() {
        // `.unwrap()` panics on the absent case → force unwrap, partial totality.
        let source = "fn opt() -> Option<i32> { None }\nfn main() { opt().unwrap(); }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        let edge = &result.graph.edges[0];
        assert_eq!(edge.resolution, EffectResolution::Unwrapped);
        let ev = edge.unwrap_evidence.as_ref().expect("unwrap evidence");
        assert_eq!(ev.kind, UnwrapKind::Force);
        assert_eq!(ev.totality, Totality::Partial);
    }

    #[test]
    fn expect_method_call_is_force_partial() {
        let source = "fn opt() -> Option<i32> { None }\nfn main() { opt().expect(\"boom\"); }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        let edge = &result.graph.edges[0];
        assert_eq!(edge.resolution, EffectResolution::Unwrapped);
        let ev = edge.unwrap_evidence.as_ref().expect("unwrap evidence");
        assert_eq!(ev.kind, UnwrapKind::Force);
        assert_eq!(ev.totality, Totality::Partial);
    }

    #[test]
    fn try_operator_is_ordinary_total() {
        // `?` is the canonical well-handled unwrap → ordinary, total.
        let source = "fn inner() -> Result<i32, String> { Ok(1) }\nfn caller() -> Result<i32, String> { inner()? }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        assert_eq!(result.graph.edges.len(), 1);
        let edge = &result.graph.edges[0];
        assert_eq!(edge.resolution, EffectResolution::Unwrapped);
        let ev = edge.unwrap_evidence.as_ref().expect("unwrap evidence");
        assert_eq!(ev.kind, UnwrapKind::Ordinary);
        assert_eq!(ev.totality, Totality::Total);
    }

    #[test]
    fn try_operator_on_unresolved_method_is_safe() {
        // `?` on a method whose callee does not resolve to a known function
        // cannot attach unwrap evidence (there is no edge to attach to).
        // This verifies the path is safe and produces no spurious unwrapped
        // edge on the unrelated receiver.
        let source =
            "fn maker() -> Thing { Thing }\nfn caller() -> Result<i32, E> { maker().do_thing()? }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        // `maker` edge exists, tagged as an ordinary propagated call (the `?`
        // applies to `.do_thing()`, which has no node).
        let unwrapped: Vec<_> = result
            .graph
            .edges
            .iter()
            .filter(|e| e.resolution == EffectResolution::Unwrapped)
            .collect();
        assert!(
            unwrapped.is_empty(),
            "no edge should be marked unwrapped here"
        );
    }

    #[test]
    fn use_path_preserves_original_path() {
        let source = "pub use foo::bar::baz;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"), source);
        assert_eq!(result.facades.len(), 1);
        let entry = &result.facades[0].entries[0];
        assert_eq!(entry.name, "baz");
        assert_eq!(entry.original_path, "foo::bar::baz");
    }

    #[test]
    fn use_path_rename_preserves_original_path() {
        let source = "pub use foo::bar::baz as qux;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"), source);
        let entry = &result.facades[0].entries[0];
        assert_eq!(entry.name, "qux");
        assert_eq!(entry.original_path, "foo::bar::baz");
    }

    #[test]
    fn use_glob_preserves_prefix_path() {
        let source = "pub use foo::bar::*;";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("lib.rs"), source);
        let entry = &result.facades[0].entries[0];
        assert!(entry.is_wildcard);
        assert_eq!(entry.original_path, "foo::bar");
    }

    #[test]
    fn same_line_calls_produce_distinct_edges() {
        let source = "fn helper() -> i32 { 42 }\nfn main() { helper(); helper(); }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        assert_eq!(result.graph.nodes.len(), 2);
        assert_eq!(
            result.graph.edges.len(),
            2,
            "two same-line calls must produce two distinct edges"
        );
    }

    #[test]
    fn same_named_functions_in_different_modules_dont_collide() {
        let source = "mod a { pub fn f() -> i32 { 1 } }\nmod b { pub fn f() -> i32 { 2 } }\nmod a { fn caller() -> i32 { f() } }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        // Two `f` declarations → two distinct nodes (distinct stable IDs).
        let f_nodes: Vec<_> = result
            .graph
            .nodes
            .iter()
            .filter(|n| n.name.as_deref() == Some("f"))
            .collect();
        assert_eq!(f_nodes.len(), 2);
        assert_ne!(f_nodes[0].id, f_nodes[1].id);
    }

    #[test]
    fn never_return_type_is_bottom_shape() {
        let source = "fn boom() -> ! { panic!() }";
        let syntax = syn::parse_file(source).unwrap();
        let result = extract_graph(&syntax, Path::new("test.rs"), source);
        assert_eq!(result.graph.nodes.len(), 1);
        assert_eq!(result.graph.nodes[0].codomain, Shape::Bottom);
    }

    #[test]
    fn stable_id_is_hex_and_content_sensitive() {
        // Same name + location, different body content → different IDs.
        let a = "fn f() -> i32 { 1 }";
        let b = "fn f() -> i32 { 2 }";
        let ga = extract_graph(&syn::parse_file(a).unwrap(), Path::new("t.rs"), a);
        let gb = extract_graph(&syn::parse_file(b).unwrap(), Path::new("t.rs"), b);
        let id_a = &ga.graph.nodes[0].id;
        let id_b = &gb.graph.nodes[0].id;
        assert_ne!(id_a, id_b, "IDs must be content-sensitive");
        // IDs are 32-char hex (128 bits).
        assert_eq!(id_a.as_str().len(), 32);
        assert!(id_a.as_str().chars().all(|c| c.is_ascii_hexdigit()));
        // Same input re-extracted → same ID (repeatable).
        let ga2 = extract_graph(&syn::parse_file(a).unwrap(), Path::new("t.rs"), a);
        assert_eq!(&ga2.graph.nodes[0].id, id_a);
    }
}
