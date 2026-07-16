//! Integration tests for Option C Unit B canonical call resolution (091.012-T /
//! 091.013-T).
//!
//! Canonical resolution is precision-gated: a staged qualified or known-receiver
//! call may produce a `calls_resolved_canonical` edge only when the canonical
//! path resolves to exactly one non-empty `function_meta.canonical_path`.

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;
use engram::services::retrieval_eval::evaluate_target_correctness;

fn write_file(ws: &Path, rel: &str, content: &str) {
    let full = ws.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create dirs");
    }
    fs::write(full, content).expect("write file");
}

fn write_manifest(ws: &Path) {
    write_file(
        ws,
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
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

async fn index(ws: &Path) -> CodeGraphQueries {
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    CodeGraphQueries::new(db)
}

async fn name_to_ids(q: &CodeGraphQueries) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for f in q.all_functions().await.expect("all_functions") {
        map.entry(f.name).or_default().push(f.id);
    }
    map
}

async fn canonical_edges(q: &CodeGraphQueries) -> HashSet<(String, String)> {
    q.list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges")
        .into_iter()
        .collect()
}

#[test]
async fn canonical_singleton_qualified_and_self_calls_create_edges() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(ws, "src/lib.rs", "pub mod caller;\npub mod util;\n");
    write_file(
        ws,
        "src/util.rs",
        r#"
pub fn helper() {}

pub struct Widget;

impl Widget {
    pub fn build() {}
    pub fn call_self(&self) {
        self.build();
    }
    pub fn call_self_type() {
        Self::build();
    }
}
"#,
    );
    write_file(
        ws,
        "src/caller.rs",
        r#"
pub fn caller() {
    crate::util::helper();
    crate::util::Widget::build();
}
"#,
    );

    let q = index(ws).await;
    let names = name_to_ids(&q).await;
    let edges = canonical_edges(&q).await;

    let caller_id = names["caller"][0].clone();
    let helper_id = names["helper"][0].clone();
    let build_id = names["Widget::build"][0].clone();
    let call_self_id = names["Widget::call_self"][0].clone();
    let call_self_type_id = names["Widget::call_self_type"][0].clone();

    for expected in [
        (caller_id.clone(), helper_id),
        (caller_id, build_id.clone()),
        (call_self_id, build_id.clone()),
        (call_self_type_id, build_id),
    ] {
        assert!(
            edges.contains(&expected),
            "expected canonical edge {expected:?}; got {edges:?}"
        );
    }
}

#[test]
async fn bare_and_qualified_call_to_same_in_file_target_stays_direct() {
    // Regression (Cycle-5 Finding A): a caller that reaches the same in-file
    // function BOTH bare (`helper()`, resolved to a `direct` edge at parse) and
    // via a qualified path (`crate::thing::helper()`, staged for the canonical
    // pass) must NOT have its `direct` edge downgraded to
    // `calls_resolved_canonical`. `calls_edge` is keyed by `(from, to)`, so a
    // canonical overwrite would (a) double-count the pair in `edges_created` and
    // (b) make the down-migration rollback — which retracts canonical edges —
    // delete an edge that represents a genuine direct in-file call.
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(ws, "src/lib.rs", "pub mod thing;\n");
    write_file(
        ws,
        "src/thing.rs",
        "pub fn helper() {}\npub fn caller() {\n    helper();\n    crate::thing::helper();\n}\n",
    );

    let q = index(ws).await;
    let names = name_to_ids(&q).await;
    let caller_id = names["caller"][0].clone();
    let helper_id = names["helper"][0].clone();
    let pair = (caller_id, helper_id);

    let canonical = canonical_edges(&q).await;
    assert!(
        !canonical.contains(&pair),
        "the direct edge must not be downgraded to canonical; canonical={canonical:?}"
    );

    let direct: HashSet<(String, String)> = q
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("list direct edges")
        .into_iter()
        .collect();
    assert!(
        direct.contains(&pair),
        "the in-file call must remain a direct edge; direct={direct:?}"
    );
}

