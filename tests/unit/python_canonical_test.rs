//! Unit tests for Python canonical module-path derivation (feature 096-F, T1-setup).
//!
//! External harness exercising the public
//! [`engram::services::parsing::python_canonical::python_module_path_for_file`]
//! over the fail-closed acceptance table. Registration owned by 096.012-T; the
//! algorithm under test is owned by 096.001-T (T1) and consumed here.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use engram::db::connect_db;
use engram::db::queries::{CodeGraphQueries, NoCanonicalTargetReason};
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;
use engram::services::parsing::python_canonical::extract_python_import_bindings;
use engram::services::parsing::python_canonical::{
    BindingKind, CallResolution, python_module_path_for_file,
};

/// Build a regular-package predicate from a set of `/`-joined directory paths
/// known to contain an `__init__.py` (mirrors T3's derived predicate contract).
fn packages(dirs: &[&str]) -> impl Fn(&Path) -> bool {
    let set: HashSet<String> = dirs.iter().map(|d| (*d).to_owned()).collect();
    move |p: &Path| set.contains(&p.to_string_lossy().replace('\\', "/"))
}

#[test]
fn resolves_nested_regular_package_chain() {
    let pred = packages(&["p", "p/q"]);
    assert_eq!(
        python_module_path_for_file("p/q/r.py", &pred),
        Some("p.q.r".to_owned())
    );
}

#[test]
fn resolves_top_level_module() {
    let pred = packages(&[]);
    assert_eq!(
        python_module_path_for_file("mod.py", &pred),
        Some("mod".to_owned())
    );
}

#[test]
fn fails_closed_on_init_marker() {
    let pred = packages(&["p", "p/q"]);
    assert_eq!(python_module_path_for_file("p/__init__.py", &pred), None);
}

#[test]
fn fails_closed_on_src_root_without_init() {
    // `src/` lacks __init__.py, so ancestor `src` is not a regular package.
    let pred = packages(&["src/pkg"]);
    assert_eq!(python_module_path_for_file("src/pkg/mod.py", &pred), None);
}

#[test]
fn fails_closed_on_pep420_namespace_package() {
    // Implicit namespace package: `ns` has no __init__.py.
    let pred = packages(&[]);
    assert_eq!(python_module_path_for_file("ns/mod.py", &pred), None);
}

#[test]
fn fails_closed_on_non_identifier_segment() {
    let pred = packages(&["foo-bar"]);
    assert_eq!(python_module_path_for_file("foo-bar/mod.py", &pred), None);
}

#[test]
fn fails_closed_on_non_python_path() {
    let pred = packages(&["p"]);
    assert_eq!(python_module_path_for_file("p/notes.txt", &pred), None);
}

// ---------------------------------------------------------------------------
// T2 — Python import-binding capture (096.002-T)
// ---------------------------------------------------------------------------

#[test]
fn t2_captures_kind_and_position() {
    let src = "from p import f\nimport a.b as c\n";
    let b = extract_python_import_bindings(src);

    let f = b
        .module_binding("f")
        .expect("f should be a firm from-import");
    assert_eq!(f.canonical_path, "p.f");
    assert_eq!(f.kind, BindingKind::FromImportSymbol);
    assert_eq!(f.position, 0);

    let c = b
        .module_binding("c")
        .expect("c should be a firm module import");
    assert_eq!(c.canonical_path, "a.b");
    assert_eq!(c.kind, BindingKind::ModuleImport);
    assert_eq!(c.position, "from p import f\n".len());
}

#[test]
fn t2_relative_marks_and_star_invalidates() {
    let rel = extract_python_import_bindings("from . import x\n");
    assert!(rel.module_binding("x").is_none(), "relative binds nothing");
    assert!(
        rel.is_ambiguous("x"),
        "relative import is an ambiguity marker (T-b)"
    );

    let star = extract_python_import_bindings("from p import *\n");
    assert!(star.module_binding("p").is_none(), "star mints no binding");
    assert_eq!(
        star.star_invalidators(),
        &[0],
        "module-scope star is a positioned invalidator (F4)"
    );
}

