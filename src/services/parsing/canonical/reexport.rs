//! A4 — re-export (`pub use`) transitive closure with cycle/depth cap.
//!
//! A workspace-wide map of `pub use` re-export edges (`<module>::<alias>` →
//! target canonical path). [`ReexportMap::canonicalize`] rewrites a path through
//! the **longest matching re-exported prefix**, transitively, so a call reaching
//! a type via a re-export resolves to the same canonical identity as the type's
//! original definition.
//!
//! **Fail-closed (D8):** re-export cycles and chains exceeding the depth/size cap
//! yield `None` rather than looping or guessing. Lookups are O(1) hashmap hits
//! and the bounded walk is effectively memoised by the map itself.

use std::collections::{HashMap, HashSet};

/// Maximum re-export hops before failing closed (size/depth cap, D8).
const MAX_REEXPORT_DEPTH: usize = 64;

/// A map of `pub use` re-export edges: a re-exported symbol's canonical path →
/// the canonical path it re-exports.
#[derive(Debug, Clone, Default)]
pub struct ReexportMap {
    edges: HashMap<String, String>,
}

impl ReexportMap {
    /// An empty re-export map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `reexport` (a `pub use` alias's canonical path) re-exports
    /// `target`.
    pub fn insert(&mut self, reexport: impl Into<String>, target: impl Into<String>) {
        self.edges.insert(reexport.into(), target.into());
    }

    /// Whether any re-export edges are recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    /// Number of re-export edges.
    #[must_use]
    pub fn len(&self) -> usize {
        self.edges.len()
    }

    /// Rewrite `path` through the re-export closure to its terminal (original)
    /// canonical path, or `None` on a cycle or when the cap is exceeded.
    ///
    /// Rewrites the **longest matching re-exported prefix** at each hop, so
    /// `a::Widget::method` follows a `pub use` of the `Widget` type. Fail-closed
    /// on cycles (a repeated state) and on chains longer than
    /// [`MAX_REEXPORT_DEPTH`] (D8).
    #[must_use]
    pub fn canonicalize(&self, path: &str) -> Option<String> {
        if self.edges.is_empty() {
            return Some(path.to_owned());
        }
        let mut current = path.to_owned();
        let mut visited: HashSet<String> = HashSet::new();
        for _ in 0..=MAX_REEXPORT_DEPTH {
            if !visited.insert(current.clone()) {
                return None; // cycle
            }
            match self.longest_prefix_rewrite(&current) {
                None => return Some(current), // terminal
                Some(next) => current = next,
            }
        }
        None // depth/size cap exceeded
    }

    /// Rewrite the longest `::`-boundary prefix of `path` that is a re-export
    /// edge, keeping the remaining suffix, or `None` when no prefix matches.
    fn longest_prefix_rewrite(&self, path: &str) -> Option<String> {
        let segs: Vec<&str> = path.split("::").collect();
        for take in (1..=segs.len()).rev() {
            let prefix = segs[..take].join("::");
            if let Some(target) = self.edges.get(&prefix) {
                let suffix = &segs[take..];
                return Some(if suffix.is_empty() {
                    target.clone()
                } else {
                    format!("{target}::{}", suffix.join("::"))
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_reexport_is_identity() {
        let map = ReexportMap::new();
        assert_eq!(
            map.canonicalize("engram::a::Widget::method").as_deref(),
            Some("engram::a::Widget::method")
        );
    }

    #[test]
    fn single_hop_rewrites_type_prefix() {
        let mut map = ReexportMap::new();
        map.insert("engram::a::Widget", "engram::b::Widget");
        assert_eq!(
            map.canonicalize("engram::a::Widget::method").as_deref(),
            Some("engram::b::Widget::method")
        );
    }

    #[test]
    fn exact_path_hop() {
        let mut map = ReexportMap::new();
        map.insert("engram::a::X", "engram::b::Y");
        assert_eq!(
            map.canonicalize("engram::a::X").as_deref(),
            Some("engram::b::Y")
        );
    }

    #[test]
    fn transitive_chain() {
        let mut map = ReexportMap::new();
        map.insert("k::a", "k::b");
        map.insert("k::b", "k::c");
        assert_eq!(map.canonicalize("k::a::m").as_deref(), Some("k::c::m"));
    }

    #[test]
    fn cycle_fails_closed() {
        let mut map = ReexportMap::new();
        map.insert("k::a", "k::b");
        map.insert("k::b", "k::a");
        assert_eq!(map.canonicalize("k::a::m"), None);
    }

    #[test]
    fn depth_cap_fails_closed() {
        let mut map = ReexportMap::new();
        // A chain longer than the cap: n0 -> n1 -> ... -> n(cap+5).
        for i in 0..(MAX_REEXPORT_DEPTH + 5) {
            map.insert(format!("k::n{i}"), format!("k::n{}", i + 1));
        }
        assert_eq!(map.canonicalize("k::n0::m"), None);
    }

    #[test]
    fn longest_prefix_wins() {
        let mut map = ReexportMap::new();
        map.insert("k::a", "k::WRONG");
        map.insert("k::a::b", "k::Y");
        // The more specific `k::a::b` prefix must win over `k::a`.
        assert_eq!(map.canonicalize("k::a::b::m").as_deref(), Some("k::Y::m"));
    }
}