#[test]
async fn distinct_qualifiers_same_callee_name_both_resolve() {
    // Regression (091.012-T / C2): two qualified calls to the same callee NAME
    // but different qualifiers in one caller must BOTH resolve. `raw_qualifier`
    // is part of the `staged_call` key, so the two rows no longer collide and
    // overwrite one another — which previously dropped one of the two canonical
    // edges (fail-closed recall loss).
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(
        ws,
        "src/lib.rs",
        "pub mod a;\npub mod b;\npub mod caller;\n",
    );
    write_file(ws, "src/a.rs", "pub fn build() {}\n");
    write_file(ws, "src/b.rs", "pub fn build() {}\n");
    write_file(
        ws,
        "src/caller.rs",
        "pub fn caller() {\n    crate::a::build();\n    crate::b::build();\n}\n",
    );

    let q = index(ws).await;
    let names = name_to_ids(&q).await;
    let edges = canonical_edges(&q).await;

    let caller_id = names["caller"][0].clone();
    let build_ids = &names["build"];
    assert_eq!(
        build_ids.len(),
        2,
        "expected two distinct build definitions, got {build_ids:?}"
    );
    for build_id in build_ids {
        assert!(
            edges.contains(&(caller_id.clone(), build_id.clone())),
            "expected canonical edge caller->{build_id}; got {edges:?}"
        );
    }
}

#[test]
async fn ambiguous_duplicate_canonical_path_emits_no_edge() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(ws, "src/lib.rs", "pub mod a;\npub mod caller;\n");
    write_file(ws, "src/a.rs", "pub fn build() {}\npub fn build() {}\n");
    write_file(
        ws,
        "src/caller.rs",
        "pub fn caller() {\n    crate::a::build();\n}\n",
    );

    let q = index(ws).await;
    let edges = canonical_edges(&q).await;
    assert!(
        edges.is_empty(),
        "duplicate canonical targets must fail closed, got {edges:?}"
    );
}

#[test]
async fn adversarial_precision_fixtures_emit_no_canonical_false_edges() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(
        ws,
        "src/lib.rs",
        r#"
pub mod f1;
pub mod f6;
pub mod glob_collision;
pub mod generic_convergence;
pub mod self_forge;
pub mod shadow_std;
pub mod primitive_qualifier;
pub mod local_alias_shadow;
pub mod trait_impl_self;
"#,
    );
    write_file(
        ws,
        "src/f1.rs",
        "use std::mem;\npub fn caller() { mem::swap(); }\npub fn swap() {}\n",
    );
    write_file(
        ws,
        "src/f6.rs",
        "use ext::Widget as Alias;\npub struct Widget;\nimpl Widget { pub fn build() {} }\npub fn caller() { Alias::build(); }\n",
    );
    write_file(
        ws,
        "src/glob_collision.rs",
        "use crate::f6::*;\npub struct Widget;\nimpl Widget { pub fn build() {} }\npub fn caller() { Widget::build(); }\n",
    );
    write_file(
        ws,
        "src/generic_convergence.rs",
        "pub struct Widget<T>(T);\nimpl Widget<u8> { pub fn build() {} }\nimpl Widget<u16> { pub fn build() {} }\npub fn caller() { Widget::build(); }\n",
    );
    write_file(
        ws,
        "src/self_forge.rs",
        "pub struct Widget;\nimpl Widget { pub fn caller() { Self::Assoc::build(); } pub fn build() {} }\n",
    );
    write_file(
        ws,
        "src/shadow_std.rs",
        "pub mod std { pub mod mem { pub fn swap() {} } }\npub fn caller() { std::mem::swap(); }\n",
    );
    write_file(
        ws,
        "src/primitive_qualifier.rs",
        "pub mod u32 { pub fn parse() {} }\npub fn caller() { u32::parse(1); }\n",
    );
    write_file(
        ws,
        "src/local_alias_shadow.rs",
        "pub struct Alias;\nimpl Alias { pub fn build() {} }\npub fn caller() { use ext::Widget as Alias; Alias::build(); }\n",
    );
    write_file(
        ws,
        "src/trait_impl_self.rs",
        "pub trait Builder { fn caller(&self); fn build(&self); }\npub struct Widget;\nimpl Builder for Widget { fn caller(&self) { self.build(); } fn build(&self) {} }\n",
    );

    let q = index(ws).await;
    let edges = canonical_edges(&q).await;
    assert!(
        edges.is_empty(),
        "adversarial precision fixtures must emit no canonical false edges: {edges:?}"
    );
}