#[test]
fn t2_star_invalidator_is_order_aware() {
    let after = extract_python_import_bindings("from bar import parse\nfrom n import *\n");
    let parse = after.module_binding("parse").expect("parse is firm");
    assert_eq!(parse.position, 0);
    let star_after = after.star_invalidators()[0];
    assert!(
        star_after > parse.position,
        "star after winner is recorded later"
    );

    let before = extract_python_import_bindings("from n import *\nfrom bar import parse\n");
    let parse2 = before.module_binding("parse").expect("parse is firm");
    let star_before = before.star_invalidators()[0];
    assert!(
        star_before < parse2.position,
        "star before winner is recorded earlier"
    );
}

#[test]
fn t2_competing_and_conditional_fail_closed() {
    let dup = extract_python_import_bindings("import p\nfrom q import p\n");
    assert!(
        dup.module_binding("p").is_none(),
        "competing binding fails closed (M1)"
    );
    assert!(
        dup.is_ambiguous("p"),
        "competing binding is an ambiguity marker"
    );

    let cond = extract_python_import_bindings("if enabled:\n    from a import g\n");
    assert!(
        cond.module_binding("g").is_none(),
        "conditional import fails closed (T-c)"
    );
    assert!(
        cond.is_ambiguous("g"),
        "conditional import is an ambiguity marker"
    );
}

// ---------------------------------------------------------------------------
// T2b — Scope-aware binding isolation (096.009-T)
// ---------------------------------------------------------------------------

/// Byte offsets of every `f()` call-expression occurrence, in source order.
fn call_offsets(src: &str, call: &str) -> Vec<usize> {
    src.match_indices(call).map(|(i, _)| i).collect()
}

#[test]
fn t2b_function_local_import_is_order_aware() {
    // A function-local `from x import f` is visible for a call AFTER it, but a
    // call BEFORE the local import is a poison/tombstone that does NOT fall
    // through to the module-level `f` (Y1/F8 — UnboundLocalError semantics).
    let src = "from mod import f\n\ndef after():\n    from x import f\n    f()\n\ndef before():\n    f()\n    from x import f\n";
    let b = extract_python_import_bindings(src);

    let calls = call_offsets(src, "f()");
    let (after_call, before_call) = (calls[0], calls[1]);

    match b.resolve_call(after_call, "f") {
        CallResolution::LocalImport(binding) => {
            assert_eq!(binding.canonical_path, "x.f");
            assert_eq!(binding.kind, BindingKind::FromImportSymbol);
        }
        other => panic!("expected LocalImport(x.f) for post-import call, got {other:?}"),
    }

    assert_eq!(
        b.resolve_call(before_call, "f"),
        CallResolution::Poisoned,
        "call before the function-local import fails closed (no module fall-through)"
    );
}

#[test]
fn t2b_function_local_import_does_not_leak_to_sibling() {
    // The SAME call site in a sibling function that lacks the import gets no
    // local binding — it defers to module scope (no leak, M1).
    let src = "def owner():\n    from x import f\n    f()\n\ndef user():\n    f()\n";
    let b = extract_python_import_bindings(src);

    let user_call = call_offsets(src, "f()")[1];
    assert_eq!(
        b.resolve_call(user_call, "f"),
        CallResolution::ModuleScope,
        "sibling without the import defers to module scope"
    );
    assert!(
        b.module_binding("f").is_none(),
        "module scope binds no f, so nothing is minted"
    );
}

#[test]
fn t2b_nested_isolation_and_global_redirect() {
    // (a) A nested/closure function's local import does NOT leak UP into its
    //     enclosing top-level caller's calls (F5).
    let nested = "def outer():\n    def inner():\n        from x import f\n    f()\n";
    let nb = extract_python_import_bindings(nested);
    let outer_call = call_offsets(nested, "f()")[0];
    assert_eq!(
        nb.resolve_call(outer_call, "f"),
        CallResolution::ModuleScope,
        "enclosing caller resolves against module scope, not the nested local"
    );

    // (b) A `global`-declared name in a top-level caller redirects to module
    //     scope (resolves against the module binding, not a local).
    let global_src = "import g\n\ndef caller():\n    global g\n    g()\n";
    let gb = extract_python_import_bindings(global_src);
    let gcall = global_src.find("g()").expect("g() present");
    assert_eq!(
        gb.resolve_call(gcall, "g"),
        CallResolution::ModuleScope,
        "global-declared name redirects to module scope"
    );
    assert!(
        gb.module_binding("g").is_some(),
        "module g remains resolvable at module scope"
    );
}

