//! Unit tests for method/receiver call capture in the Rust extractor
//! (082.001-T). Exercises `resolve_call_name`'s new `field_expression` arm via
//! the public `parse_rust_source` API.
//!
//! Scenarios (4):
//!   1. `x.foo()`         -> callee `foo`
//!   2. `self.bar()`      -> callee `bar`
//!   3. `x.clone()`       -> None (blocklisted, not captured)
//!   4. `a.b().c()`       -> both `b` and `c` captured (non-blocklisted)

#![allow(clippy::needless_raw_string_hashes)]

use engram::services::parsing::{ExtractedEdge, parse_rust_source};

/// Collect all `callee` names from the `Calls` edges of a parsed source.
fn callees(source: &str) -> Vec<String> {
    let result = parse_rust_source(source).expect("rust source must parse");
    result
        .edges
        .into_iter()
        .filter_map(|e| match e {
            ExtractedEdge::Calls { callee, .. } => Some(callee),
            _ => None,
        })
        .collect()
}

#[test]
fn method_call_on_receiver_captures_method_name() {
    let source = r#"
fn caller() {
    x.foo();
}
"#;
    let found = callees(source);
    assert!(
        found.iter().any(|c| c == "foo"),
        "x.foo() should capture method name `foo`, got {found:?}"
    );
}

#[test]
fn self_method_call_captures_method_name() {
    let source = r#"
fn caller() {
    self.bar();
}
"#;
    let found = callees(source);
    assert!(
        found.iter().any(|c| c == "bar"),
        "self.bar() should capture method name `bar`, got {found:?}"
    );
}

#[test]
fn blocklisted_method_call_is_not_captured() {
    let source = r#"
fn caller() {
    x.clone();
}
"#;
    let found = callees(source);
    assert!(
        !found.iter().any(|c| c == "clone"),
        "x.clone() is blocklisted and must NOT be captured, got {found:?}"
    );
}

#[test]
fn chained_method_calls_capture_all_non_blocklisted() {
    let source = r#"
fn caller() {
    a.b().c();
}
"#;
    let found = callees(source);
    assert!(
        found.iter().any(|c| c == "b"),
        "a.b().c() should capture `b`, got {found:?}"
    );
    assert!(
        found.iter().any(|c| c == "c"),
        "a.b().c() should capture `c`, got {found:?}"
    );
}
