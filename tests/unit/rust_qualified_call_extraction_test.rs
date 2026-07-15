//! Unit tests for qualifier capture in the Rust extractor (088-F / 088.002-T).
//!
//! The extractor already reduces a path-qualified call `A::b()` to its bare
//! final segment `b` and marks it `is_qualified`. Qualification-aware resolution
//! (088.003-T / 088.004-T) additionally needs the full path prefix
//! (`crate::util` for `crate::util::helper()`) so it can gate on a crate-internal
//! root and distinguish a type-associated target (`Type::method`) from a module
//! target (`module::helper`).
//!
//! These scenarios assert the new `qualifier` field on `ExtractedEdge::Calls`.
//! They COMPILE against the scaffolded field but FAIL until 088.003-T populates
//! the prefix and 088.004-T rewrites `Self` to a crate-rooted path. The bare
//! `callee` value is unchanged, so the 082.001-T extraction contract
//! (`rust_method_call_extraction_test`) still holds.

#![allow(clippy::needless_raw_string_hashes)]

use engram::services::parsing::{ExtractedEdge, parse_rust_source};

/// Collect `(callee, is_method, is_qualified, qualifier)` tuples from the
/// `Calls` edges of a parsed source.
fn calls(source: &str) -> Vec<(String, bool, bool, Option<String>)> {
    let result = parse_rust_source(source).expect("rust source must parse");
    result
        .edges
        .into_iter()
        .filter_map(|e| match e {
            ExtractedEdge::Calls {
                callee,
                is_method,
                is_qualified,
                qualifier,
                ..
            } => Some((callee, is_method, is_qualified, qualifier)),
            _ => None,
        })
        .collect()
}

// A type-associated call `Type::parse()` keeps its bare callee `parse` but must
// now also carry the immediate qualifier `Type`, so the resolver can match it
// against the impl-method index name `Type::parse` instead of an unrelated free
// `parse` (the finding-1/finding-7 false-edge case).
#[test]
fn type_qualified_call_carries_type_qualifier() {
    let source = r#"
fn caller() {
    Type::parse();
}
"#;
    let found = calls(source);
    assert!(
        found
            .iter()
            .any(|(c, m, q, qual)| c == "parse" && !*m && *q && qual.as_deref() == Some("Type")),
        "Type::parse() must capture callee `parse`, is_qualified, qualifier Some(\"Type\"), got {found:?}"
    );
}

// A module-path call `module::helper()` keeps its bare callee `helper` and must
// carry the immediate qualifier `module`, so the resolver knows the qualifier is
// a module (lower-case) and resolves the bare free-function name.
#[test]
fn module_qualified_call_carries_module_qualifier() {
    let source = r#"
fn caller() {
    module::helper();
}
"#;
    let found = calls(source);
    assert!(
        found
            .iter()
            .any(|(c, m, q, qual)| c == "helper" && !*m && *q && qual.as_deref() == Some("module")),
        "module::helper() must carry qualifier Some(\"module\"), got {found:?}"
    );
}

// For a multi-segment path `crate::util::helper()` the qualifier is the FULL
// path prefix (`crate::util`), not just the immediate segment — the resolver
// needs the root (`crate`) to gate the in-workspace module route and the
// immediate segment (`util`) to detect a type qualifier.
#[test]
fn multi_segment_path_captures_full_prefix() {
    let source = r#"
fn caller() {
    crate::util::helper();
}
"#;
    let found = calls(source);
    assert!(
        found
            .iter()
            .any(|(c, _, q, qual)| c == "helper" && *q && qual.as_deref() == Some("crate::util")),
        "crate::util::helper() must carry the full prefix Some(\"crate::util\"), got {found:?}"
    );
}

// A `Self::build()` call inside `impl Widget` must rewrite `Self` to a
// crate-rooted path carrying the concrete enclosing type (`crate::Widget`), so
// the resolver treats it as a workspace-identified `Widget::build`.
#[test]
fn self_qualified_call_rewrites_to_crate_rooted_enclosing_type() {
    let source = r#"
struct Widget;
impl Widget {
    fn make() {
        Self::build();
    }
    fn build() {}
}
"#;
    let found = calls(source);
    assert!(
        found
            .iter()
            .any(|(c, _, q, qual)| c == "build" && *q && qual.as_deref() == Some("crate::Widget")),
        "Self::build() in impl Widget must carry qualifier Some(\"crate::Widget\"), got {found:?}"
    );
}

// Method / receiver calls and bare identifier calls carry NO qualifier: the
// former stay deferred (they need receiver-type inference), the latter resolve
// by bare name as before.
#[test]
fn method_and_bare_calls_have_no_qualifier() {
    let source = r#"
fn caller() {
    x.foo();
    free_fn();
}
"#;
    let found = calls(source);
    assert!(
        found
            .iter()
            .any(|(c, m, q, qual)| c == "foo" && *m && !*q && qual.is_none()),
        "x.foo() must be is_method with qualifier None, got {found:?}"
    );
    assert!(
        found
            .iter()
            .any(|(c, m, q, qual)| c == "free_fn" && !*m && !*q && qual.is_none()),
        "free_fn() must have qualifier None, got {found:?}"
    );
}