#[test]
async fn unproven_external_or_non_default_module_roots_emit_no_edge() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(
        ws,
        "src/lib.rs",
        "#[path = \"actual.rs\"]\npub mod remapped;\npub fn caller() { remapped::target(); tokio::spawn(); }\n",
    );
    write_file(ws, "src/remapped.rs", "pub fn target() {}\n");
    write_file(ws, "src/tokio.rs", "pub fn spawn() {}\n");

    let q = index(ws).await;
    let edges = canonical_edges(&q).await;
    assert!(
        edges.is_empty(),
        "unproven external roots and non-default module mappings must fail closed: {edges:?}"
    );
}

#[test]
async fn cross_file_non_default_module_mapping_emits_no_edge_to_stray_default_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(
        ws,
        "src/lib.rs",
        "#[path = \"actual.rs\"]\npub mod remapped;\npub mod caller;\n",
    );
    write_file(
        ws,
        "src/caller.rs",
        "pub fn caller() { crate::remapped::target(); }\n",
    );
    write_file(ws, "src/actual.rs", "pub fn target() {}\n");
    write_file(ws, "src/remapped.rs", "pub fn target() {}\n");

    let q = index(ws).await;
    let edges = canonical_edges(&q).await;
    assert!(
        edges.is_empty(),
        "cross-file calls through non-default module mappings must not resolve to stray default-layout files: {edges:?}"
    );
}

#[test]
async fn empty_canonical_path_is_never_a_match_target() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_file(
        ws,
        "src/a.rs",
        "pub fn caller() { crate::a::target(); }\npub fn target() {}\n",
    );

    let q = index(ws).await;
    let edges = canonical_edges(&q).await;
    assert!(
        edges.is_empty(),
        "workspace without a manifest has empty canonical_path rows and must not resolve: {edges:?}"
    );
}

#[test]
async fn generic_type_param_head_does_not_resolve_to_shadowed_local_type() {
    // Regression (M2): a generic type parameter `T` shadows a same-named local
    // `struct T`, so `fn caller<T: Builder>() { T::build(); }` must NOT emit a
    // canonical edge to `crate::…::T::build` (the local inherent method). The
    // true callee is `<T as Builder>::build`, which requires type inference the
    // resolver refuses to do — fail closed, never a false edge.
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(ws, "src/lib.rs", "pub mod shadowed;\npub mod plain;\n");
    write_file(
        ws,
        "src/shadowed.rs",
        "pub trait Builder { fn build(); }\npub struct T;\nimpl T { pub fn build() {} }\npub fn caller<T: Builder>() { T::build(); }\n",
    );
    // Control: the SAME bare `Type::method()` shape WITHOUT a shadowing generic
    // still resolves, proving the guard is specific to generic-parameter heads
    // and does not broadly disable in-module local-root resolution.
    write_file(
        ws,
        "src/plain.rs",
        "pub struct Widget;\nimpl Widget { pub fn build() {} }\npub fn caller2() { Widget::build(); }\n",
    );

    let q = index(ws).await;
    let names = name_to_ids(&q).await;
    let edges = canonical_edges(&q).await;

    let shadow_caller = names["caller"][0].clone();
    let local_t_build = names["T::build"][0].clone();
    assert!(
        !edges.contains(&(shadow_caller, local_t_build)),
        "generic param `T` must not resolve to shadowed local `T::build`: {edges:?}"
    );

    let plain_caller = names["caller2"][0].clone();
    let widget_build = names["Widget::build"][0].clone();
    assert!(
        edges.contains(&(plain_caller, widget_build)),
        "non-shadowed in-module `Widget::build()` must still resolve: {edges:?}"
    );
}

