//! A2 — full per-file `use`-graph extraction for Rust.
//!
//! Extends the legacy first-child-only `extract_use_path` into a complete
//! per-file view of `use` declarations: single imports, brace groups
//! (`{a, b}`), nested groups, `as` aliases, `self`/`super`/`crate` roots,
//! `use path::{self, …}`, glob markers (`use a::b::*`), and `pub use`
//! re-export flags.
//!
//! **Precision-neutral**: the graph is identity *data* consumed by the resolver
//! (A3/A4). It records raw bindings and glob *markers* only; it never decides an
//! edge. Glob imports are recorded as markers (never expanded), so the resolver
//! can fail closed on any name that could originate from a glob.

use super::module_path::mod_mapping_is_non_default;

use tree_sitter::{Node, Parser};

/// A single non-glob `use` binding: a local alias mapped to its source path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseBinding {
    /// The name introduced into the current scope (the final path segment, or
    /// the `as` alias).
    pub alias: String,
    /// The source path exactly as written from the `use` root (e.g.
    /// `crate::a::B`, `std::collections::HashMap`, `super::x::Y`).
    pub path: String,
    /// `true` when introduced by a `pub use` re-export.
    pub is_reexport: bool,
}

/// The `use` graph for one Rust file (module-level `use` declarations).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UseGraph {
    /// All non-glob bindings, in source order.
    pub bindings: Vec<UseBinding>,
    /// Base paths of glob imports (`use a::b::*` → `a::b`). Recorded as markers
    /// only and never expanded (fail-closed).
    pub globs: Vec<String>,
    /// Top-level items that prove a bare path root is local to this module.
    pub local_roots: Vec<String>,
    /// Whether the file contains a nested `use` declaration outside module scope.
    pub has_nested_use: bool,
    /// Whether a top-level `mod` declaration uses non-default or conditional
    /// mapping attributes (`#[path]`, `#[cfg]`, or `#[cfg_attr]`).
    pub has_non_default_mod_mapping: bool,
    /// Top-level module names whose declarations use non-default or conditional
    /// mapping attributes.
    pub non_default_mod_roots: Vec<String>,
    /// Type-parameter names declared anywhere in the file (`fn f<T>`, `impl<T>`,
    /// `struct S<T>`, …). A bare `T::method()` call whose head matches one of
    /// these may be a generic-parameter method call shadowing a same-named local
    /// type; resolving it would require type inference, so the resolver fails
    /// closed on any such head (M2, no-false-edge invariant).
    pub generic_type_params: Vec<String>,
    /// Type-item NAMES declared inside a block scope (a function body or nested
    /// `{ ... }` block) rather than at module level. A block-local `struct T`
    /// shadows a same-named top-level type at call sites within that scope, so a
    /// bare `T::method()` head matching one of these could resolve to the WRONG
    /// (top-level) type; the resolver fails closed on any such head (M2 sibling,
    /// no-false-edge invariant).
    pub block_local_type_names: Vec<String>,
    /// Local aliases introduced by `extern crate <crate> as <alias>;`. The alias
    /// re-points a crate-root name to a DIFFERENT crate, shadowing the
    /// extern-prelude name — `extern crate ext as demo; demo::build()` designates
    /// `ext`, not the workspace crate `demo`. The resolver must not take the
    /// workspace-crate fast path for such a head (no-false-edge invariant).
    pub extern_crate_aliases: Vec<String>,
}

impl UseGraph {
    /// Whether the file contains any glob import (`use …::*`).
    #[must_use]
    pub fn has_glob(&self) -> bool {
        !self.globs.is_empty()
    }

