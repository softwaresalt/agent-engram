//! Integration tests for qualification-/method-aware call resolution
//! (088-F / 088.002-T precision-guard harness).
//!
//! rec1 (082-F) promotes only unambiguous BARE-NAME cross-file calls to
//! `calls_resolved_singleton` edges. Path-qualified (`module::helper`,
//! `Type::method`) and method/receiver (`x.foo()`) calls are extracted but
//! dropped, conservatively losing recall to avoid false edges (findings 1 & 7).
//!
//! 088.003-T (path/module-qualified) and 088.004-T (method/Type-associated)
//! recover that recall WITHOUT reintroducing false edges, using Option A from
//! deliberation 013-D (qualified-name exact, singleton-only). These scenarios
//! pin the required behavior and FAIL until those resolvers land.
//!
//! Scenarios:
//!   1. unambiguous `module::helper` cross-file  -> ONE singleton (088.003)
//!   2. unambiguous `Type::method` cross-file    -> ONE singleton (088.004)
//!   3. ambiguous module target (2 free defs)    -> NO edge  (precision)
//!   4. ambiguous `Type::method` (2 impl defs)   -> NO edge  (precision)
//!   5. `Type::parse` with only a free `parse`   -> NO edge  (the finding-1/7
//!      guard: a type-qualified call must NOT fall back to an unrelated free
//!      function of the same bare name)
//!   6. method/receiver `obj.render()`           -> NO edge  (deferred / Option B)

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use std::fs;
use std::path::Path;

use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;

fn write_sample_file(dir: &Path, rel_path: &str, content: &str) {
    let full = dir.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create dirs");
    }
    fs::write(full, content).expect("write file");
}

fn test_db_params(path: &Path) -> (std::path::PathBuf, String) {
    use sha2::{Digest, Sha256};
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    let branch = format!("{:x}", Sha256::digest(canon.as_bytes()));
    (std::env::temp_dir().join("engram-test"), branch)
}

async fn queries_for(data_dir: &Path, branch: &str) -> CodeGraphQueries {
    let db = connect_db(data_dir, branch).await.expect("connect_db");
    CodeGraphQueries::new(db)
}

async fn singleton_count(q: &CodeGraphQueries) -> u64 {
    q.count_calls_edges_by_resolution()
        .await
        .expect("count_calls_edges_by_resolution")
        .get("calls_resolved_singleton")
        .copied()
        .unwrap_or(0)
}

/// Index `files` in a fresh workspace via the real full-index path (which runs
/// the deferred cross-file post-pass) and return queries over the resulting db.
async fn index_files(files: &[(&str, &str)]) -> (tempfile::TempDir, CodeGraphQueries) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    for (rel, content) in files {
        write_sample_file(ws, rel, content);
    }
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");
    let q = queries_for(&data_dir, &branch).await;
    (tmp, q)
}

/// Assert a single `calls_resolved_singleton` edge exists from the function
/// named `caller` to the function whose (possibly qualified) index name is
/// `target`, by exact identity.
async fn assert_single_edge_to(q: &CodeGraphQueries, caller: &str, target: &str) {
    let caller_id = q
        .resolve_reference_target(caller)
        .await
        .expect("resolve caller")
        .unwrap_or_else(|| panic!("caller `{caller}` must be indexed"));
    let target_id = q
        .resolve_reference_target(target)
        .await
        .expect("resolve target")
        .unwrap_or_else(|| panic!("target `{target}` must be indexed"));
    let edges = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons");
    assert_eq!(edges.len(), 1, "exactly one singleton edge, got {edges:?}");
    assert_eq!(
        edges[0],
        (caller_id, target_id),
        "the singleton must connect `{caller}` -> `{target}` by exact identity"
    );
}

