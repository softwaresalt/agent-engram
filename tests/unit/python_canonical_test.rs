//! Unit tests for Python canonical module-path derivation (feature 096-F, T1-setup).
//!
//! External harness exercising the public
//! [`engram::services::parsing::python_canonical::python_module_path_for_file`]
//! over the fail-closed acceptance table. Registration owned by 096.012-T; the
//! algorithm under test is owned by 096.001-T (T1) and consumed here.

use std::collections::HashSet;
use std::path::Path;

use engram::services::parsing::python_canonical::python_module_path_for_file;

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