    /// All bindings that introduce `alias` into scope (normally zero or one; two
    /// or more signals a genuine ambiguity the resolver must fail closed on).
    pub fn bindings_for<'a>(&'a self, alias: &str) -> impl Iterator<Item = &'a UseBinding> {
        let alias = alias.to_owned();
        self.bindings.iter().filter(move |b| b.alias == alias)
    }

    /// Whether `root` is proven to be a top-level item in the current module.
    #[must_use]
    pub fn has_local_root(&self, root: &str) -> bool {
        self.local_roots.iter().any(|r| r == root)
    }

    /// Whether the file contains nested `use` declarations whose lexical scope is
    /// not modelled by this module-level graph.
    #[must_use]
    pub fn has_nested_use(&self) -> bool {
        self.has_nested_use
    }

    /// Whether any top-level `mod` declaration has non-default mapping
    /// attributes, making filesystem-derived module identity unsafe for call
    /// target resolution.
    #[must_use]
    pub fn has_non_default_mod_mapping(&self) -> bool {
        self.has_non_default_mod_mapping
    }

    /// Module roots introduced by non-default mapping declarations.
    #[must_use]
    pub fn non_default_mod_roots(&self) -> &[String] {
        &self.non_default_mod_roots
    }

    /// Whether `name` is declared as a generic type parameter anywhere in the
    /// file. The resolver fails closed on a bare `name::method()` head that
    /// matches, because a generic parameter shadows any same-named local type and
    /// the true callee requires trait/type inference the resolver refuses to do
    /// (M2, no-false-edge invariant).
    #[must_use]
    pub fn is_generic_param(&self, name: &str) -> bool {
        self.generic_type_params.iter().any(|p| p == name)
    }

    /// Whether `name` is declared as a block-local type item (inside a function
    /// body or nested block). The resolver fails closed on a bare
    /// `name::method()` head that matches, because a block-local type shadows any
    /// same-named top-level type at the call site and resolving to the top-level
    /// type would forge a false canonical edge (no-false-edge invariant).
    #[must_use]
    pub fn is_block_local_type(&self, name: &str) -> bool {
        self.block_local_type_names.iter().any(|n| n == name)
    }

    /// Whether `name` is re-pointed by an `extern crate ... as name;` alias to a
    /// different crate, shadowing the extern-prelude crate name. The resolver must
    /// not take the workspace-crate fast path for such a head (no-false-edge
    /// invariant).
    #[must_use]
    pub fn is_extern_crate_alias(&self, name: &str) -> bool {
        self.extern_crate_aliases.iter().any(|n| n == name)
    }
}

/// Extract the module-level `use` graph from Rust `source`.
///
/// Only top-level (file-module) `use` declarations are captured. Unit B records
/// nested `use` declarations as a fail-closed marker for call-target resolution:
/// the graph does not model lexical scopes deeply enough to resolve calls in
/// those files safely.
#[must_use]
pub fn extract_use_graph(source: &str) -> UseGraph {
    let mut graph = UseGraph::default();
    let Some(tree) = parse_tree(source) else {
        return graph;
    };
    let root = tree.root_node();
    let mut cursor = root.walk();
    let mut pending_attrs = Vec::new();
    for child in root.children(&mut cursor) {
        if child.kind() == "attribute_item" {
            pending_attrs.push(node_text(child, source));
            continue;
        }
        if child.kind() == "use_declaration" {
            let is_pub = has_pub_visibility(child, source);
            if let Some(arg) = child
                .child_by_field_name("argument")
                .or_else(|| first_use_clause(child))
            {
                walk_use_tree(arg, source, "", is_pub, 0, &mut graph);
            }
        } else if contains_use_declaration(child) {
            graph.has_nested_use = true;
        }

        if child.kind() == "mod_item" {
            let mut attr_texts = pending_attrs.clone();
            attr_texts.extend(child_attributes(child, source));
            if mod_mapping_is_non_default(&attr_texts) {
                graph.has_non_default_mod_mapping = true;
                if let Some(root_name) = local_root_name(child, source) {
                    graph.non_default_mod_roots.push(root_name);
                }
            }
            // Descend an inline module body so a NESTED `#[path]`/`#[cfg]` remap
            // (`mod outer { #[path=…] mod inner; }`) is recorded with its full
            // relative path — its default filesystem identity must not be trusted
            // for call resolution either (M3, no-false-edge invariant).
            if let (Some(name), Some(body)) = (
                local_root_name(child, source),
                child.child_by_field_name("body"),
            ) {
                let prefix = format!("{name}::");
                collect_nested_non_default_mod_roots(body, source, &prefix, &mut graph);
            }
        }
        pending_attrs.clear();

        if let Some(root_name) = local_root_name(child, source) {
            graph.local_roots.push(root_name);
        }
    }
    // Collect generic type-parameter declarations across the whole file so the
    // resolver can fail closed on a bare `T::method()` head that a generic param
    // shadows (M2). Over-approximating scope (file-wide, not lexical) is safe:
    // it only ever drops an edge, never invents one.
    collect_generic_type_params(root, source, &mut graph.generic_type_params);
    graph.generic_type_params.sort();
    graph.generic_type_params.dedup();
    // Collect block-local type items and extern-crate aliases so the resolver can
    // fail closed on a bare head that either shadows a same-named top-level type
    // (block-local `struct T`) or re-points a crate name (`extern crate ext as
    // demo`). Over-approximating scope file-wide is safe: it only drops an edge,
    // never invents one (no-false-edge invariant).
    collect_block_local_type_names(root, source, false, &mut graph.block_local_type_names);
    graph.block_local_type_names.sort();
    graph.block_local_type_names.dedup();
    collect_extern_crate_aliases(root, source, &mut graph.extern_crate_aliases);
    graph.extern_crate_aliases.sort();
    graph.extern_crate_aliases.dedup();
    graph
}

