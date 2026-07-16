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
}

/// Extract the module-level `use` graph from Rust `source`.
///
/// Only top-level (file-module) `use` declarations are captured; `use`
/// declarations nested inside inline `mod` blocks belong to that inner scope
/// and are intentionally not surfaced here (their absence is fail-closed for the
/// resolver, never a wrong binding).
#[must_use]
pub fn extract_use_graph(source: &str) -> UseGraph {
    let mut graph = UseGraph::default();
    let Some(tree) = parse_tree(source) else {
        return graph;
    };
    let root = tree.root_node();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "use_declaration" {
            let is_pub = has_pub_visibility(child, source);
            if let Some(arg) = child
                .child_by_field_name("argument")
                .or_else(|| first_use_clause(child))
            {
                walk_use_tree(arg, source, "", is_pub, &mut graph);
            }
        }
    }
    graph
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

fn walk_use_tree(node: Node<'_>, source: &str, prefix: &str, is_pub: bool, graph: &mut UseGraph) {
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
                walk_use_tree(list, source, &new_prefix, is_pub, graph);
            }
        }
        "use_list" => {
            let mut cursor = node.walk();
            let items: Vec<Node<'_>> = node
                .children(&mut cursor)
                .filter(|c| is_use_clause(c.kind()))
                .collect();
            for item in items {
                walk_use_tree(item, source, prefix, is_pub, graph);
            }
        }
        "use_as_clause" => {
            if let (Some(path_node), Some(alias_node)) = (
                node.child_by_field_name("path"),
                node.child_by_field_name("alias"),
            ) {
                let path = join_path(prefix, &node_text(path_node, source));
                let alias = node_text(alias_node, source);
                // `as _` introduces no referenceable name — skip it.
                if alias != "_" {
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
