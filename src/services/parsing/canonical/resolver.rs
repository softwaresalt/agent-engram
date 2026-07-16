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
//! - a proven in-module local item (`Type::method` on a type defined in this
//!   module).
//!
//! Everything uncertain is dropped: external-crate roots, glob-only bindings,
//! macro-generated names, unproven bare roots, `use`/local shadowing,
//! module-shadows-extern-root, and any ambiguity (≥2 bindings). This is the
//! resolver half of the absolute no-false-edge invariant (013-D); the singleton
//! match against `function_meta.canonical_path` in Unit B is the other half.

use super::generics::normalize_generics;
use super::module_path::{ModulePath, WorkspaceCrates, is_module_ident};
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

/// A call-site qualifier as understood by the resolver.
///
/// The `Self` type is represented by the **typed** [`Qualifier::SelfType`]
/// sentinel — an enum variant a source string cannot reproduce. The parser sets
/// it only when it observes the `Self` keyword at a scoped-path root inside an
/// impl (`Self` is a reserved keyword, so it can never be a user identifier).
/// A source qualifier written as the string `Self` is therefore *not* the
/// marker: [`resolve_path`] fails closed on a bare `Self` root, so
/// `Self::Assoc::method()` cannot forge the enclosing-type substitution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Qualifier {
    /// A source-written path qualifier (module or type path), e.g.
    /// `crate::a::Widget` or `module`.
    Path(String),
    /// The enclosing impl's `Self` type — an unforgeable typed sentinel.
    SelfType,
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

/// Primitive type roots are language built-ins, not workspace modules.
const PRIMITIVE_TYPE_ROOTS: &[&str] = &[
    "bool", "char", "str", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64",
    "i128", "isize", "f32", "f64",
];

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
    // Fail closed on any non-identifier segment: a mangled receiver spelling
    // (`<T as Trait>`, or a generic-leak such as `Widget C`) must never become a
    // canonical match target (D4 / I2).
    if resolved.is_empty() || resolved.iter().any(|s| !is_module_ident(s)) {
        return None;
    }
    Some(CanonicalId(resolved.join("::")))
}

