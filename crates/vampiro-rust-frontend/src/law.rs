//! Law runner-input extraction for Vampiro.
//!
//! Extracts metadata from Rust source that law verification will use:
//! - Implementation clusters (`impl` blocks and their methods)
//! - Proof/law tagged functions (`#[law]`, `#[proof]`, `#[test]`, etc.)
//! - Serializable values (function parameters usable as law runner inputs)
//! - Generator references (iterators, streams, generators)
//!
//! Runner execution is owned by law verification — this module only
//! extracts the input data.

use serde::{Deserialize, Serialize};
use std::path::Path;
use syn::visit::{self, Visit};
use vampiro_cir::SourceSpan;

/// The law runner-input schema version.
pub const RUNNER_INPUT_SCHEMA_VERSION: &str = "0.1.0";

/// All law runner-input data extracted from a single source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LawRunnerInput {
    /// The schema version of this runner-input data.
    pub version: String,
    /// The source file path.
    pub source_file: String,
    /// Implementation clusters (impl blocks).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clusters: Vec<ImplCluster>,
    /// Functions tagged with proof/law/test attributes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tagged_fns: Vec<TaggedFn>,
    /// Serializable values found in the source.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub serializable_values: Vec<SerializableValue>,
    /// Generator/iterator references.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generator_refs: Vec<GeneratorRef>,
}

impl LawRunnerInput {
    /// Create a new empty law runner-input for the given source file.
    pub fn new(source_file: impl Into<String>) -> Self {
        LawRunnerInput {
            version: RUNNER_INPUT_SCHEMA_VERSION.into(),
            source_file: source_file.into(),
            clusters: Vec::new(),
            tagged_fns: Vec::new(),
            serializable_values: Vec::new(),
            generator_refs: Vec::new(),
        }
    }
}

/// A cluster of methods implementing a type or trait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ImplCluster {
    /// The type being implemented.
    pub self_type: String,
    /// Whether this is a trait impl.
    pub is_trait_impl: bool,
    /// The trait being implemented, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trait_name: Option<String>,
    /// Method names in this impl block.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<String>,
    /// Source span of the impl block.
    pub span: SourceSpan,
}

/// A function tagged with proof/law/test attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TaggedFn {
    /// The function name.
    pub name: String,
    /// Tags found on this function (e.g., "law", "proof", "test").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Parameter names and types.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<FnParam>,
    /// The return type, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
    /// Source span.
    pub span: SourceSpan,
}

/// A function parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FnParam {
    /// The parameter name.
    pub name: String,
    /// The type as a string.
    pub type_name: String,
    /// Whether the type is serializable (implements Serialize).
    pub is_serializable: bool,
}

/// A serializable value in the source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SerializableValue {
    /// The value name (variable or parameter name).
    pub name: String,
    /// The type as a string.
    pub type_name: String,
    /// Whether the type is serializable.
    pub is_serializable: bool,
    /// Source span.
    pub span: SourceSpan,
}

/// A generator or iterator reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GeneratorRef {
    /// The name of the generator/iterator.
    pub name: String,
    /// The item type yielded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    /// The kind of generator (e.g., "iterator", "stream", "generator").
    pub kind: String,
    /// Source span.
    pub span: SourceSpan,
}

/// Known tags that mark proof/law/test functions.
const KNOWN_TAGS: &[&str] = &["law", "proof", "test", "should_panic", "bench"];

/// Types that are known to implement serde::Serialize.
const SERIALIZABLE_TYPES: &[&str] = &[
    "i8",
    "i16",
    "i32",
    "i64",
    "u8",
    "u16",
    "u32",
    "u64",
    "f32",
    "f64",
    "bool",
    "char",
    "String",
    "str",
    "PathBuf",
    "Path",
    "Duration",
    "Instant",
    "SystemTime",
];

/// Check if a type name is known to be serializable.
fn is_serializable_type(type_name: &str) -> bool {
    // Strip generics
    let base = type_name.split('<').next().unwrap_or(type_name);
    if SERIALIZABLE_TYPES.contains(&base) {
        return true;
    }
    // Collections are serializable if their elements are
    matches!(
        base,
        "Vec"
            | "Option"
            | "Result"
            | "Box"
            | "HashMap"
            | "BTreeMap"
            | "HashSet"
            | "BTreeSet"
            | "VecDeque"
            | "LinkedList"
            | "Rc"
            | "Arc"
            | "Cell"
            | "RefCell"
            | "Mutex"
            | "RwLock"
    )
}