#[test]
async fn generic_type_param_named_like_workspace_crate_fails_closed() {
    // Regression (M2 precedence): a generic type parameter whose name COLLIDES
    // with the workspace crate name (`demo`) must fail closed BEFORE the
    // workspace-crate resolution arm. Otherwise `fn caller<demo: Builder>() {
    // demo::build(); }` would resolve `demo::build` to the crate-root `build`
    // via the `is_workspace_crate` fast path, forging a false canonical edge that
    // the generic-parameter guard is supposed to stop.
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws); // crate name = "demo"
    write_file(
        ws,
        "src/lib.rs",
        "pub mod caller;\npub trait Builder { fn build(); }\npub fn build() {}\n",
    );
    write_file(
        ws,
        "src/caller.rs",
        "use crate::Builder;\npub fn caller<demo: Builder>() { demo::build(); }\n",
    );

    let q = index(ws).await;
    let names = name_to_ids(&q).await;
    let edges = canonical_edges(&q).await;

    let caller_id = names["caller"][0].clone();
    assert!(
        edges.iter().all(|(from, _)| from != &caller_id),
        "a generic param named like the crate must not forge any canonical edge: {edges:?}"
    );
}

#[test]
async fn nested_non_default_module_mapping_emits_no_edge_to_stray_default_file() {
    // Regression (M3): a `#[path]` remap NESTED inside an inline module
    // (`mod outer { #[path=…] mod inner; }`) must mark `crate::outer::inner`
    // unsafe just like a top-level remap. Otherwise a stray default-layout file
    // at `src/outer/inner.rs` receives canonical path `crate::outer::inner::*`
    // and a qualified call falsely resolves to it.
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(
        ws,
        "src/lib.rs",
        "pub mod caller;\npub mod outer {\n    #[path = \"nested_actual.rs\"]\n    pub mod inner;\n}\n",
    );
    write_file(
        ws,
        "src/caller.rs",
        "pub fn caller() { crate::outer::inner::target(); }\n",
    );
    // Stray default-layout file for the remapped nested module.
    write_file(ws, "src/outer/inner.rs", "pub fn target() {}\n");

    let q = index(ws).await;
    let names = name_to_ids(&q).await;
    let edges = canonical_edges(&q).await;

    let caller_id = names["caller"][0].clone();
    if let Some(target_ids) = names.get("target") {
        for target_id in target_ids {
            assert!(
                !edges.contains(&(caller_id.clone(), target_id.clone())),
                "nested `#[path]` remap must fail closed, not resolve to stray default file: {edges:?}"
            );
        }
    }
}

#[test]
async fn canonical_edges_are_scored_by_the_target_correctness_gate() {
    // Regression (M4): the target-correctness gate must EVALUATE canonical edges,
    // not only singletons — otherwise a wrong-but-existing canonical edge escapes
    // it. Feed produced canonical edges through the gate: a correct fixture scores
    // zero mismatch, and a manifest expecting a different identity flags a
    // mismatch (proving the gate has teeth for the canonical resolution class).
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_manifest(ws);
    write_file(ws, "src/lib.rs", "pub mod caller;\npub mod util;\n");
    write_file(ws, "src/util.rs", "pub fn helper() {}\n");
    write_file(
        ws,
        "src/caller.rs",
        "pub fn caller() { crate::util::helper(); }\n",
    );

    let q = index(ws).await;
    let names = name_to_ids(&q).await;
    let produced: Vec<(String, String)> = q
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await
        .expect("list canonical edges")
        .into_iter()
        .collect();
    assert!(
        !produced.is_empty(),
        "fixture must produce at least one canonical edge to score"
    );

    let caller_id = names["caller"][0].clone();
    let helper_id = names["helper"][0].clone();
    let correct: HashSet<(String, String)> = [(caller_id.clone(), helper_id)].into_iter().collect();
    let tc = evaluate_target_correctness(&produced, &correct);
    assert_eq!(
        tc.target_mismatch, 0,
        "a correct canonical edge must not be flagged as a mismatch"
    );
    assert!(
        tc.target_correct >= 1,
        "the canonical edge must be scored as target-correct"
    );

    // A manifest expecting a DIFFERENT identity flags the produced canonical edge
    // as a mismatch — the gate is not blind to canonical edges (M4).
    let wrong: HashSet<(String, String)> = [(caller_id, String::from("crate::util::not_helper"))]
        .into_iter()
        .collect();
    let tc_wrong = evaluate_target_correctness(&produced, &wrong);
    assert!(
        tc_wrong.target_mismatch >= 1,
        "a wrong-but-existing canonical edge must be flagged by the gate"
    );
}