/// Recurse into an inline module body collecting nested non-default `mod`
/// mappings, prefixing each with its enclosing inline-module path so a nested
/// `#[path]`/`#[cfg]` remap is recorded as `outer::inner` (M3).
fn collect_nested_non_default_mod_roots(
    body: Node<'_>,
    source: &str,
    prefix: &str,
    graph: &mut UseGraph,
) {
    let mut cursor = body.walk();
    let mut pending_attrs: Vec<String> = Vec::new();
    for child in body.children(&mut cursor) {
        if child.kind() == "attribute_item" {
            pending_attrs.push(node_text(child, source));
            continue;
        }
        if child.kind() == "mod_item" {
            if let Some(name) = local_root_name(child, source) {
                let mut attr_texts = pending_attrs.clone();
                attr_texts.extend(child_attributes(child, source));
                let full = format!("{prefix}{name}");
                if mod_mapping_is_non_default(&attr_texts) {
                    graph.has_non_default_mod_mapping = true;
                    graph.non_default_mod_roots.push(full.clone());
                }
                if let Some(inner) = child.child_by_field_name("body") {
                    let inner_prefix = format!("{full}::");
                    collect_nested_non_default_mod_roots(inner, source, &inner_prefix, graph);
                }
            }
        }
        pending_attrs.clear();
    }
}

/// Collect every generic type-parameter NAME declared under `node` (any depth).
/// Only parameter declarations (`<T>`, `<T: Bound>`, `<T = Default>`) are
/// gathered — bound/default type ARGUMENTS are not, because a parameter list is
/// not descended into.
fn collect_generic_type_params(node: Node<'_>, source: &str, out: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameters" {
            let mut param_cursor = child.walk();
            for param in child.named_children(&mut param_cursor) {
                if let Some(name) = generic_param_name(param, source) {
                    out.push(name);
                }
            }
        }
        // Recurse into EVERY child, including a `type_parameters` node's own
        // subtree: a generic can be DECLARED in a nested inline item inside a
        // bound or const-generic default (`<T = { fn f<U: B>() { U::m() } 0 }>`),
        // and such a `U` must also fail closed. Type ARGUMENTS live under distinct
        // `type_arguments` nodes and are never mistaken for declarations, because a
        // name is only ever gathered from the direct children of a
        // `type_parameters` node.
        collect_generic_type_params(child, source, out);
    }
}

/// Collect type-item NAMES declared inside a block scope (a function body or
/// nested `{ ... }` block) under `node`. Module and impl item lists are
/// `declaration_list` nodes, NOT `block` nodes, so top-level and module-scoped
/// items are excluded — only items lexically inside a `block` are gathered.
///
/// A block-local `struct T` shadows a same-named top-level type at call sites
/// within that scope, so a bare `T::method()` head that matches one of these
/// could resolve to the wrong (top-level) type. Fail-closed over-approximation
/// (file-wide, not lexical) is safe: it only ever drops an edge, never invents
/// one (no-false-edge invariant).
fn collect_block_local_type_names(
    node: Node<'_>,
    source: &str,
    inside_block: bool,
    out: &mut Vec<String>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if inside_block {
            if let Some(name) = local_root_name(child, source) {
                out.push(name);
            }
        }
        // A `block` node's descendants are in block scope; once inside a block,
        // stay inside so a nested module/impl item inside a function body is still
        // treated as shadowing-scoped.
        let child_inside = inside_block || child.kind() == "block";
        collect_block_local_type_names(child, source, child_inside, out);
    }
}