/// Resolve a staged call qualifier + trailing segments to a canonical identity,
/// or `None` (fail-closed).
///
/// - [`Qualifier::SelfType`] substitutes the enclosing impl's canonical type
///   (`enclosing_self`), then appends a **single** method segment. A missing
///   enclosing type (`Self::` outside an impl) or an intermediate segment
///   (`Self::Assoc::method` — an associated-type projection) fails closed.
/// - [`Qualifier::Path`] delegates to [`resolve_path`] over the joined path.
#[must_use]
pub fn resolve_qualifier(
    ctx: &ResolveContext,
    enclosing_self: Option<&str>,
    qualifier: &Qualifier,
    tail: &[&str],
) -> Option<CanonicalId> {
    match qualifier {
        Qualifier::SelfType => match tail {
            [method] if !method.is_empty() => {
                let enclosing = enclosing_self?;
                Some(CanonicalId(format!("{enclosing}::{method}")))
            }
            // Bare `Self`, or `Self::Assoc::method` projection → fail-closed.
            _ => None,
        },
        Qualifier::Path(path) => {
            if tail.is_empty() {
                resolve_path(ctx, path)
            } else {
                resolve_path(ctx, &format!("{path}::{}", tail.join("::")))
            }
        }
    }
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
        // A bare `Self` string is never resolved as a path: the enclosing type is
        // substituted only via the typed `Qualifier::SelfType` marker (A7). This
        // makes `Self::Assoc::method()` unforgeable — it cannot masquerade as the
        // Self sentinel.
        "Self" => None,
        "" => resolve_absolute(ctx, tail),
        // A bare head naming an in-file generic type parameter
        // (`fn f<T: Bound>() { T::m() }`) cannot be resolved without type
        // inference and could shadow a same-named local type OR a workspace
        // crate whose name collides with the parameter — fail closed BEFORE the
        // workspace-crate arm so a generic param named like a crate cannot forge
        // a canonical edge (M2, no-false-edge invariant).
        h if ctx.use_graph.is_generic_param(h) => None,
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
            if KNOWN_EXTERN_ROOTS.contains(&h) || PRIMITIVE_TYPE_ROOTS.contains(&h) {
                return None; // D7: std/core/alloc root, not re-rooted
            }
            if ctx.use_graph.has_glob() {
                return None; // D6: could originate from a glob import
            }
            if !in_module_ok {
                return None; // external crate root in a use-path
            }
            if !ctx.use_graph.has_local_root(h) {
                return None; // unproven bare root; could be an external crate
            }
            // Proven in-module local item (`Type::method` on a type defined here).
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

/// Compute the canonical identity of a **definition** whose parser-assigned
/// `def_name` is either a free-function name (`helper`) or an impl-method name
/// (`Type::method`, as produced by `extract_impl`).
///
/// Free functions canonicalise to `<module>::name`. Impl methods canonicalise
/// their receiver type via the resolver (so `impl Widget`, `impl crate::b::Widget`,
/// and same-named cross-module types get **distinct** identities — the RMeJ0
/// regression) and append the method name. Generic arguments are normalised
/// first (A5). Returns `None` (→ empty `canonical_path`, never a match target,
/// D4) when the receiver type cannot be resolved.
#[must_use]
pub fn canonical_path_for_def(ctx: &ResolveContext, def_name: &str) -> Option<CanonicalId> {
    // Normalise generics first; an unbalanced spelling fails closed (I1 / D4).
    let norm = normalize_generics(def_name.trim())?;
    if norm.is_empty() {
        return None;
    }
    let Some((type_path, method)) = norm.rsplit_once("::") else {
        // Free function / top-level item defined in this module. A non-identifier
        // name (a generic leak that survived normalisation) fails closed (I2).
        if !is_module_ident(&norm) {
            return None;
        }
        let mut segs = module_segments(ctx.module);
        segs.push(norm);
        return Some(CanonicalId(segs.join("::")));
    };
    // The method segment must be a plain identifier; the receiver type is
    // validated by `resolve_path` (I2 / D4).
    if type_path.is_empty() || method.is_empty() || !is_module_ident(method) {
        return None;
    }
    let ty = resolve_path(ctx, type_path)?;
    Some(CanonicalId(format!("{}::{method}", ty.as_str())))
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
            resolve(&["a"], "struct Widget;", "Widget::build").as_deref(),
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
            local_roots: vec![],
            has_nested_use: false,
            has_non_default_mod_mapping: false,
            non_default_mod_roots: vec![],
            generic_type_params: vec![],
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

    fn def_canon(module_segs: &[&str], uses: &str, def_name: &str) -> Option<String> {
        let crates = crates();
        let m = module(module_segs);
        let ug = extract_use_graph(uses);
        let ctx = ResolveContext {
            module: &m,
            crates: &crates,
            use_graph: &ug,
        };
        canonical_path_for_def(&ctx, def_name).map(CanonicalId::into_string)
    }

    #[test]
    fn def_free_function_uses_module_path() {
        assert_eq!(
            def_canon(&["a"], "", "helper").as_deref(),
            Some("engram::a::helper")
        );
    }

    #[test]
    fn def_impl_method_in_module() {
        assert_eq!(
            def_canon(&["a"], "struct Widget;", "Widget::build").as_deref(),
            Some("engram::a::Widget::build")
        );
    }

    #[test]
    fn def_impl_method_explicit_crate_path() {
        assert_eq!(
            def_canon(&["x"], "", "crate::b::Widget::build").as_deref(),
            Some("engram::b::Widget::build")
        );
    }

    #[test]
    fn def_rmej0_distinct_cross_module_same_name() {
        // The RMeJ0 regression: same source spelling `Widget::m` in different
        // modules must produce distinct canonical identities.
        let a = def_canon(&["a"], "struct Widget;", "Widget::m");
        let b = def_canon(&["b"], "struct Widget;", "Widget::m");
        assert_eq!(a.as_deref(), Some("engram::a::Widget::m"));
        assert_eq!(b.as_deref(), Some("engram::b::Widget::m"));
        assert_ne!(a, b);
        // An impl on an *imported* Widget canonicalises to the type's real home,
        // matching module a's Widget (not module x where the impl is written).
        let imported = def_canon(&["x"], "use crate::a::Widget;", "Widget::m");
        assert_eq!(imported, a);
    }

    #[test]
    fn def_generic_impl_normalises() {
        assert_eq!(
            def_canon(&["a"], "struct Widget<T>;", "Widget<T>::build").as_deref(),
            Some("engram::a::Widget::build")
        );
        // Generic argument containing a path must not confuse the type/method split.
        assert_eq!(
            def_canon(&["a"], "struct Foo<T>;", "Foo<a::B>::build").as_deref(),
            Some("engram::a::Foo::build")
        );
    }

    #[test]
    fn def_impl_on_external_type_fails_closed() {
        // D4: an impl on an externally-aliased type yields no canonical identity.
        assert_eq!(def_canon(&["a"], "use ext::Widget;", "Widget::m"), None);
    }

    #[test]
    fn def_empty_name_fails_closed() {
        assert_eq!(def_canon(&["a"], "", ""), None);
    }

    #[test]
    fn def_return_arrow_generic_fails_closed() {
        // I1: `->` inside `<…>` closes the group early and the trailing text leaks
        // (`Widget C::method`); the unbalanced closer must drop the identity, never
        // store a garbage canonical path.
        assert_eq!(def_canon(&["a"], "", "Widget<Fn() -> C>::method"), None);
    }

    #[test]
    fn resolve_non_identifier_head_fails_closed() {
        // I2: a mangled receiver spelling must not resolve to a canonical target.
        assert_eq!(resolve(&["a"], "", "<T as Trait>::method"), None);
        assert_eq!(resolve(&["a"], "", "Widget C::method"), None);
    }

    fn self_ctx<'a>(
        crates: &'a WorkspaceCrates,
        m: &'a ModulePath,
        ug: &'a UseGraph,
    ) -> ResolveContext<'a> {
        ResolveContext {
            module: m,
            crates,
            use_graph: ug,
        }
    }

    #[test]
    fn self_marker_resolves_to_enclosing_type() {
        let crates = crates();
        let m = module(&["a"]);
        let ug = extract_use_graph("");
        let ctx = self_ctx(&crates, &m, &ug);
        let got = resolve_qualifier(
            &ctx,
            Some("engram::a::Widget"),
            &Qualifier::SelfType,
            &["make"],
        );
        assert_eq!(
            got.map(CanonicalId::into_string).as_deref(),
            Some("engram::a::Widget::make")
        );
    }

    #[test]
    fn self_assoc_projection_fails_closed() {
        let crates = crates();
        let m = module(&["a"]);
        let ug = extract_use_graph("");
        let ctx = self_ctx(&crates, &m, &ug);
        // Self::Assoc::method — an associated-type projection is out of scope.
        assert_eq!(
            resolve_qualifier(
                &ctx,
                Some("engram::a::Widget"),
                &Qualifier::SelfType,
                &["Assoc", "method"]
            ),
            None
        );
        // Bare Self with no method also fails closed.
        assert_eq!(
            resolve_qualifier(&ctx, Some("engram::a::Widget"), &Qualifier::SelfType, &[]),
            None
        );
    }

    #[test]
    fn self_outside_impl_fails_closed() {
        let crates = crates();
        let m = module(&["a"]);
        let ug = extract_use_graph("");
        let ctx = self_ctx(&crates, &m, &ug);
        assert_eq!(
            resolve_qualifier(&ctx, None, &Qualifier::SelfType, &["make"]),
            None
        );
    }

    #[test]
    fn self_string_cannot_forge_the_marker() {
        // A source path textually rooted at `Self` is NOT the typed marker and
        // never resolves — only `Qualifier::SelfType` substitutes the enclosing
        // type (088-F Self-forge defence).
        assert_eq!(resolve(&["a"], "", "Self::make"), None);
        assert_eq!(resolve(&["a"], "", "Self::Assoc::make"), None);
    }

    #[test]
    fn path_qualifier_delegates_to_resolve_path() {
        let crates = crates();
        let m = module(&["z"]);
        let ug = extract_use_graph("use crate::a::Widget;");
        let ctx = self_ctx(&crates, &m, &ug);
        let got = resolve_qualifier(
            &ctx,
            None,
            &Qualifier::Path("Widget".to_owned()),
            &["build"],
        );
        assert_eq!(
            got.map(CanonicalId::into_string).as_deref(),
            Some("engram::a::Widget::build")
        );
    }
}
