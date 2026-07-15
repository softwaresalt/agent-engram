//! Unit tests for qualifier capture in the Rust extractor (088-F / 088.002-T).
//!
//! The extractor already reduces a path-qualified call `A::b()` to its bare
//! final segment `b` and marks it `is_qualified`. Qualification-aware resolution
//! (088.004-T) additionally needs the full path prefix (`crate::util` for
//! `crate::util::helper()`, `Self::<EnclosingType>` for a `Self::` call) so the
//! router can recognise the one provably workspace-owned form — a `Self::` call —
//! and defer every other prefix.
//!
//! These scenarios assert the `qualifier` field on `ExtractedEdge::Calls`. The
//! extractor CAPTURES the qualifier for every path-qualified call regardless of
//! whether the resolver promotes it; a `Self` root is rewritten to the
//! `Self::<EnclosingType>` marker. The bare `callee` value is unchanged, so the
//! 082.001-T extraction contract (`rust_method_call_extraction_test`) still holds.

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
// also carry the immediate qualifier `Type`. Extraction captures the qualifier
// for every path-qualified call; the resolver then DEFERS this bare `Type::method`
// form (only `Self::` resolves), so the capture must still be present and correct
// even though it is not promoted (the finding-1/finding-7 false-edge case).
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
// carry the immediate qualifier `module`. Extraction captures the qualifier for
// every path-qualified call; the resolver then DEFERS this module form (only
// `Self::` resolves — a module qualifier cannot be proven workspace-owned without
// scope/import analysis), so the capture must still be present and correct.
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
// path prefix (`crate::util`), not just the immediate segment. Extraction
// captures the whole prefix so the router has complete information; this module
// form then defers (only `Self::` resolves), but the capture must be exact.
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

// A `Self::build()` call inside `impl Widget` must rewrite `Self` to the
// `Self::<EnclosingType>` marker (`Self::Widget`), so the resolver matches the
// exact `Widget::build` index name — the only qualified form that is provably
// workspace-owned.
#[test]
fn self_qualified_call_rewrites_to_self_enclosing_type_marker() {
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
            .any(|(c, _, q, qual)| c == "build" && *q && qual.as_deref() == Some("Self::Widget")),
        "Self::build() in impl Widget must carry qualifier Some(\"Self::Widget\"), got {found:?}"
    );
}

// A `Self::build()` call inside a TRAIT impl (`impl Trait for Widget`) must NOT
// be rewritten to the `Self::Widget` marker. Rust coherence lets a trait impl
// target an imported external type, so its `Self` is not provably workspace-local
// (`use dep::Widget; impl Trait for Widget`). The qualifier stays the raw `Self`,
// which the resolver defers — closing the trait-impl false-edge vector.
#[test]
fn self_call_in_trait_impl_is_not_marked() {
    let source = r#"
struct Widget;
trait Draw {
    fn run(&self);
}
impl Draw for Widget {
    fn run(&self) {
        Self::build();
    }
}
"#;
    let found = calls(source);
    assert!(
        found
            .iter()
            .any(|(c, _, q, qual)| c == "build" && *q && qual.as_deref() == Some("Self")),
        "Self::build() in a trait impl must keep the raw `Self` qualifier (deferred), got {found:?}"
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