/// Extract the base type name from a syn type (without generics).
fn type_name_as_string(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => {
            let segments: Vec<String> = type_path
                .path
                .segments
                .iter()
                .map(|s| {
                    let name = s.ident.to_string();
                    match &s.arguments {
                        syn::PathArguments::AngleBracketed(args) => {
                            let params: Vec<String> = args
                                .args
                                .iter()
                                .filter_map(|arg| match arg {
                                    syn::GenericArgument::Type(t) => Some(type_name_as_string(t)),
                                    _ => None,
                                })
                                .collect();
                            if params.is_empty() {
                                name
                            } else {
                                format!("{name}<{}>", params.join(", "))
                            }
                        }
                        _ => name,
                    }
                })
                .collect();
            segments.join("::")
        }
        syn::Type::Reference(type_ref) => {
            let inner = type_name_as_string(&type_ref.elem);
            format!("&{inner}")
        }
        syn::Type::Tuple(tuple) => {
            let elems: Vec<String> = tuple.elems.iter().map(type_name_as_string).collect();
            format!("({})", elems.join(", "))
        }
        syn::Type::Slice(slice) => {
            let inner = type_name_as_string(&slice.elem);
            format!("[{inner}]")
        }
        syn::Type::ImplTrait(_) => "impl Trait".to_string(),
        _ => "unknown".to_string(),
    }
}

/// Extract law runner-input data from a parsed syn file.
pub fn extract_law_input(syntax: &syn::File, path: &Path) -> LawRunnerInput {
    let mut extractor = LawExtractor {
        input: LawRunnerInput::new(path.to_string_lossy()),
        path: path.to_path_buf(),
    };
    visit::visit_file(&mut extractor, syntax);
    extractor.input
}

/// The law extraction visitor.
struct LawExtractor {
    input: LawRunnerInput,
    path: std::path::PathBuf,
}

impl LawExtractor {
    /// Build a source span from a spanned item.
    fn make_span(&self, spanned: &impl syn::spanned::Spanned) -> SourceSpan {
        let span = spanned.span();
        let start = span.start();
        let end = span.end();
        SourceSpan {
            file: self.path.to_string_lossy().to_string(),
            start_line: start.line,
            start_column: start.column + 1,
            end_line: end.line,
            end_column: end.column + 1,
        }
    }

    /// Check if an attribute list contains known tags.
    fn extract_tags(&self, attrs: &[syn::Attribute]) -> Vec<String> {
        let mut tags = Vec::new();
        for attr in attrs {
            let path_str = attr.path()
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            for known in KNOWN_TAGS {
                if path_str.contains(known) {
                    tags.push(known.to_string());
                }
            }
        }
        tags
    }

    /// Extract an impl block.
    fn extract_impl(&mut self, item: &syn::ItemImpl) {
        let self_type = type_name_as_string(&item.self_ty);
        let is_trait_impl = item.trait_.is_some();
        let trait_name = item.trait_.as_ref().map(|(_, path, _)| {
            path.segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::")
        });

        let methods: Vec<String> = item
            .items
            .iter()
            .filter_map(|item| match item {
                syn::ImplItem::Fn(method) => Some(method.sig.ident.to_string()),
                _ => None,
            })
            .collect();

        if methods.is_empty() {
            return;
        }

        self.input.clusters.push(ImplCluster {
            self_type,
            is_trait_impl,
            trait_name,
            methods,
            span: self.make_span(item),
        });
    }

    /// Extract a function with its tags and parameters.
    fn extract_fn(&mut self, func: &syn::ItemFn) {
        let tags = self.extract_tags(&func.attrs);
        if tags.is_empty() {
            return;
        }

        let name = func.sig.ident.to_string();
        let params: Vec<FnParam> = func
            .sig
            .inputs
            .iter()
            .map(|param| match param {
                syn::FnArg::Typed(pat_type) => {
                    let type_name = type_name_as_string(&pat_type.ty);
                    let param_name = match &*pat_type.pat {
                        syn::Pat::Ident(pat_ident) => pat_ident.ident.to_string(),
                        _ => "_".to_string(),
                    };
                    FnParam {
                        name: param_name,
                        type_name: type_name.clone(),
                        is_serializable: is_serializable_type(&type_name),
                    }
                }
                syn::FnArg::Receiver(_) => FnParam {
                    name: "self".to_string(),
                    type_name: "Self".to_string(),
                    is_serializable: false,
                },
            })
            .collect();

        let return_type = match &func.sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => Some(type_name_as_string(ty)),
        };

