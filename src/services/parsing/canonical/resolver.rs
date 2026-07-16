//! A3 — canonical resolver core (in-crate roots, fail-closed).
//!
//! A pure function from a source path expression to a single workspace-global
//! canonical identity, or `None` (fail-closed). It resolves the deterministic
//! parts of Rust name resolution that require **no type inference**:
//!
//! - `crate::` / `self::` / `super::` roots (against the current module);
//! - a leading workspace-crate name (absolute path);
//! - a single explicit `use` alias (substituted, then the *use-path* re-resolved
//!   with external roots failing closed — kills the `use ext::X as A; A::m()`
//!   false-edge class, 088-F6);
//! - an in-module local item (`Type::method` on a type defined in this module).
//!
//! Everything uncertain is dropped: external-crate roots, glob-only bindings,
//! macro-generated names, `use`/local shadowing, module-shadows-extern-root, and
//! any ambiguity (≥2 bindings). This is the resolver half of the absolute
//! no-false-edge invariant (013-D); the singleton match against
//! `function_meta.canonical_path` in Unit B is the other half.

use super::module_path::{ModulePath, WorkspaceCrates};
use super::use_graph::UseGraph;

/// A resolved workspace-global canonical identity (e.g. `engram::a::Widget::build`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalId(String);

impl CanonicalId {
    /// Wrap a canonical path string.
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    /// The canonical path as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into the owned canonical path string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for CanonicalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The context a path expression is resolved against: the current file's module,
/// the workspace crate set, and the file's use-graph.
#[derive(Debug, Clone, Copy)]
pub struct ResolveContext<'a> {
    /// Module of the file the call appears in.
    pub module: &'a ModulePath,
    /// Workspace crate-name set (workspace-vs-external classification).
    pub crates: &'a WorkspaceCrates,
    /// The file's use-graph (aliases + glob markers).
    pub use_graph: &'a UseGraph,
}

/// Crate roots that always denote external code and therefore fail closed unless
/// a workspace module legitimately re-roots them via `crate::`/`self::`/`super::`.
const KNOWN_EXTERN_ROOTS: &[&str] = &["std", "core", "alloc", "proc_macro"];

/// Maximum alias-substitution depth before failing closed (defends against
/// pathological/cyclic aliasing).
const MAX_RESOLVE_DEPTH: u32 = 16;

/// Resolve a source path expression to a single canonical identity, or `None`
/// (fail-closed).
#[must_use]
pub fn resolve_path(ctx: &ResolveContext, path_expr: &str) -> Option<CanonicalId> {
    let expr = path_expr.trim();
    if expr.is_empty() {
        return None;
    }
    let segs: Vec<&str> = expr.split("::").collect();
    let resolved = resolve_core(ctx, &segs, true, 0)?;
    if resolved.is_empty() {
        return None;
    }
    Some(CanonicalId(resolved.join("::")))
}

/// Resolve `segs` to canonical segments (`[crate_name, seg, …]`), or `None`.
///
/// `in_module_ok` distinguishes a **source** expression (a bare, unimported,
/// unqualified head may be a local item of the current module) from a **use
/// path** substituted during alias resolution (a bare non-workspace head is an
/// external crate root and fails closed — D3).
fn resolve_core(
    ctx: &ResolveContext,
    segs: &[&str],
    in_module_ok: bool,
    depth: u32,
) -> Option<Vec<String>> {
    if depth > MAX_RESOLVE_DEPTH {
        return None;
    }
    let head = *segs.first()?;
    let tail = &segs[1..];
    match head {
        "crate" => Some(with_tail(vec![ctx.module.crate_name.clone()], tail)),
        "self" => Some(with_tail(module_segments(ctx.module), tail)),
        "super" => resolve_super(ctx, segs),
        "" => resolve_absolute(ctx, tail),
        h if ctx.crates.is_workspace_crate(h) => {
            Some(segs.iter().map(|s| (*s).to_owned()).collect())
        }
        h => {
            let bindings: Vec<_> = ctx.use_graph.bindings_for(h).collect();
            if bindings.len() > 1 {
                return None; // ambiguous alias
            }
            if let Some(binding) = bindings.first() {
                // Explicit `use` wins (D6). Re-resolve the use-path with
                // in_module_ok=false so an external root fails closed (D3/F6).
                let use_segs: Vec<&str> = binding.path.split("::").collect();
                let base = resolve_core(ctx, &use_segs, false, depth + 1)?;
                return Some(with_tail(base, tail));
            }
            if KNOWN_EXTERN_ROOTS.contains(&h) {
                return None; // D7: std/core/alloc root, not re-rooted
            }
            if ctx.use_graph.has_glob() {
                return None; // D6: could originate from a glob import
            }
            if !in_module_ok {
                return None; // external crate root in a use-path
            }
            // In-module local item (`Type::method` on a type defined here).
            let mut out = module_segments(ctx.module);
            out.push(h.to_owned());
            Some(with_tail(out, tail))
        }
    }
}

/// Resolve a `super::…` (possibly repeated) path against the current module.
fn resolve_super(ctx: &ResolveContext, segs: &[&str]) -> Option<Vec<String>> {
    let mut supers = 0usize;
    while segs.get(supers).is_some_and(|s| *s == "super") {
        supers += 1;
    }
    let mut module = ctx.module.clone();
    for _ in 0..supers {
        module = module.parent()?; // fail-closed above crate root
    }
    Some(with_tail(module_segments(&module), &segs[supers..]))
}

/// Resolve a leading-`::` absolute path: the first real segment must be a
/// workspace crate, else external → fail-closed.
fn resolve_absolute(ctx: &ResolveContext, tail: &[&str]) -> Option<Vec<String>> {
    let head = *tail.first()?;
    if ctx.crates.is_workspace_crate(head) {
        Some(tail.iter().map(|s| (*s).to_owned()).collect())
    } else {
        None
    }
}

