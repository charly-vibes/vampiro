//! Struct-field registry for field-access shape inference
//! (vampiro-224.14).
//!
//! `Expr::Field` used to return the *base* expression's shape: a `&Ctx`
//! parameter's field access inferred `Ref(Opaque)`, which does not
//! auto-deref against `Scalar(String)` → a genuine-mismatch false positive
//! (the espectacular check.rs class). The fix: visit `ItemStruct` items,
//! record field name → declared shape per struct, and resolve `base.field`
//! through the registry. Unknown base type, unknown field, or tuple-index
//! access degrade to opaque (224.3 rule: never guess).
//!
//! Callees take two parameters so the domain is a `Record` and
//! `domain_slot(slot)` actually indexes (bare-scalar domains return `None`
//! and are never compared).

use std::path::Path;

use vampiro_cir::Frontend;
use vampiro_rust_frontend::RustFrontend;
use vampiro_seam_analysis::CompositionAnalyzer;

fn findings(source: &str) -> Vec<String> {
    let graph = RustFrontend
        .extract(source, Path::new("src/lib.rs"))
        .expect("frontend extraction must succeed");
    CompositionAnalyzer::new()
        .analyze(&graph)
        .iter()
        .map(|f| f.rule.clone())
        .collect()
}

/// A field access on a reference-typed parameter must infer the field's
/// declared shape (`Scalar(String)`), not the base's `Ref(Opaque)`.
#[test]
fn field_access_argument_matches_declared_field() {
    let source = r#"
        struct ScenarioContext {
            spec_path: String,
            retries: u32,
        }

        fn probe(path: String, line: u32) {}

        fn caller(ctx: &ScenarioContext) {
            probe(ctx.spec_path, 1);
        }
    "#;
    assert!(
        findings(source).is_empty(),
        "field access must resolve to the field's declared shape, got {:?}",
        findings(source)
    );
}

/// Chained access `outer.inner.name` resolves through two registry hops:
/// `Outer.inner` → `Inner`, `Inner.name` → `Scalar(String)`.
#[test]
fn chained_field_access_resolves_via_registry() {
    let source = r#"
        struct Inner {
            name: String,
        }

        struct Outer {
            inner: Inner,
        }

        fn probe(name: String, line: u32) {}

        fn caller(outer: &Outer) {
            probe(outer.inner.name, 1);
        }
    "#;
    assert!(
        findings(source).is_empty(),
        "chained field access must resolve each hop, got {:?}",
        findings(source)
    );
}

/// A second field with a different declared kind must also match its own
/// shape — pins that the registry is per-field, not per-struct.
#[test]
fn second_field_resolves_to_its_own_shape() {
    let source = r#"
        struct Config {
            name: String,
            retries: u32,
        }

        fn probe_int(n: u32, line: u32) {}
        fn probe_str(s: String, line: u32) {}

        fn caller(cfg: &Config) {
            probe_int(cfg.retries, 1);
            probe_str(cfg.name, 2);
        }
    "#;
    assert!(
        findings(source).is_empty(),
        "each field must resolve to its own declared shape, got {:?}",
        findings(source)
    );
}

/// Base type not in the registry (e.g. defined in another file) → opaque,
/// which is excluded from composition checking (224.3 rule). The base's
/// `Ref(Opaque)` shape must NOT leak through as the field's shape.
#[test]
fn field_access_on_unknown_base_is_opaque() {
    let source = r#"
        // ExternalType is not declared in this file.
        fn probe(count: u32, line: u32) {}

        fn caller(ext: &ExternalType) {
            probe(ext.count, 1);
        }
    "#;
    assert!(
        findings(source).is_empty(),
        "unresolvable base must degrade to opaque, got {:?}",
        findings(source)
    );
}

/// Tuple-index access (`pair.0`) has no named field → opaque.
#[test]
fn tuple_index_access_stays_opaque() {
    let source = r#"
        struct Pair {
            left: u32,
            right: u32,
        }

        fn probe(n: u32, line: u32) {}

        fn caller(p: &Pair) {
            probe(p.0, 1);
        }
    "#;
    assert!(
        findings(source).is_empty(),
        "tuple-index access must degrade to opaque, got {:?}",
        findings(source)
    );
}

/// A local binding with a type annotation feeds the registry lookup too:
/// `let ctx: &Ctx = ...; ctx.field`.
#[test]
fn annotated_local_binding_resolves_field() {
    let source = r#"
        struct Ctx {
            spec_path: String,
        }

        fn probe(path: String, line: u32) {}

        fn make_ctx() -> &'static Ctx { unreachable!() }

        fn caller() -> u32 {
            let ctx: &Ctx = make_ctx();
            probe(ctx.spec_path, 1);
            0
        }
    "#;
    assert!(
        findings(source).is_empty(),
        "annotated let binding must resolve via the registry, got {:?}",
        findings(source)
    );
}