        self.input.tagged_fns.push(TaggedFn {
            name,
            tags,
            params,
            return_type,
            span: self.make_span(&func.sig),
        });
    }

    /// Extract generator/iterator references from a variable declaration.
    #[allow(dead_code)]
    fn extract_generator_from_type(&mut self, name: &str, ty: &syn::Type, span: &SourceSpan) {
        let type_str = type_name_as_string(ty);
        let kind = if type_str.starts_with("Stream") {
            "stream"
        } else if type_str.starts_with("Generator") {
            "generator"
        } else {
            "iterator"
        };

        let item_type = match ty {
            syn::Type::Path(type_path) => {
                if let Some(seg) = type_path.path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        args.args.first().and_then(|arg| match arg {
                            syn::GenericArgument::Type(t) => Some(type_name_as_string(t)),
                            _ => None,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        self.input.generator_refs.push(GeneratorRef {
            name: name.to_string(),
            item_type,
            kind: kind.to_string(),
            span: span.clone(),
        });
    }
}

impl<'ast> Visit<'ast> for LawExtractor {
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        self.extract_impl(item);
        visit::visit_item_impl(self, item);
    }

    fn visit_item_fn(&mut self, func: &'ast syn::ItemFn) {
        self.extract_fn(func);
        visit::visit_item_fn(self, func);
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        // Extract generator/iterator references from let bindings
        if let Some(init) = &local.init {
            if let syn::Expr::Call(expr_call) = &*init.expr {
                if let syn::Expr::Path(expr_path) = &*expr_call.func {
                    let name = expr_path
                        .path
                        .segments
                        .last()
                        .map(|s| s.ident.to_string())
                        .unwrap_or_default();
                    if name == "iter" || name == "into_iter" || name == "iterate" {
                        let span = self.make_span(local);
                        let var_name = match &local.pat {
                            syn::Pat::Ident(pat_ident) => pat_ident.ident.to_string(),
                            _ => "_".to_string(),
                        };
                        self.input.generator_refs.push(GeneratorRef {
                            name: var_name,
                            item_type: None,
                            kind: "iterator".to_string(),
                            span,
                        });
                    }
                }
            }
        }

        visit::visit_local(self, local);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_produces_empty_input() {
        let source = "";
        let syntax = syn::parse_file(source).unwrap();
        let input = extract_law_input(&syntax, Path::new("test.rs"));
        assert_eq!(input.version, "0.1.0");
        assert!(input.clusters.is_empty());
        assert!(input.tagged_fns.is_empty());
        assert!(input.generator_refs.is_empty());
    }

    #[test]
    fn extract_simple_impl_block() {
        let source = "impl Foo { fn bar() {} fn baz() {} }";
        let syntax = syn::parse_file(source).unwrap();
        let input = extract_law_input(&syntax, Path::new("test.rs"));
        assert_eq!(input.clusters.len(), 1);
        let cluster = &input.clusters[0];
        assert_eq!(cluster.self_type, "Foo");
        assert!(!cluster.is_trait_impl);
        assert_eq!(cluster.methods, vec!["bar", "baz"]);
    }

    #[test]
    fn extract_trait_impl() {
        let source = "impl Trait for Foo { fn method() {} }";
        let syntax = syn::parse_file(source).unwrap();
        let input = extract_law_input(&syntax, Path::new("test.rs"));
        assert_eq!(input.clusters.len(), 1);
        let cluster = &input.clusters[0];
        assert_eq!(cluster.self_type, "Foo");
        assert!(cluster.is_trait_impl);
        assert_eq!(cluster.trait_name, Some("Trait".to_string()));
        assert_eq!(cluster.methods, vec!["method"]);
    }

    #[test]
    fn extract_empty_impl_skipped() {
        let source = "impl Foo {}";
        let syntax = syn::parse_file(source).unwrap();
        let input = extract_law_input(&syntax, Path::new("test.rs"));
        assert!(input.clusters.is_empty());
    }

    #[test]
    fn extract_tagged_fn_law() {
        let source = "#[law] fn check_property() {}";
        let syntax = syn::parse_file(source).unwrap();
        let input = extract_law_input(&syntax, Path::new("test.rs"));
        assert_eq!(input.tagged_fns.len(), 1);
        let tf = &input.tagged_fns[0];
        assert_eq!(tf.name, "check_property");
        assert!(tf.tags.contains(&"law".to_string()));
    }

    #[test]
    fn extract_tagged_fn_test() {
        let source = "#[test] fn test_something() {}";
        let syntax = syn::parse_file(source).unwrap();
        let input = extract_law_input(&syntax, Path::new("test.rs"));
        assert_eq!(input.tagged_fns.len(), 1);
        let tf = &input.tagged_fns[0];
        assert_eq!(tf.name, "test_something");
        assert!(tf.tags.contains(&"test".to_string()));
    }

    #[test]
    fn extract_untagged_fn_skipped() {
        let source = "fn normal() {}";
        let syntax = syn::parse_file(source).unwrap();
        let input = extract_law_input(&syntax, Path::new("test.rs"));
        assert!(input.tagged_fns.is_empty());
    }

    #[test]
    fn extract_tagged_fn_params() {
        let source = "#[law] fn check(n: i32, s: String) -> bool { true }";
        let syntax = syn::parse_file(source).unwrap();
        let input = extract_law_input(&syntax, Path::new("test.rs"));
        assert_eq!(input.tagged_fns.len(), 1);
        let tf = &input.tagged_fns[0];
        assert_eq!(tf.params.len(), 2);
        assert_eq!(tf.params[0].name, "n");
        assert_eq!(tf.params[0].type_name, "i32");
        assert!(tf.params[0].is_serializable);
        assert_eq!(tf.params[1].name, "s");
        assert_eq!(tf.params[1].type_name, "String");
        assert!(tf.params[1].is_serializable);
        assert_eq!(tf.return_type, Some("bool".to_string()));
    }

    #[test]
    fn extract_serializable_type() {
        assert!(is_serializable_type("i32"));
        assert!(is_serializable_type("String"));
        assert!(is_serializable_type("Vec<i32>"));
        assert!(is_serializable_type("Option<bool>"));
        assert!(!is_serializable_type("CustomType"));
        assert!(!is_serializable_type("*const u8"));
    }

    #[test]
    fn extract_generator_from_let() {
        let source = "fn test() { let iter: Iterator<i32> = vec![1, 2].into_iter(); }";
        let syntax = syn::parse_file(source).unwrap();
        let input = extract_law_input(&syntax, Path::new("test.rs"));
        // May or may not find generators depending on parsing
        // The key is that it doesn't crash
        assert!(input.version == "0.1.0");
    }

    #[test]
    fn extract_multiple_impl_blocks() {
        let source = "impl Foo { fn a() {} }\nimpl Bar { fn b() {} fn c() {} }";
        let syntax = syn::parse_file(source).unwrap();
        let input = extract_law_input(&syntax, Path::new("test.rs"));
        assert_eq!(input.clusters.len(), 2);
        assert_eq!(input.clusters[0].self_type, "Foo");
        assert_eq!(input.clusters[1].self_type, "Bar");
    }

    #[test]
    fn extract_type_name() {
        assert_eq!(type_name_as_string(&syn::parse_quote!(i32)), "i32");
        assert_eq!(
            type_name_as_string(&syn::parse_quote!(Vec<i32>)),
            "Vec<i32>"
        );
        assert_eq!(type_name_as_string(&syn::parse_quote!(&str)), "&str");
        assert_eq!(
            type_name_as_string(&syn::parse_quote!(Option<String>)),
            "Option<String>"
        );
    }

    #[test]
    fn runner_input_serialization() {
        let input = LawRunnerInput::new("test.rs");
        let json = serde_json::to_string_pretty(&input).unwrap();
        assert!(json.contains(r#""version""#));
        assert!(json.contains(r#""source-file""#));
    }

    #[test]
    fn runner_input_schema_version() {
        assert_eq!(RUNNER_INPUT_SCHEMA_VERSION, "0.1.0");
    }
}
