//! Unit tests for method/receiver call capture in the Rust extractor
//! (082.001-T). Exercises `resolve_call_name`'s `field_expression` and
//! `scoped_identifier` arms via the public `parse_rust_source` API.
//!
//! Scenarios (6):
//!   1. `x.foo()`         -> callee `foo`
//!   2. `self.bar()`      -> callee `bar`
//!   3. `x.clone()`       -> None (blocklisted, not captured)
//!   4. `a.b().c()`       -> both `b` and `c` captured (non-blocklisted)
//!   5. method calls are marked `is_method` (not promoted); free calls are not
//!   6. qualified calls (`Type::parse()`) are marked `is_qualified` (not
//!      promoted); bare identifier calls are not

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

/// Collect `(callee, is_method, is_qualified)` tuples from the `Calls` edges of
/// a parsed source.
fn calls(source: &str) -> Vec<(String, bool, bool)> {
    let result = parse_rust_source(source).expect("rust source must parse");
    result
        .edges
        .into_iter()
        .filter_map(|e| match e {
            ExtractedEdge::Calls {
                callee,
                is_method,
                is_qualified,
                ..
            } => Some((callee, is_method, is_qualified)),
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

// Method / receiver calls must be MARKED `is_method` so the code-graph staging
// step never promotes them to a `calls_edge` (they cannot resolve correctly
// under name-only matching and would risk a false singleton edge). Free-function
// calls must remain `is_method == false` so they still resolve/stage as before.
#[test]
fn method_calls_marked_is_method_free_calls_not() {
    let source = r#"
fn caller() {
    self.bar();
    free_fn();
}
"#;
    let pairs = calls(source);
    assert!(
        pairs.iter().any(|(c, m, _)| c == "bar" && *m),
        "self.bar() must be marked is_method=true, got {pairs:?}"
    );
    assert!(
        pairs.iter().any(|(c, m, q)| c == "free_fn" && !*m && !*q),
        "free_fn() must be marked is_method=false is_qualified=false, got {pairs:?}"
    );
}

// Path-qualified calls (`Type::assoc()`, `module::helper()`) are reduced to the
// bare final segment here, but must be MARKED `is_qualified` so the code-graph
// staging step never promotes them: a `Type::parse()` call would otherwise
// resolve to an unrelated unique top-level `parse` (a false singleton edge),
// since type-associated targets are indexed under their qualified name. Free
// (unqualified) identifier calls must remain `is_qualified == false` so they
// still resolve/stage as before.
#[test]
fn scoped_calls_marked_is_qualified_bare_calls_not() {
    let source = r#"
fn caller() {
    Type::parse();
    module::helper();
    free_fn();
}
"#;
    let pairs = calls(source);
    assert!(
        pairs.iter().any(|(c, m, q)| c == "parse" && !*m && *q),
        "Type::parse() must be marked is_qualified=true is_method=false, got {pairs:?}"
    );
    assert!(
        pairs.iter().any(|(c, m, q)| c == "helper" && !*m && *q),
        "module::helper() must be marked is_qualified=true, got {pairs:?}"
    );
    assert!(
        pairs.iter().any(|(c, m, q)| c == "free_fn" && !*m && !*q),
        "free_fn() must be marked is_qualified=false, got {pairs:?}"
    );
}
