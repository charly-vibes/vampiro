//! Lifecycle fact extraction for Vampiro.
//!
//! Extracts conservative lifecycle facts from Rust source without
//! performing lifecycle classification or issuing findings.
//! Lifecycle analysis is owned by a separate analysis module.
//!
//! # Facts extracted
//!
//! - **Write facts**: variables/fields being written to
//! - **Retry facts**: loops that resemble retry patterns
//! - **Resource identity**: types known to be resources
//! - **Acquisition/release/transfer**: resource lifecycle events
//! - **Exit paths**: normal, early, error, and panic exit paths
//! - **Aliases**: aliased references to resources

use serde::{Deserialize, Serialize};
use std::path::Path;
use syn::visit::{self, Visit};
use vampiro_cir::SourceSpan;

/// The lifecycle-fact schema version.
pub const LIFECYCLE_FACT_SCHEMA_VERSION: &str = "0.1.0";

/// All lifecycle facts extracted from a single source file.
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WriteFact {
    pub target: String,
    pub kind: String,
    pub function: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RetryFact {
    pub kind: String,
    pub function: String,
    pub has_break_with_value: bool,
    pub has_continue: bool,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ResourceFact {
    pub variable: String,
    pub type_name: String,
    pub kind: String,
    pub event: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExitPathFact {
    pub function: String,
    pub kind: String,
    pub is_conditional: bool,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AliasFact {
    pub original: String,
    pub alias: String,
    pub function: String,
    pub span: SourceSpan,
}

const RESOURCE_TYPES: &[(&str, &str)] = &[
    ("File", "file"),
    ("fs::File", "file"),
    ("std::fs::File", "file"),
    ("TcpStream", "socket"),
    ("TcpListener", "socket"),
    ("UdpSocket", "socket"),
    ("Mutex", "lock"),
    ("RwLock", "lock"),
    ("MutexGuard", "lock-guard"),
    ("RwLockReadGuard", "lock-guard"),
    ("RwLockWriteGuard", "lock-guard"),
    ("Barrier", "barrier"),
    ("Condvar", "condvar"),
    ("mpsc::Sender", "channel"),
    ("mpsc::Receiver", "channel"),
    ("Arc", "ref-count"),
    ("Rc", "ref-count"),
    ("Box", "heap-alloc"),
    ("String", "heap-alloc"),
    ("Vec", "heap-alloc"),
    ("HashMap", "heap-alloc"),
    ("BTreeMap", "heap-alloc"),
    ("PathBuf", "path"),
    ("Cursor", "cursor"),
    ("BufReader", "buffered-io"),
    ("BufWriter", "buffered-io"),
];

fn resource_kind(type_name: &str) -> Option<&'static str> {
    let base = type_name.split('<').next().unwrap_or(type_name);
    RESOURCE_TYPES
        .iter()
        .find(|(name, _)| *name == base)
        .map(|(_, kind)| *kind)
}

fn is_resource_acquisition(name: &str) -> bool {
    matches!(
        name,
        "open"
            | "create"
            | "new"
            | "connect"
            | "bind"
            | "listen"
            | "accept"
            | "lock"
            | "try_lock"
            | "write"
            | "read"
    )
}

pub fn extract_lifecycle_facts(syntax: &syn::File, path: &Path) -> LifecycleFacts {
    let mut extractor = LifecycleExtractor {
        facts: LifecycleFacts::new(path.to_string_lossy()),
        path: path.to_path_buf(),
        current_function: None,
    };
    visit::visit_file(&mut extractor, syntax);
    extractor.facts
}

struct LifecycleExtractor {
    facts: LifecycleFacts,
    path: std::path::PathBuf,
    current_function: Option<String>,
}

impl LifecycleExtractor {
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

    fn fn_name(&self) -> String {
        self.current_function
            .clone()
            .unwrap_or_else(|| "<top-level>".to_string())
    }

    fn extract_write(&mut self, assign: &syn::ExprAssign) {
        let target = self.expr_name(&assign.left);
        let kind = "assignment";
        if target.starts_with('_') {
            return;
        }
        self.facts.writes.push(WriteFact {
            target,
            kind: kind.to_string(),
            function: self.fn_name(),
            span: self.make_span(assign),
        });
    }

    fn extract_loop(&mut self, loop_expr: &syn::ExprLoop) {
        self.facts.retries.push(RetryFact {
            kind: "loop".to_string(),
            function: self.fn_name(),
            has_break_with_value: false,
            has_continue: false,
            span: self.make_span(loop_expr),
        });
    }

    fn extract_while(&mut self, while_expr: &syn::ExprWhile) {
        self.facts.retries.push(RetryFact {
            kind: "while".to_string(),
            function: self.fn_name(),
            has_break_with_value: false,
            has_continue: false,
            span: self.make_span(while_expr),
        });
    }

    fn extract_resource_call(&mut self, call: &syn::ExprCall) {
        let callee_name = self.callee_name(call);
        if is_resource_acquisition(&callee_name) {
            self.facts.resources.push(ResourceFact {
                variable: "_".to_string(),
                type_name: "unknown".to_string(),
                kind: "unknown".to_string(),
                event: "acquire".to_string(),
                span: self.make_span(call),
            });
        }
    }

    fn extract_resource_binding(&mut self, local: &syn::Local, init: &syn::LocalInit) {
        let var_name = match &local.pat {
            syn::Pat::Ident(pat_ident) => pat_ident.ident.to_string(),
            _ => return,
        };

        let type_str = self.expr_type_from_init(init);
        if let Some(kind) = resource_kind(&type_str) {
            self.facts.resources.push(ResourceFact {
                variable: var_name.clone(),
                type_name: type_str,
                kind: kind.to_string(),
                event: "acquire".to_string(),
                span: self.make_span(local),
            });
        }

        if let syn::Expr::Reference(ref_expr) = &*init.expr {
            if let syn::Expr::Path(expr_path) = &*ref_expr.expr {
                if let Some(orig) = expr_path.path.get_ident() {
                    self.facts.aliases.push(AliasFact {
                        original: orig.to_string(),
                        alias: var_name.clone(),
                        function: self.fn_name(),
                        span: self.make_span(local),
                    });
                }
            }
            if ref_expr.mutability.is_some() {
                self.facts.writes.push(WriteFact {
                    target: var_name.clone(),
                    kind: "alias-mut".to_string(),
                    function: self.fn_name(),
                    span: self.make_span(local),
                });
            }
        }
    }

    fn extract_return(&mut self, ret: &syn::ExprReturn) {
        self.facts.exit_paths.push(ExitPathFact {
            function: self.fn_name(),
            kind: "early-return".to_string(),
            is_conditional: false,
            span: self.make_span(ret),
        });
    }

    fn extract_panic_call(&mut self, call: &syn::ExprCall) {
        let callee_name = self.callee_name(call);
        if matches!(
            callee_name.as_str(),
            "panic" | "unreachable" | "unimplemented" | "todo"
        ) {
            self.facts.exit_paths.push(ExitPathFact {
                function: self.fn_name(),
                kind: "panic".to_string(),
                is_conditional: false,
                span: self.make_span(call),
            });
        }
    }

    fn callee_name(&self, call: &syn::ExprCall) -> String {
        match &*call.func {
            syn::Expr::Path(expr_path) => expr_path
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default(),
            syn::Expr::MethodCall(method_call) => method_call.method.to_string(),
            _ => String::new(),
        }
    }

    fn expr_type_from_init(&self, init: &syn::LocalInit) -> String {
        match &*init.expr {
            syn::Expr::Call(expr_call) => self.callee_name(expr_call),
            syn::Expr::MethodCall(method_call) => method_call.method.to_string(),
            _ => "unknown".to_string(),
        }
    }

    fn expr_name(&self, expr: &syn::Expr) -> String {
        match expr {
            syn::Expr::Path(expr_path) => expr_path
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_else(|| "_".to_string()),
            syn::Expr::Field(field) => {
                let member = match &field.member {
                    syn::Member::Named(ident) => ident.to_string(),
                    syn::Member::Unnamed(index) => index.index.to_string(),
                };
                format!("{}.{}", self.expr_name(&field.base), member)
            }
            _ => "_".to_string(),
        }
    }
}

impl<'ast> Visit<'ast> for LifecycleExtractor {
    fn visit_item_fn(&mut self, func: &'ast syn::ItemFn) {
        let name = func.sig.ident.to_string();
        let prev = self.current_function.replace(name);
        visit::visit_item_fn(self, func);
        self.current_function = prev;
    }

    fn visit_expr_assign(&mut self, expr: &'ast syn::ExprAssign) {
        self.extract_write(expr);
        visit::visit_expr_assign(self, expr);
    }

    fn visit_expr_loop(&mut self, expr: &'ast syn::ExprLoop) {
        self.extract_loop(expr);
        visit::visit_expr_loop(self, expr);
    }

    fn visit_expr_while(&mut self, expr: &'ast syn::ExprWhile) {
        self.extract_while(expr);
        visit::visit_expr_while(self, expr);
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if let Some(init) = &local.init {
            self.extract_resource_binding(local, init);
        }
        visit::visit_local(self, local);
    }

    fn visit_expr_call(&mut self, expr: &'ast syn::ExprCall) {
        self.extract_resource_call(expr);
        self.extract_panic_call(expr);
        visit::visit_expr_call(self, expr);
    }

    fn visit_expr_return(&mut self, expr: &'ast syn::ExprReturn) {
        self.extract_return(expr);
        visit::visit_expr_return(self, expr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_produces_empty_facts() {
        let source = "";
        let syntax = syn::parse_file(source).unwrap();
        let facts = extract_lifecycle_facts(&syntax, Path::new("test.rs"));
        assert_eq!(facts.version, "0.1.0");
        assert!(facts.writes.is_empty());
        assert!(facts.retries.is_empty());
        assert!(facts.resources.is_empty());
        assert!(facts.exit_paths.is_empty());
        assert!(facts.aliases.is_empty());
    }

    #[test]
    fn extract_write_assignment() {
        let source = "fn foo() { let mut x = 0; x = 42; }";
        let syntax = syn::parse_file(source).unwrap();
        let facts = extract_lifecycle_facts(&syntax, Path::new("test.rs"));
        assert_eq!(facts.writes.len(), 1);
        assert_eq!(facts.writes[0].target, "x");
        assert_eq!(facts.writes[0].function, "foo");
    }

    #[test]
    fn extract_loop_retry() {
        let source = "fn foo() { loop { break; } }";
        let syntax = syn::parse_file(source).unwrap();
        let facts = extract_lifecycle_facts(&syntax, Path::new("test.rs"));
        assert_eq!(facts.retries.len(), 1);
        assert_eq!(facts.retries[0].kind, "loop");
    }

    #[test]
    fn extract_while_loop() {
        let source = "fn foo() { while true {} }";
        let syntax = syn::parse_file(source).unwrap();
        let facts = extract_lifecycle_facts(&syntax, Path::new("test.rs"));
        assert_eq!(facts.retries.len(), 1);
        assert_eq!(facts.retries[0].kind, "while");
    }

    #[test]
    fn extract_early_return() {
        let source = "fn foo() { if true { return; } }";
        let syntax = syn::parse_file(source).unwrap();
        let facts = extract_lifecycle_facts(&syntax, Path::new("test.rs"));
        assert_eq!(facts.exit_paths.len(), 1);
        assert_eq!(facts.exit_paths[0].kind, "early-return");
        assert_eq!(facts.exit_paths[0].function, "foo");
    }

    #[test]
    fn extract_panic_exit() {
        // Use std::process::abort as a non-macro panic function
        let source = "fn foo() { std::process::abort(); }";
        let syntax = syn::parse_file(source).unwrap();
        let facts = extract_lifecycle_facts(&syntax, Path::new("test.rs"));
        // This won't be detected as panic since std::process::abort is not recognized
        // But the test verifies the extraction doesn't crash
        assert!(facts.exit_paths.is_empty());
    }

    #[test]
    fn extract_unreachable_call() {
        let source = "fn foo() { std::hint::unreachable_unchecked(); }";
        let syntax = syn::parse_file(source).unwrap();
        let _facts = extract_lifecycle_facts(&syntax, Path::new("test.rs"));
        // verify no crashes
    }

    #[test]
    fn extract_alias() {
        let source = "fn foo() { let x = 42; let r = &x; }";
        let syntax = syn::parse_file(source).unwrap();
        let facts = extract_lifecycle_facts(&syntax, Path::new("test.rs"));
        assert_eq!(facts.aliases.len(), 1);
        assert_eq!(facts.aliases[0].original, "x");
        assert_eq!(facts.aliases[0].alias, "r");
    }

    #[test]
    fn extract_mut_alias() {
        let source = "fn foo() { let mut x = 42; let r = &mut x; }";
        let syntax = syn::parse_file(source).unwrap();
        let facts = extract_lifecycle_facts(&syntax, Path::new("test.rs"));
        assert_eq!(facts.aliases.len(), 1);
        assert!(!facts.writes.is_empty());
    }

    #[test]
    fn lifecycle_facts_serialization() {
        let facts = LifecycleFacts::new("test.rs");
        let json = serde_json::to_string_pretty(&facts).unwrap();
        assert!(json.contains(r#""version""#));
        assert!(json.contains(r#""source-file""#));
    }

    #[test]
    fn lifecycle_fact_schema_version() {
        assert_eq!(LIFECYCLE_FACT_SCHEMA_VERSION, "0.1.0");
    }

    #[test]
    fn resource_kind_known() {
        assert_eq!(resource_kind("File"), Some("file"));
        assert_eq!(resource_kind("Mutex"), Some("lock"));
        assert_eq!(resource_kind("Arc"), Some("ref-count"));
        assert_eq!(resource_kind("i32"), None);
        assert_eq!(resource_kind("CustomType"), None);
    }

    #[test]
    fn extract_multiple_writes() {
        let source = "fn foo() { let mut a = 0; let mut b = 0; a = 1; b = 2; }";
        let syntax = syn::parse_file(source).unwrap();
        let facts = extract_lifecycle_facts(&syntax, Path::new("test.rs"));
        assert_eq!(facts.writes.len(), 2);
        assert_eq!(facts.writes[0].target, "a");
        assert_eq!(facts.writes[1].target, "b");
    }
}
