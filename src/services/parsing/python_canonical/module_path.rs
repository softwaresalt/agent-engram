//! T1 — Python module-path derivation (feature 096-F, Unit T1).
//!
//! Derives the **module namespace** for a Python source file from its
//! workspace-relative path (`foo/bar.py` → `foo.bar`), but **only** when every
//! ancestor directory is a *provable regular package* (contains an
//! `__init__.py`). The regular-package predicate is supplied by the caller,
//! derived from the already-indexed file set (no configuration source, Q3).
//!
//! Deterministic and **fail-closed** (013-D): any layout that cannot be mapped
//! to a provable dotted namespace yields `None` rather than a guess. Fail-closed
//! cases include `__init__.py` itself, a non-identifier path segment, any
//! ancestor directory lacking `__init__.py` (which conservatively rejects both
//! implicit PEP 420 namespace packages and `src/`-style source-root layouts),
//! and any non-`.py` path.
//!
//! **v1 NON-GOAL (Q3/Q6):** source-root-aware resolution (stripping a declared
//! `src/` root to yield `pkg.mod`) is explicitly out of scope. There is no
//! production source for source-root declarations, so rather than invent
//! speculative config (Constitution VI) v1 fails closed on `src/`-layouts and
//! namespace packages.

use std::path::Path;

/// Resolve a workspace-relative `.py` path to its dotted module namespace.
///
/// Returns `Some("foo.bar")` for `foo/bar.py` **only** when every ancestor
/// directory (`foo`) is a provable regular package per `is_regular_package`; a
/// top-level module (`mod.py`, no ancestor dirs) resolves to `Some("mod")`.
/// Returns `None` (fail closed) for `__init__.py`, a non-identifier segment, an
/// ancestor directory that is not a regular package (PEP 420 namespace or
/// `src/`-style layout), or a non-`.py` path.
///
/// `is_regular_package` receives an ancestor **directory** path built from the
/// leading path segments joined with `/` (workspace-relative, forward slashes),
/// e.g. `foo` then `foo/bar` for `foo/bar/baz.py`.
#[must_use]
pub fn python_module_path_for_file(
    rel_path: &str,
    is_regular_package: &impl Fn(&Path) -> bool,
) -> Option<String> {
    let rel = rel_path.replace('\\', "/");
    let stem = rel.strip_suffix(".py")?;
    if stem.is_empty() {
        return None;
    }
    let segments: Vec<&str> = stem.split('/').collect();
    // The final segment is the module file's stem; the rest are ancestor dirs.
    let (file_stem, ancestor_dirs) = segments.split_last()?;
    // `__init__.py` is the package marker, never a resolvable module.
    if *file_stem == "__init__" {
        return None;
    }
    // Every path segment (ancestor dir names + the file stem) must be a valid
    // Python identifier; otherwise the layout is not a provable namespace.
    if !segments.iter().all(|s| is_python_identifier(s)) {
        return None;
    }
    // Every ancestor directory must be a provable regular package.
    let mut dir = String::new();
    for d in ancestor_dirs {
        if !dir.is_empty() {
            dir.push('/');
        }
        dir.push_str(d);
        if !is_regular_package(Path::new(&dir)) {
            return None;
        }
    }
    Some(segments.join("."))
}

/// Whether `seg` is a valid Python identifier (ASCII/Unicode letter or `_`
/// start, alphanumeric or `_` thereafter). Empty segments are rejected.
fn is_python_identifier(seg: &str) -> bool {
    let mut chars = seg.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c.is_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Path;

    /// Build a regular-package predicate from a set of `/`-joined directory
    /// paths known to contain an `__init__.py`.
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
    fn fails_closed_table() {
        // __init__.py itself never resolves.
        let all = packages(&["p", "p/q", "src", "ns"]);
        assert_eq!(python_module_path_for_file("p/__init__.py", &all), None);
        // A `src/`-root where `src/` lacks __init__.py: only `src/pkg` is a
        // package, so ancestor `src` fails closed.
        let src_pkg = packages(&["src/pkg"]);
        assert_eq!(
            python_module_path_for_file("src/pkg/mod.py", &src_pkg),
            None
        );
        // Implicit PEP 420 namespace package: `ns` has no __init__.py.
        let none = packages(&[]);
        assert_eq!(python_module_path_for_file("ns/mod.py", &none), None);
        // Non-identifier directory segment.
        let dashed = packages(&["foo-bar"]);
        assert_eq!(python_module_path_for_file("foo-bar/mod.py", &dashed), None);
    }

    #[test]
    fn fails_closed_non_python_path() {
        let pred = packages(&["p"]);
        assert_eq!(python_module_path_for_file("p/data.json", &pred), None);
    }
}
