//! Integration tests for qualification-/method-aware call resolution
//! (088-F / 088.002-T precision-guard harness).
//!
//! rec1 (082-F) promotes only unambiguous BARE-NAME cross-file calls to
//! `calls_resolved_singleton` edges. Path-qualified (`module::helper`,
//! `Type::method`) and method/receiver (`x.foo()`) calls are extracted but
//! dropped, conservatively losing recall to avoid false edges (findings 1 & 7).
//!
//! 088.003-T (path/module-qualified) and 088.004-T (method/Type-associated)
//! set out to recover that recall. Copilot review proved that only a `Self::`
//! call in an INHERENT impl is resolvable WITHOUT import/scope analysis: Rust
//! coherence (E0116) forbids an inherent `impl Widget` on a type defined outside
//! this crate, so such a `Self` is guaranteed workspace-local and `Self::method()`
//! rewrites to the exact `Type::method` index name, immune to re-exports,
//! imports, and module/type ambiguity. A `Self::` call in a TRAIT impl
//! (`impl Trait for Widget`, which MAY target an imported external type), and
//! every other qualified form (`crate::free`, bare `Type::method`, `module::free`,
//! `super::free`) and every method/receiver call, can be ambiguous without that
//! analysis (Option C, deferred by 013-D), so they DEFER to preserve the
//! no-false-edge invariant (findings 1 & 7). These scenarios pin both the
//! resolving inherent-impl `Self::` path and every deferral boundary.
//!
//! Scenarios (by function name):
//!   crate_root_free_fn_call_defers               crate::free()  -> NO edge
//!   crate_rooted_explicit_type_path_defers       crate::T::m()  -> NO edge
//!   self_ambiguous_type_method_creates_no_edge   Self:: (2 defs) -> NO edge
//!   type_qualified_does_not_fall_back_to_free_function  no free fallback
//!   method_receiver_call_stays_deferred          obj.render()   -> NO edge
//!   external_module_call_does_not_resolve_to_free_function  mem::swap()
//!   super_rooted_free_fn_call_defers             super::free()  -> NO edge
//!   nested_impl_self_call_does_not_mis_resolve_to_outer_type
//!   bare_type_qualified_call_defers_even_with_workspace_type
//!   qualified_call_with_local_same_name_creates_no_wrong_direct_edge
//!   self_qualified_call_resolves_cross_file      Self::m() -> ONE singleton
//!   self_call_on_path_qualified_impl_type_uses_full_type  full impl type text
//!   qualified_call_does_not_overwrite_direct_edge
//!   submodule_module_qualified_free_fn_defers    crate::pkg::free() -> NO edge
//!   trait_impl_self_call_defers                  Self:: in trait impl -> NO edge

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

