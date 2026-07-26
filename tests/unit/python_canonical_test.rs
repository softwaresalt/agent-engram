//! Unit tests for Python canonical module-path derivation (feature 096-F, T1-setup).
//!
//! External harness exercising the public
//! [`engram::services::parsing::python_canonical::python_module_path_for_file`]
//! over the fail-closed acceptance table. Registration owned by 096.012-T; the
//! algorithm under test is owned by 096.001-T (T1) and consumed here.

use std::collections::HashSet;
use std::path::Path;

use engram::services::parsing::python_canonical::extract_python_import_bindings;
use engram::services::parsing::python_canonical::{BindingKind, python_module_path_for_file};

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
