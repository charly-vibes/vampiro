//! Return-position gating for the return-boundary composition check
//! (vampiro-224.9).
//!
//! The return-boundary check compares callee codomain vs caller codomain —
//! meaningful only when the callee's result actually flows to the caller's
//! return. The frontend tags each call-result edge with `return_position`;
//! these tests pin which call contexts are (and are not) return position.

use std::collections::HashMap;
use std::path::Path;

use vampiro_cir::{CirGraph, Frontend, NodeKind};
use vampiro_rust_frontend::RustFrontend;

fn extract(source: &str) -> CirGraph {
    RustFrontend
        .extract(source, Path::new("src/lib.rs"))
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
fn stmt_callee() -> i32 { 1 }
fn let_callee() -> i32 { 2 }
fn tail_callee() -> i32 { 3 }
fn return_callee() -> i32 { 4 }
fn cond_callee() -> bool { true }
fn inner_callee() -> i32 { 5 }
fn outer_callee(x: i32) -> i32 { x }
fn try_callee() -> Result<i32, ()> { Ok(6) }
fn if_callee() -> i32 { 7 }
fn if_stmt_callee() -> i32 { 8 }

fn stmt_call() -> i32 {
    stmt_callee();
    0
}

fn let_bound() -> i32 {
    let _x = let_callee();
    0
}

fn tail_call() -> i32 {
    tail_callee()
}

fn explicit_return() -> i32 {
    if cond_callee() {
        return return_callee();
    }
    0
}

fn tail_nested() -> i32 {
    outer_callee(inner_callee())
}

fn tail_try() -> i32 {
    try_callee()?
}

fn tail_if() -> i32 {
    if cond_callee() {
        if_stmt_callee();
        if_callee()
    } else {
        0
    }
}
"#;

fn flags() -> HashMap<String, Vec<bool>> {
    call_result_flags(&extract(SOURCE))
}

#[test]
fn statement_position_call_is_not_return_position() {
    assert_eq!(flags()["stmt_callee"], vec![false]);
}

#[test]
fn let_bound_call_is_not_return_position() {
    assert_eq!(flags()["let_callee"], vec![false]);
}

#[test]
fn tail_call_is_return_position() {
    assert_eq!(flags()["tail_callee"], vec![true]);
}

#[test]
fn return_operand_is_return_position_but_condition_is_not() {
    let f = flags();
    assert_eq!(f["return_callee"], vec![true]);
    // cond_callee appears twice (explicit_return + tail_if conditions) —
    // both are control-flow conditions, never return position.
    assert!(f["cond_callee"].iter().all(|&flag| !flag));
}

#[test]
fn nested_tail_call_only_flags_outermost() {
    let f = flags();
    assert_eq!(f["outer_callee"], vec![true]);
    assert_eq!(f["inner_callee"], vec![false]);
}

#[test]
fn try_tail_is_return_position() {
    assert_eq!(flags()["try_callee"], vec![true]);
}

#[test]
fn tail_if_arm_is_return_position_but_block_statement_is_not() {
    let f = flags();
    assert_eq!(f["if_callee"], vec![true]);
    assert_eq!(f["if_stmt_callee"], vec![false]);
    // cond_callee also appears in tail_if's condition — still not return
    // position.
    assert!(f["cond_callee"].iter().all(|&flag| !flag));
}