// Scenario 1 (precision): a crate-root free-function call DEFERS. `crate::` does
// NOT prove workspace ownership — `pub use dep::do_work;` makes `crate::do_work()`
// an external re-export, and bare `do_work` could hit an unrelated local. Sound
// module/free-fn resolution needs import/scope analysis (Option C, deferred), so
// the call is deferred rather than risk a false edge.
#[test]
async fn crate_root_free_fn_call_defers() {
    let (_tmp, q) = index_files(&[
        ("src/a.rs", "pub fn caller() {\n    crate::do_work();\n}\n"),
        ("src/b.rs", "pub fn do_work() {}\n"),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "a crate-root free-fn call defers (cannot prove ownership vs a re-export)"
    );
    let do_work = q
        .resolve_reference_target("do_work")
        .await
        .expect("resolve do_work")
        .expect("do_work indexed");
    let edges = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons");
    assert!(
        !edges.iter().any(|(_, to)| to == &do_work),
        "must not target the unrelated free do_work, got {edges:?}"
    );
}

// Scenario 2 (precision): a crate-rooted EXPLICIT type path DEFERS. Without scope
// analysis the middle segment's module-vs-type identity is unknown, and the type
// may be an import/re-export, so `crate::Widget::build()` is deferred (only a
// `Self::` call, anchored to the concrete impl type, resolves a `Type::method`).
#[test]
async fn crate_rooted_explicit_type_path_defers() {
    let (_tmp, q) = index_files(&[
        (
            "src/a.rs",
            "pub fn caller() {\n    crate::Widget::build();\n}\n",
        ),
        (
            "src/b.rs",
            "pub struct Widget;\nimpl Widget {\n    pub fn build() {}\n}\n",
        ),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "an explicit crate-rooted type path defers; only Self:: resolves a Type::method"
    );
}

// Scenario 3 (precision): a `Self::` call whose `Type::method` is defined in two
// impl blocks is ambiguous and must create NO edge — the singleton guard on the
// only resolving path.
#[test]
async fn self_ambiguous_type_method_creates_no_edge() {
    let (_tmp, q) = index_files(&[
        (
            "src/a.rs",
            "pub struct Widget;\nimpl Widget {\n    pub fn run() {\n        Self::build();\n    }\n}\n",
        ),
        ("src/b.rs", "impl Widget {\n    pub fn build() {}\n}\n"),
        ("src/c.rs", "impl Widget {\n    pub fn build() {}\n}\n"),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "an ambiguous Self:: target (2 impl defs of Widget::build) must not create a singleton"
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
// `swap`. Module/free-fn resolution needs import/scope analysis (Option C,
// deferred), so every module-qualified call — external OR crate-rooted — defers.
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

// Scenario 8 (precision): a `super::`-rooted free-function call DEFERS. A crate
// root no longer proves workspace ownership (re-exports), so `super::helper()` is
// deferred like every non-`Self::` qualified call.
#[test]
async fn super_rooted_free_fn_call_defers() {
    let (_tmp, q) = index_files(&[
        ("src/a.rs", "pub fn caller() {\n    super::helper();\n}\n"),
        ("src/b.rs", "pub fn helper() {}\n"),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "a super::-rooted free-fn call defers (crate roots do not prove ownership)"
    );
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
// defines an unrelated local `do_work`, and it also makes a `crate::do_work()`
// call. That crate-root qualified call DEFERS (it is not `Self::`), so it must
// neither take the same-file bare-name fast path (a wrong `direct` edge to the
// local `do_work`) nor stage a singleton — it yields no edge at all.
#[test]
async fn qualified_call_with_local_same_name_creates_no_wrong_direct_edge() {
    let (_tmp, q) = index_files(&[
        (
            "src/a.rs",
            "pub fn caller() {\n    crate::do_work();\n}\n\npub fn do_work() {}\n",
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
        "the deferred crate-root qualified call resolves to no edge"
    );
}

// Scenario 12 (088.004 — Self:: cross-file): `Self::assist()` inside `impl Gadget`
// (a.rs) rewrites to the `Self::Gadget` marker, resolves to the exact
// `Gadget::assist` index name, and binds to the unique impl method in b.rs.
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

// Scenario 13 (C1 — Self:: on a path-qualified impl type uses the FULL type text):
// `Self::help()` inside `impl foo::Bar` must resolve to `foo::Bar::help` (matching
// the impl's index name exactly), NOT to the immediate segment `Bar::help`, so it
// cannot mis-bind to an unrelated `Bar::help` on a different `Bar`.
#[test]
async fn self_call_on_path_qualified_impl_type_uses_full_type() {
    let (_tmp, q) = index_files(&[
        (
            "src/a.rs",
            "impl foo::Bar {\n    fn run() {\n        Self::help();\n    }\n}\n",
        ),
        ("src/b.rs", "impl foo::Bar {\n    fn help() {}\n}\n"),
        // An unrelated `Bar::help` on a different `Bar` — must NOT be the target.
        (
            "src/c.rs",
            "pub struct Bar;\nimpl Bar {\n    fn help() {}\n}\n",
        ),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        1,
        "Self::help() must resolve to the unique foo::Bar::help, not the unrelated Bar::help"
    );
    assert_single_edge_to(&q, "foo::Bar::run", "foo::Bar::help").await;
    let bar_help = q
        .resolve_reference_target("Bar::help")
        .await
        .expect("resolve Bar::help")
        .expect("Bar::help indexed");
    let edges = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons");
    assert!(
        !edges.iter().any(|(_, to)| to == &bar_help),
        "must not target the unrelated Bar::help, got {edges:?}"
    );
}

// Scenario 14 (C2 — deferred qualified call leaves a direct edge intact): a
// caller makes both a `helper()` in-file bare call (a `direct` edge) and a
// `crate::helper()` call for the same target. The `crate::` call DEFERS (not
// `Self::`), so it stages nothing and cannot relabel or retract the `direct`
// edge — the in-file provenance is preserved. (The post-pass also keeps a
// direct-edge guard so a future resolving qualified form can never overwrite a
// `direct` edge either.)
#[test]
async fn qualified_call_does_not_overwrite_direct_edge() {
    let (_tmp, q) = index_files(&[(
        "src/a.rs",
        "pub fn caller() {\n    helper();\n    crate::helper();\n}\n\npub fn helper() {}\n",
    )])
    .await;
    let caller = q
        .resolve_reference_target("caller")
        .await
        .expect("resolve caller")
        .expect("caller indexed");
    let helper = q
        .resolve_reference_target("helper")
        .await
        .expect("resolve helper")
        .expect("helper indexed");
    let direct = q
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("list direct");
    assert!(
        direct.contains(&(caller.clone(), helper.clone())),
        "the in-file call must keep its direct edge, got {direct:?}"
    );
    let singletons = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons");
    assert!(
        !singletons.contains(&(caller, helper)),
        "the direct edge must not be overwritten to a singleton, got {singletons:?}"
    );
}

// Scenario 15 (C1/C3 boundary — submodule module free-fn defers): a
// submodule-qualified free-function call `crate::pkg::helper()` resolves to the
// EXACT `pkg::helper`, which is never indexed for a module free function (those
// are indexed by bare name), so it DEFERS — no false edge to the unrelated free
// `helper`. Sound module-path resolution of submodule free fns needs scope/index
// analysis (Option C, deferred).
#[test]
async fn submodule_module_qualified_free_fn_defers() {
    let (_tmp, q) = index_files(&[
        (
            "src/a.rs",
            "pub fn caller() {\n    crate::pkg::helper();\n}\n",
        ),
        ("src/b.rs", "pub fn helper() {}\n"),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "a submodule module free-fn call defers (pkg::helper is not indexed); no false edge"
    );
    let helper = q
        .resolve_reference_target("helper")
        .await
        .expect("resolve helper")
        .expect("helper indexed");
    let edges = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons");
    assert!(
        !edges.iter().any(|(_, to)| to == &helper),
        "must not target the unrelated free `helper`, got {edges:?}"
    );
}

// Scenario 16 (Copilot round-8 finding — trait-impl Self:: defers): a `Self::`
// call inside a TRAIT impl (`impl Trait for Widget`) must DEFER. Rust coherence
// (E0116) forbids an INHERENT `impl Widget` on an external type, but a TRAIT impl
// may target an imported external type (`use dep::Widget;`). Resolving its
// `Self::build()` to a same-named but UNRELATED local `Widget::build` would be a
// false edge, so trait-impl `Self::` is not marked and defers. Only inherent-impl
// `Self::` (coherence-guaranteed workspace-local) resolves.
#[test]
async fn trait_impl_self_call_defers() {
    let (_tmp, q) = index_files(&[
        (
            "src/a.rs",
            "pub trait LocalTrait {\n    fn run(&self);\n}\nimpl LocalTrait for Widget {\n    fn run(&self) {\n        Self::build();\n    }\n}\n",
        ),
        (
            "src/b.rs",
            "pub struct Widget;\nimpl Widget {\n    pub fn build() {}\n}\n",
        ),
    ])
    .await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "a Self:: call in a trait impl must defer (coherence lets the impl type be external)"
    );
    let build = q
        .resolve_reference_target("Widget::build")
        .await
        .expect("resolve Widget::build")
        .expect("Widget::build indexed");
    let edges = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons");
    assert!(
        !edges.iter().any(|(_, to)| to == &build),
        "must not target the unrelated local Widget::build, got {edges:?}"
    );
}