fn module_segments(module: &ModulePath) -> Vec<String> {
    let mut out = Vec::with_capacity(module.segments.len() + 1);
    out.push(module.crate_name.clone());
    out.extend(module.segments.iter().cloned());
    out
}

fn with_tail(mut base: Vec<String>, tail: &[&str]) -> Vec<String> {
    base.extend(tail.iter().map(|s| (*s).to_owned()));
    base
}

#[cfg(test)]
mod tests {
    use super::super::module_path::CrateRoot;
    use super::super::use_graph::{UseBinding, UseGraph, extract_use_graph};
    use super::*;

    fn crates() -> WorkspaceCrates {
        WorkspaceCrates::new(vec![
            CrateRoot {
                name: "engram".to_owned(),
                dir: String::new(),
            },
            CrateRoot {
                name: "powerbi_tmdl_parser".to_owned(),
                dir: "crates/powerbi-tmdl-parser".to_owned(),
            },
        ])
    }

    fn module(segments: &[&str]) -> ModulePath {
        ModulePath {
            crate_name: "engram".to_owned(),
            segments: segments.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    fn resolve(module_segs: &[&str], uses: &str, expr: &str) -> Option<String> {
        let crates = crates();
        let m = module(module_segs);
        let ug = extract_use_graph(uses);
        let ctx = ResolveContext {
            module: &m,
            crates: &crates,
            use_graph: &ug,
        };
        resolve_path(&ctx, expr).map(CanonicalId::into_string)
    }

    #[test]
    fn crate_root_resolves_to_crate_name() {
        assert_eq!(
            resolve(&["a"], "", "crate::b::C").as_deref(),
            Some("engram::b::C")
        );
    }

    #[test]
    fn self_root_resolves_to_current_module() {
        assert_eq!(
            resolve(&["a"], "", "self::b::C").as_deref(),
            Some("engram::a::b::C")
        );
    }

    #[test]
    fn super_root_resolves_to_parent_module() {
        assert_eq!(
            resolve(&["a", "b"], "", "super::x::Y").as_deref(),
            Some("engram::a::x::Y")
        );
    }

    #[test]
    fn repeated_super_walks_up() {
        assert_eq!(
            resolve(&["a", "b"], "", "super::super::Z").as_deref(),
            Some("engram::Z")
        );
    }

    #[test]
    fn super_past_crate_root_fails_closed() {
        assert_eq!(resolve(&["a"], "", "super::super::Z"), None);
    }

    #[test]
    fn workspace_crate_root_is_absolute() {
        assert_eq!(
            resolve(&["a"], "", "powerbi_tmdl_parser::tmdl::Lexer").as_deref(),
            Some("powerbi_tmdl_parser::tmdl::Lexer")
        );
    }

    #[test]
    fn own_crate_name_root_is_absolute() {
        assert_eq!(
            resolve(&["z"], "", "engram::a::B").as_deref(),
            Some("engram::a::B")
        );
    }

    #[test]
    fn explicit_alias_substitutes_and_reresolves() {
        assert_eq!(
            resolve(&["z"], "use crate::a::Widget;", "Widget::build").as_deref(),
            Some("engram::a::Widget::build")
        );
    }

    #[test]
    fn alias_to_external_crate_fails_closed() {
        // 088-F6: `use ext::Widget as Alias; Alias::build()` must be 0 edges.
        assert_eq!(
            resolve(&["z"], "use ext::Widget as Alias;", "Alias::build"),
            None
        );
    }

    #[test]
    fn known_extern_root_fails_closed() {
        // 088-F1 family: a std/core qualifier never resolves to a workspace def.
        assert_eq!(resolve(&["a"], "", "std::mem::swap"), None);
        assert_eq!(resolve(&["a"], "use std::mem;", "mem::swap"), None);
    }

    #[test]
    fn glob_shadowing_fails_closed() {
        // D6: a name that could originate from a glob import cannot be proven local.
        assert_eq!(resolve(&["a"], "use other::*;", "Widget::build"), None);
    }

    #[test]
    fn in_module_local_type_resolves() {
        assert_eq!(
            resolve(&["a"], "", "Widget::build").as_deref(),
            Some("engram::a::Widget::build")
        );
    }

    #[test]
    fn leading_colon_absolute_workspace_resolves() {
        assert_eq!(
            resolve(&["a"], "", "::engram::a::B").as_deref(),
            Some("engram::a::B")
        );
    }

    #[test]
    fn leading_colon_absolute_external_fails_closed() {
        assert_eq!(resolve(&["a"], "", "::tokio::spawn"), None);
    }

    #[test]
    fn ambiguous_two_bindings_fails_closed() {
        let crates = crates();
        let m = module(&["a"]);
        let ug = UseGraph {
            bindings: vec![
                UseBinding {
                    alias: "Widget".to_owned(),
                    path: "crate::a::Widget".to_owned(),
                    is_reexport: false,
                },
                UseBinding {
                    alias: "Widget".to_owned(),
                    path: "crate::b::Widget".to_owned(),
                    is_reexport: false,
                },
            ],
            globs: vec![],
        };
        let ctx = ResolveContext {
            module: &m,
            crates: &crates,
            use_graph: &ug,
        };
        assert_eq!(resolve_path(&ctx, "Widget::build"), None);
    }

    #[test]
    fn empty_expr_fails_closed() {
        assert_eq!(resolve(&["a"], "", ""), None);
        assert_eq!(resolve(&["a"], "", "   "), None);
    }
}