#[test]
fn t2b_dynamic_global_rebind_fails_closed() {
    // A `global f; from b import f` write rebinds module `f`, making sibling
    // callers of `f` run-order-dependent -> fail closed for ALL callers (T-d).
    let src = "from a import f\n\ndef mutate():\n    global f\n    from b import f\n\ndef caller():\n    f()\n";
    let b = extract_python_import_bindings(src);

    assert!(
        b.is_dynamically_rebound("f"),
        "a global rebind of f poisons the name module-wide"
    );
    let call = src.find("f()").expect("f() present");
    assert_eq!(
        b.resolve_call(call, "f"),
        CallResolution::Poisoned,
        "sibling caller of a dynamically rebound name fails closed"
    );
}

#[test]
fn t2b_branchy_function_local_import_fails_closed() {
    // A function-local import guarded by control flow is not proven to execute
    // in order -> fail closed (poison), even for a call textually after it (Y1).
    let src = "def caller():\n    if flag:\n        from x import f\n    f()\n";
    let b = extract_python_import_bindings(src);

    let call = src.find("f()").expect("f() present");
    assert_eq!(
        b.resolve_call(call, "f"),
        CallResolution::Poisoned,
        "conditional function-local import fails closed"
    );
}

// ---------------------------------------------------------------------------
// T5b-seam — public language-scoped name->IDs helper + typed no-target policy
// (096.011-T)
// ---------------------------------------------------------------------------

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

// F9: the helper is language-scoped and honest — it returns the FULL candidate
// ID set for a name (unique -> 1, ambiguous -> N, absent -> 0). The singleton
// decision belongs to the consumer (T5b), not this seam. A same-named Rust
// definition must never leak into a Python lookup (013-D language scoping).
#[tokio::test]
async fn t5b_seam_function_ids_by_name_is_language_scoped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "a.py", "def solo():\n    pass\n");
    write_sample_file(ws, "b.py", "def dup():\n    pass\n");
    write_sample_file(ws, "c.py", "def dup():\n    pass\n");
    // A same-named Rust definition proves the language filter excludes it.
    write_sample_file(ws, "r.rs", "pub fn solo() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");
    let q = queries_for(&data_dir, &branch).await;

    // Unique Python name -> exactly one id (the Rust `solo` is excluded).
    let solo = q
        .function_ids_by_name("solo", "python")
        .await
        .expect("function_ids_by_name solo");
    assert_eq!(solo.len(), 1, "unique python name yields one id, {solo:?}");

    // Ambiguous Python name -> the full candidate set (both defs).
    let dup = q
        .function_ids_by_name("dup", "python")
        .await
        .expect("function_ids_by_name dup");
    assert_eq!(
        dup.len(),
        2,
        "ambiguous python name yields both ids, {dup:?}"
    );

    // Zero match -> empty set.
    let absent = q
        .function_ids_by_name("absent", "python")
        .await
        .expect("function_ids_by_name absent");
    assert!(absent.is_empty(), "absent name yields no ids, {absent:?}");

    // The Rust `solo` is reachable only under its own language scope.
    let rust_solo = q
        .function_ids_by_name("solo", "rust")
        .await
        .expect("function_ids_by_name rust solo");
    assert_eq!(
        rust_solo.len(),
        1,
        "rust scope sees its own solo, {rust_solo:?}"
    );
    assert_ne!(
        solo, rust_solo,
        "python and rust `solo` are distinct definitions"
    );
}

// Anchor B: the legacy name-only unique-match fallback fires ONLY for the two
// recall-safe no-target reasons; the three ambiguity reasons emit NO edge.
#[test]
fn t5b_seam_fallback_policy_allows_only_recall_safe_reasons() {
    assert!(
        NoCanonicalTargetReason::NoModuleContext.allows_name_only_fallback(),
        "no module context is recall-safe"
    );
    assert!(
        NoCanonicalTargetReason::UnsupportedImportForm.allows_name_only_fallback(),
        "unsupported import form is recall-safe"
    );
    assert!(
        !NoCanonicalTargetReason::CompetingBindings.allows_name_only_fallback(),
        "competing bindings must fail closed"
    );
    assert!(
        !NoCanonicalTargetReason::Shadowed.allows_name_only_fallback(),
        "shadowed binding must fail closed"
    );
    assert!(
        !NoCanonicalTargetReason::DuplicateSameNameImport.allows_name_only_fallback(),
        "duplicate same-name import must fail closed"
    );
}
