//! Return-position gating for the return-boundary composition check
//! (vampiro-224.9 mirrored to the Python frontend, as part of 224.11).
//!
//! The return-boundary check compares callee codomain vs caller codomain —
//! meaningful only when the callee's result actually flows to the caller's
//! return. In Python that is exclusively the operand of a `return` statement
//! (Python has no tail-expression return). The frontend tags each
//! declaration→declaration call edge with `return_position`; these tests pin
//! which call contexts are (and are not) return position.

use std::collections::HashMap;
use std::path::Path;

use vampiro_cir::{CirGraph, Frontend, NodeKind};
use vampiro_python_frontend::PythonFrontend;

fn extract(source: &str) -> CirGraph {
    PythonFrontend
        .extract(source, Path::new("scripts/mod.py"))
        .expect("frontend extraction must succeed")
}

/// Callee name → the return-position flags of its call-result edges
/// (Declaration-source, slot-less edges).
fn call_result_flags(graph: &CirGraph) -> HashMap<String, Vec<bool>> {
    let mut flags: HashMap<String, Vec<bool>> = HashMap::new();
    for edge in &graph.edges {
        let Some(source) = graph.node_by_id(&edge.source) else {
            continue;
        };
        if source.kind != NodeKind::Declaration || edge.slot.is_some() {
            continue;
        }
        let Some(target) = graph.node_by_id(&edge.target) else {
            continue;
        };
        let name = target.name.clone().unwrap_or_default();
        flags.entry(name).or_default().push(edge.return_position);
    }
    flags
}

const SOURCE: &str = r#"
def stmt_callee():
    return 1

def aug_callee():
    return 2

def return_callee():
    return 3

def arg_callee(x):
    return x

def arg_inner_callee():
    return 4


def stmt_call():
    stmt_callee()


def aug_bound():
    total = 0
    total += aug_callee()
    return total


def explicit_return():
    return return_callee()


def arg_position():
    return arg_callee(arg_inner_callee())
"#;

fn flags() -> HashMap<String, Vec<bool>> {
    call_result_flags(&extract(SOURCE))
}

#[test]
fn statement_position_call_is_not_return_position() {
    assert_eq!(flags()["stmt_callee"], vec![false]);
}

#[test]
fn augmented_assignment_call_is_not_return_position() {
    assert_eq!(flags()["aug_callee"], vec![false]);
}

#[test]
fn return_statement_operand_is_return_position() {
    assert_eq!(flags()["return_callee"], vec![true]);
}

#[test]
fn argument_position_call_is_not_return_position() {
    // The outer call (arg_callee) is in return position; the inner call's
    // value flows into a callee slot, not to the return.
    assert_eq!(flags()["arg_inner_callee"], vec![false]);
}

#[test]
fn outer_return_operand_is_return_position() {
    assert_eq!(flags()["arg_callee"], vec![true]);
}