/// Collect every `extern crate <crate> as <alias>;` ALIAS name declared under
/// `node` (any depth). Only the aliased form is collected: a plain `extern crate
/// foo;` binds `foo` to crate `foo` (identity, no shadowing), whereas `extern
/// crate ext as demo;` re-points `demo` to a DIFFERENT crate and shadows the
/// extern-prelude name `demo` (no-false-edge invariant).
fn collect_extern_crate_aliases(node: Node<'_>, source: &str, out: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "extern_crate_declaration" {
            if let Some(alias) = child.child_by_field_name("alias") {
                out.push(node_text(alias, source));
            }
        }
        collect_extern_crate_aliases(child, source, out);
    }
}

/// The declared name of a single generic parameter node, or `None` for
/// lifetimes, const parameters, and metavariables (which cannot be a call-head
/// type qualifier).
fn generic_param_name(param: Node<'_>, source: &str) -> Option<String> {
    match param.kind() {
        "type_identifier" => Some(node_text(param, source)),
        // tree-sitter-rust wraps each generic in a `type_parameter` node whose
        // `name` field is the declared identifier (bounds/defaults live in
        // sibling fields, so read the field rather than scanning children).
        "type_parameter" => {
            let name = param.child_by_field_name("name")?;
            (name.kind() == "type_identifier").then(|| node_text(name, source))
        }
        // Older/alternate grammars spell the same construct as a
        // `constrained_type_parameter` / `optional_type_parameter` whose first
        // `type_identifier` child is the declared name.
        "constrained_type_parameter" | "optional_type_parameter" => {
            let mut cursor = param.walk();
            param
                .named_children(&mut cursor)
                .find(|n| n.kind() == "type_identifier")
                .map(|n| node_text(n, source))
        }
        _ => None,
    }
}

fn local_root_name(node: Node<'_>, source: &str) -> Option<String> {
    match node.kind() {
        "struct_item" | "enum_item" | "union_item" | "trait_item" | "type_item" | "mod_item" => {
            node.child_by_field_name("name")
                .map(|n| node_text(n, source))
        }
        _ => None,
    }
}

fn child_attributes(node: Node<'_>, source: &str) -> Vec<String> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| child.kind() == "attribute_item")
        .map(|child| node_text(child, source))
        .collect()
}

fn contains_use_declaration(node: Node<'_>) -> bool {
    if node.kind() == "use_declaration" {
        return true;
    }
    let mut cursor = node.walk();
    node.children(&mut cursor).any(contains_use_declaration)
}

fn parse_tree(source: &str) -> Option<tree_sitter::Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .ok()?;
    parser.parse(source, None)
}

fn node_text(node: Node<'_>, source: &str) -> String {
    source[node.byte_range()].to_owned()
}

fn has_pub_visibility(node: Node<'_>, source: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|ch| ch.kind() == "visibility_modifier" && node_text(ch, source).starts_with("pub"))
}

fn is_use_clause(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "scoped_identifier"
            | "scoped_use_list"
            | "use_wildcard"
            | "use_as_clause"
            | "use_list"
            | "crate"
            | "self"
            | "super"
    )
}

fn first_use_clause(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|ch| is_use_clause(ch.kind()))
}

fn join_path(prefix: &str, seg: &str) -> String {
    if prefix.is_empty() {
        seg.to_owned()
    } else if seg.is_empty() {
        prefix.to_owned()
    } else {
        format!("{prefix}::{seg}")
    }
}

fn last_segment(path: &str) -> Option<&str> {
    path.rsplit("::").next().filter(|s| !s.is_empty())
}

/// The path-prefix text of a `scoped_use_list` / `use_as_clause` (the part
/// before the `{…}` or `as`).
fn path_prefix(node: Node<'_>, source: &str) -> String {
    node.child_by_field_name("path")
        .map(|p| node_text(p, source))
        .unwrap_or_default()
}

/// Maximum `use`-tree nesting depth before extraction stops descending. Real
/// `use` trees are shallow; a pathologically deep spelling (adversarial or
/// generated) is truncated rather than risking unbounded recursion. Dropped
/// deep bindings fail closed at the resolver (D6), never a wrong binding.
const MAX_USE_TREE_DEPTH: u32 = 64;