// Scenario 1 (088.003): an unambiguous crate-rooted module call resolves to
// exactly one singleton edge whose target is the bare free function. The crate
// root (`crate`/`self`/`super`) proves the target is in this workspace; because
// functions are indexed by bare name (module paths are NOT part of the index),
// resolution matches the bare final segment singleton-only. The same-named
// ambiguity guard is Scenario 3.
#[test]
async fn module_qualified_unambiguous_resolves_to_singleton() {
    let (_tmp, q) = index_files(&[
        (
            "src/a.rs",
            "pub fn caller() {\n    crate::helpers::do_work();\n}\n",
        ),
        ("src/b.rs", "pub fn do_work() {}\n"),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        1,
        "crate::module::helper() with a unique free target must create one singleton"
    );
    assert_single_edge_to(&q, "caller", "do_work").await;
}

// Scenario 2 (088.004): an unambiguous crate-rooted type-associated cross-file
// call resolves to exactly one singleton edge whose target is the `Type::method`
// impl method. Crate-rooted proves the type is in this workspace.
#[test]
async fn type_qualified_unambiguous_resolves_to_singleton() {
    let (_tmp, q) = index_files(&[
        (
            "src/a.rs",
            "pub fn caller() {\n    crate::widgets::Widget::build();\n}\n",
        ),
        (
            "src/b.rs",
            "pub struct Widget;\nimpl Widget {\n    pub fn build() {}\n}\n",
        ),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        1,
        "crate-rooted Type::method() with a unique impl target must create one singleton"
    );
    assert_single_edge_to(&q, "caller", "Widget::build").await;
}

// Scenario 3 (precision): a crate-rooted module call whose bare target is defined
// twice is ambiguous and must create NO edge.
#[test]
async fn module_qualified_ambiguous_creates_no_edge() {
    let (_tmp, q) = index_files(&[
        (
            "src/a.rs",
            "pub fn caller() {\n    crate::helpers::do_work();\n}\n",
        ),
        ("src/b.rs", "pub fn do_work() {}\n"),
        ("src/c.rs", "pub fn do_work() {}\n"),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "an ambiguous module target (2 defs) must not create a singleton edge"
    );
}

// Scenario 4 (precision): a crate-rooted type-associated call whose
// `Type::method` is defined in two impl blocks is ambiguous and must create NO
// edge.
#[test]
async fn type_qualified_ambiguous_creates_no_edge() {
    let (_tmp, q) = index_files(&[
        (
            "src/a.rs",
            "pub fn caller() {\n    crate::widgets::Widget::build();\n}\n",
        ),
        (
            "src/b.rs",
            "pub struct Widget;\nimpl Widget {\n    pub fn build() {}\n}\n",
        ),
        ("src/c.rs", "impl Widget {\n    pub fn build() {}\n}\n"),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "an ambiguous Type::method (2 impl defs) must not create a singleton edge"
    );
}

// Scenario 5 (the finding-1/finding-7 guard): a crate-rooted type-qualified call
// `crate::Thing::parse()` whose `Thing::parse` is NOT indexed must NOT fall back
// to an unrelated unique free function `parse` — that would be a false edge.
#[test]
async fn type_qualified_does_not_fall_back_to_free_function() {
    let (_tmp, q) = index_files(&[
        (
            "src/a.rs",
            "pub fn caller() {\n    crate::Thing::parse();\n}\n",
        ),
        ("src/b.rs", "pub fn parse() {}\n"),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "a type-qualified call must not resolve to an unrelated free function of the same bare name"
    );
    // The free `parse` must have no incoming resolved edge.
    let parse_id = q
        .resolve_reference_target("parse")
        .await
        .expect("resolve parse")
        .expect("free `parse` is indexed");
    let edges = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons");
    assert!(
        !edges.iter().any(|(_, to)| to == &parse_id),
        "no singleton edge may target the unrelated free `parse`, got {edges:?}"
    );
}

// Scenario 6 (deferred / Option B): a method/receiver call `obj.render()` cannot
// resolve without receiver-type inference, so it must create NO edge even though
// `Screen::render` exists.
#[test]
async fn method_receiver_call_stays_deferred() {
    let (_tmp, q) = index_files(&[
        ("src/a.rs", "pub fn caller() {\n    obj.render();\n}\n"),
        (
            "src/b.rs",
            "pub struct Screen;\nimpl Screen {\n    pub fn render(&self) {}\n}\n",
        ),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "a value-receiver method call must stay deferred (no false edge)"
    );
}

// Scenario 7 (THE finding-1/finding-7 guard for MODULE routing): an EXTERNAL
// module call `mem::swap()` must NOT resolve to an unrelated workspace free
// `swap`. Only crate-rooted (`crate`/`self`/`super`) module calls are proven
// in-workspace; an opaque external qualifier is deferred.
#[test]
async fn external_module_call_does_not_resolve_to_free_function() {
    let (_tmp, q) = index_files(&[
        ("src/a.rs", "pub fn caller() {\n    mem::swap();\n}\n"),
        ("src/b.rs", "pub fn swap() {}\n"),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "an external module call must not resolve to an unrelated free fn of the same bare name"
    );
    let swap_id = q
        .resolve_reference_target("swap")
        .await
        .expect("resolve swap")
        .expect("free `swap` is indexed");
    let edges = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons");
    assert!(
        !edges.iter().any(|(_, to)| to == &swap_id),
        "no singleton edge may target the unrelated free `swap`, got {edges:?}"
    );
}

// Scenario 8 (088.003): a `super::`-rooted module call is crate-internal, so the
// bare target is guaranteed in-workspace and a unique match resolves safely.
#[test]
async fn crate_internal_super_rooted_call_resolves() {
    let (_tmp, q) = index_files(&[
        ("src/a.rs", "pub fn caller() {\n    super::helper();\n}\n"),
        ("src/b.rs", "pub fn helper() {}\n"),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        1,
        "a super::-rooted call to a unique free fn must resolve"
    );
    assert_single_edge_to(&q, "caller", "helper").await;
}

// Scenario 9 (F2 — nested-impl Self): a `Self::` call inside an impl nested in
// another impl's method must NOT be rewritten to the OUTER enclosing type. The
// nested impl is skipped by the call walk, so no edge (direct or singleton) is
// created from the outer method to the outer type's same-named method.
#[test]
async fn nested_impl_self_call_does_not_mis_resolve_to_outer_type() {
    let (_tmp, q) = index_files(&[(
        "src/a.rs",
        "pub struct Outer;\nimpl Outer {\n    pub fn run() {\n        struct Inner;\n        impl Inner {\n            fn go() {\n                Self::helper();\n            }\n            fn helper() {}\n        }\n    }\n    pub fn helper() {}\n}\n",
    )])
    .await;
    let run_id = q
        .resolve_reference_target("Outer::run")
        .await
        .expect("resolve Outer::run")
        .expect("Outer::run indexed");
    let outer_helper_id = q
        .resolve_reference_target("Outer::helper")
        .await
        .expect("resolve Outer::helper")
        .expect("Outer::helper indexed");
    for resolution in ["direct", "calls_resolved_singleton"] {
        let edges = q
            .list_calls_edges_by_resolution(resolution)
            .await
            .expect("list edges");
        assert!(
            !edges.contains(&(run_id.clone(), outer_helper_id.clone())),
            "nested impl Self::helper() must not create an Outer::run -> Outer::helper {resolution} edge, got {edges:?}"
        );
    }
}

// Scenario 10 (C1 — type-route workspace-identity guard): a BARE `Widget::build()`
// (no crate root) cannot be shown to reference the workspace `Widget` rather than
// an identically-named external type, so it must DEFER — even though a unique
// workspace `Widget::build` exists. This is the type-route analog of the
// module-route precision guard.
#[test]
async fn bare_type_qualified_call_defers_even_with_workspace_type() {
    let (_tmp, q) = index_files(&[
        ("src/a.rs", "pub fn caller() {\n    Widget::build();\n}\n"),
        (
            "src/b.rs",
            "pub struct Widget;\nimpl Widget {\n    pub fn build() {}\n}\n",
        ),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "a bare (non-crate-rooted) Type::method() must defer to avoid an external-type collision"
    );
}

// Scenario 11 (C2/C3 — no wrong same-file direct edge): the caller's own file
// defines an unrelated local `do_work`, while the crate-rooted call targets
// another module. A qualified call must NOT take the same-file bare-name fast
// path (which would wrongly bind to the local `do_work`); it is staged for global
// singleton resolution, and since `do_work` is now ambiguous, it yields no edge.
#[test]
async fn qualified_call_with_local_same_name_creates_no_wrong_direct_edge() {
    let (_tmp, q) = index_files(&[
        (
            "src/a.rs",
            "pub fn caller() {\n    crate::helpers::do_work();\n}\n\npub fn do_work() {}\n",
        ),
        ("src/b.rs", "pub fn do_work() {}\n"),
    ])
    .await;
    let direct = q
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("list direct");
    assert!(
        direct.is_empty(),
        "a qualified call must not create a same-file direct edge, got {direct:?}"
    );
    assert_eq!(
        singleton_count(&q).await,
        0,
        "the now-ambiguous qualified target resolves to no edge"
    );
}

// Scenario 12 (088.004 — Self:: cross-file): `Self::assist()` inside `impl Gadget`
// (a.rs) rewrites to the crate-rooted `Gadget::assist` and resolves to the unique
// impl method in b.rs.
#[test]
async fn self_qualified_call_resolves_cross_file() {
    let (_tmp, q) = index_files(&[
        (
            "src/a.rs",
            "pub struct Gadget;\nimpl Gadget {\n    pub fn run() {\n        Self::assist();\n    }\n}\n",
        ),
        ("src/b.rs", "impl Gadget {\n    pub fn assist() {}\n}\n"),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        1,
        "Self::assist() must resolve to the unique Gadget::assist impl method"
    );
    assert_single_edge_to(&q, "Gadget::run", "Gadget::assist").await;
}