fn walk_use_tree(
    node: Node<'_>,
    source: &str,
    prefix: &str,
    is_pub: bool,
    depth: u32,
    graph: &mut UseGraph,
) {
    if depth > MAX_USE_TREE_DEPTH {
        return;
    }
    match node.kind() {
        "identifier" | "crate" | "self" | "super" => {
            let name = node_text(node, source);
            if name == "self" {
                // `use a::b::{self, …}` binds the prefix's own last segment.
                if let Some(last) = last_segment(prefix) {
                    graph.bindings.push(UseBinding {
                        alias: last.to_owned(),
                        path: prefix.to_owned(),
                        is_reexport: is_pub,
                    });
                }
            } else {
                let path = join_path(prefix, &name);
                graph.bindings.push(UseBinding {
                    alias: name,
                    path,
                    is_reexport: is_pub,
                });
            }
        }
        "scoped_identifier" => {
            let path = join_path(prefix, &node_text(node, source));
            if let Some(alias) = last_segment(&path) {
                let alias = alias.to_owned();
                graph.bindings.push(UseBinding {
                    alias,
                    path,
                    is_reexport: is_pub,
                });
            }
        }
        "scoped_use_list" => {
            let new_prefix = join_path(prefix, &path_prefix(node, source));
            if let Some(list) = node
                .child_by_field_name("list")
                .or_else(|| child_of_kind(node, "use_list"))
            {
                walk_use_tree(list, source, &new_prefix, is_pub, depth + 1, graph);
            }
        }
        "use_list" => {
            let mut cursor = node.walk();
            let items: Vec<Node<'_>> = node
                .children(&mut cursor)
                .filter(|c| is_use_clause(c.kind()))
                .collect();
            for item in items {
                walk_use_tree(item, source, prefix, is_pub, depth + 1, graph);
            }
        }
        "use_as_clause" => {
            if let (Some(path_node), Some(alias_node)) = (
                node.child_by_field_name("path"),
                node.child_by_field_name("alias"),
            ) {
                let path_text = node_text(path_node, source);
                // `use a::b::{self as x}` (and `use a::b as x` whose path is
                // `self`) aliases the PREFIX itself, not a `::self` child.
                let path = if path_text == "self" {
                    prefix.to_owned()
                } else {
                    join_path(prefix, &path_text)
                };
                let alias = node_text(alias_node, source);
                // `as _` introduces no referenceable name — skip it.
                if alias != "_" && !path.is_empty() {
                    graph.bindings.push(UseBinding {
                        alias,
                        path,
                        is_reexport: is_pub,
                    });
                }
            }
        }
        "use_wildcard" => {
            let text = node_text(node, source);
            let base = text.trim_end_matches('*').trim_end_matches(':');
            let full = join_path(prefix, base);
            if !full.is_empty() {
                graph.globs.push(full);
            }
        }
        _ => {}
    }
}

fn child_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|c| c.kind() == kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding<'a>(g: &'a UseGraph, alias: &str) -> Option<&'a UseBinding> {
        let mut it = g.bindings_for(alias);
        let first = it.next();
        assert!(it.next().is_none(), "expected a single binding for {alias}");
        first
    }

    #[test]
    fn collects_fn_generic_type_param_and_excludes_lifetimes() {
        let g = extract_use_graph(
            "pub fn caller<'a, T: Builder, const N: usize>(x: &'a T) { let _ = N; }\n",
        );
        assert!(g.is_generic_param("T"), "T is a type param");
        assert!(!g.is_generic_param("a"), "lifetime is not a type param");
        assert!(
            !g.is_generic_param("N"),
            "const generic is not a type param"
        );
    }

    #[test]
    fn collects_impl_and_struct_generic_type_params() {
        let g = extract_use_graph(
            "pub struct Holder<V> { v: V }\nimpl<W> Holder<W> { fn take() {} }\n",
        );
        assert!(g.is_generic_param("V"), "struct type param V");
        assert!(g.is_generic_param("W"), "impl type param W");
    }

    #[test]
    fn collects_block_local_type_and_excludes_top_level() {
        let g = extract_use_graph(
            "pub struct Top;\nfn caller() { struct T; enum E { A } let _ = T; }\n",
        );
        assert!(
            g.is_block_local_type("T"),
            "block-local struct T shadows any top-level T at the call site"
        );
        assert!(g.is_block_local_type("E"), "block-local enum E");
        assert!(
            !g.is_block_local_type("Top"),
            "a top-level type is a local_root, not a block-local shadow"
        );
        assert!(
            g.has_local_root("Top"),
            "top-level Top is still recorded as a local root"
        );
    }

    #[test]
    fn collects_extern_crate_alias_and_excludes_plain() {
        let g = extract_use_graph("extern crate ext as demo;\nextern crate plain;\n");
        assert!(
            g.is_extern_crate_alias("demo"),
            "`extern crate ext as demo` re-points demo to a different crate"
        );
        assert!(
            !g.is_extern_crate_alias("plain"),
            "a plain `extern crate plain` is identity, not a shadowing alias"
        );
        assert!(
            !g.is_extern_crate_alias("ext"),
            "the aliased crate name itself is not the shadowing binding"
        );
    }

    #[test]
    fn single_import() {
        let g = extract_use_graph("use std::collections::HashMap;");
        let b = binding(&g, "HashMap").expect("HashMap bound");
        assert_eq!(b.path, "std::collections::HashMap");
        assert!(!b.is_reexport);
        assert!(!g.has_glob());
    }

    #[test]
    fn bare_import() {
        let g = extract_use_graph("use foo;");
        let b = binding(&g, "foo").expect("foo bound");
        assert_eq!(b.path, "foo");
    }

    #[test]
    fn brace_group() {
        let g = extract_use_graph("use a::b::{c, d};");
        assert_eq!(binding(&g, "c").unwrap().path, "a::b::c");
        assert_eq!(binding(&g, "d").unwrap().path, "a::b::d");
    }

    #[test]
    fn as_alias() {
        let g = extract_use_graph("use a::b::Widget as W;");
        assert!(binding(&g, "Widget").is_none());
        assert_eq!(binding(&g, "W").unwrap().path, "a::b::Widget");
    }

    #[test]
    fn glob_is_marker_only() {
        let g = extract_use_graph("use a::b::*;");
        assert!(g.bindings.is_empty());
        assert!(g.has_glob());
        assert_eq!(g.globs, vec!["a::b".to_owned()]);
    }

    #[test]
    fn crate_root() {
        let g = extract_use_graph("use crate::services::Widget;");
        assert_eq!(
            binding(&g, "Widget").unwrap().path,
            "crate::services::Widget"
        );
    }

    #[test]
    fn self_and_super_roots() {
        let g = extract_use_graph("use self::foo::Bar; use super::baz::Qux;");
        assert_eq!(binding(&g, "Bar").unwrap().path, "self::foo::Bar");
        assert_eq!(binding(&g, "Qux").unwrap().path, "super::baz::Qux");
    }

    #[test]
    fn pub_use_is_reexport() {
        let g = extract_use_graph("pub use a::b::C;");
        let b = binding(&g, "C").unwrap();
        assert_eq!(b.path, "a::b::C");
        assert!(b.is_reexport, "pub use flagged as re-export");
    }

    #[test]
    fn plain_use_is_not_reexport() {
        let g = extract_use_graph("use a::b::C;");
        assert!(!binding(&g, "C").unwrap().is_reexport);
    }

    #[test]
    fn nested_groups() {
        let g = extract_use_graph("use a::{b::C, d::{E, F}};");
        assert_eq!(binding(&g, "C").unwrap().path, "a::b::C");
        assert_eq!(binding(&g, "E").unwrap().path, "a::d::E");
        assert_eq!(binding(&g, "F").unwrap().path, "a::d::F");
    }

    #[test]
    fn self_in_group_binds_prefix() {
        // `use a::b::{self, C}` binds `b` -> `a::b` and `C` -> `a::b::C`.
        let g = extract_use_graph("use a::b::{self, C};");
        assert_eq!(binding(&g, "b").unwrap().path, "a::b");
        assert_eq!(binding(&g, "C").unwrap().path, "a::b::C");
    }

    #[test]
    fn self_alias_in_group_binds_prefix() {
        // `use a::b::{self as alias}` aliases the PREFIX `a::b`, not `a::b::self`.
        let g = extract_use_graph("use a::b::{self as alias};");
        assert!(binding(&g, "self").is_none());
        assert_eq!(binding(&g, "alias").unwrap().path, "a::b");
    }

    #[test]
    fn multiple_declarations_accumulate() {
        let g = extract_use_graph("use a::B;\nuse c::D;\npub use e::F;");
        assert_eq!(binding(&g, "B").unwrap().path, "a::B");
        assert_eq!(binding(&g, "D").unwrap().path, "c::D");
        assert!(binding(&g, "F").unwrap().is_reexport);
    }

    #[test]
    fn glob_and_explicit_coexist() {
        // The resolver applies precedence (D6); the graph records both.
        let g = extract_use_graph("use a::*;\nuse a::B;");
        assert_eq!(g.globs, vec!["a".to_owned()]);
        assert_eq!(binding(&g, "B").unwrap().path, "a::B");
    }
}
