//! T2 — Python import-binding capture (feature 096-F, Unit T2).
//!
//! Walks tree-sitter `import_statement` / `import_from_statement` nodes to map a
//! **local name** to its **canonical origin, binding kind, and source position**
//! (R2 + F14). Every firm binding carries a byte-offset `position` so T5c's
//! order-aware winner anchor (F1) and the F4 star invalidator can reason about
//! source order.
//!
//! **Fail-closed (013-D).** A local name is a *firm* module-scope binding only
//! when exactly one unconditional module-scope import binds it. Any of the
//! following instead poison the name (recorded as a positioned **ambiguity
//! marker**, Anchor C T-b/T-c) so a later stage treats a same-file same-name
//! `def` as *contested* rather than minting a direct `M.f` edge:
//!
//! * a relative import (`from . import x`, `from .mod import y`),
//! * a duplicate / competing binding of the same local name (M1),
//! * a control-flow-conditional / `try`-guarded import (not proven to execute).
//!
//! A module-scope `from N import *` mints **no** binding for any name; it is
//! recorded as a positioned **order-aware invalidator** (F4) — the star never
//! resolves its own name and T5c fails closed only when a star occurs *after*
//! the winning binding.

use std::collections::HashMap;

use tree_sitter::{Node, Parser};

/// Whether a binding names a **module receiver** or an **imported symbol** (R2).
///
/// T5b must distinguish `import pkg` (`pkg.func()` — a module receiver) from
/// `from pkg import parse` (`parse()` — an imported symbol); without the kind a
/// from-import would be mis-resolved as a module and mint a wrong edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    /// `import a.b` / `import a.b as c` — the local name is a module receiver.
    ModuleImport,
    /// `from N import x` / `... as p` — the local name is an imported symbol.
    FromImportSymbol,
}

/// A firm import binding: the canonical origin, its [`BindingKind`], and the
/// byte-offset `position` of the binding import (F14, for source-order rules).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportBinding {
    /// Canonical dotted origin: `"a.b"` for `import a.b as c`, `"a"` for
    /// `import a.b`, `"N.x"` for `from N import x`.
    pub canonical_path: String,
    /// Whether the local name is a module receiver or an imported symbol.
    pub kind: BindingKind,
    /// Byte offset of the binding `import` statement (source order).
    pub position: usize,
}

/// Module-scope import bindings plus the positioned fail-closed markers.
#[derive(Debug, Default, Clone)]
pub struct ImportBindings {
    module: HashMap<String, ImportBinding>,
    ambiguous: HashMap<String, usize>,
    stars: Vec<usize>,
}

impl ImportBindings {
    /// The firm module-scope binding for `name`, if exactly one unconditional
    /// module-scope import binds it and nothing poisons it.
    #[must_use]
    pub fn module_binding(&self, name: &str) -> Option<&ImportBinding> {
        self.module.get(name)
    }

    /// Whether `name` is poisoned (fail closed) at module scope — a relative,
    /// duplicate/competing, or conditional import bound it.
    #[must_use]
    pub fn is_ambiguous(&self, name: &str) -> bool {
        self.ambiguous.contains_key(name)
    }

    /// The recorded position of `name`'s ambiguity marker, if any (Anchor C).
    #[must_use]
    pub fn ambiguity_position(&self, name: &str) -> Option<usize> {
        self.ambiguous.get(name).copied()
    }

    /// Byte-offset positions of module-scope `from N import *` invalidators
    /// (F4), in encounter order. T5c compares these against the winner position.
    #[must_use]
    pub fn star_invalidators(&self) -> &[usize] {
        &self.stars
    }
}

/// Extract module-scope import bindings and fail-closed markers from Python
/// `source`. Infallible: a parse failure yields empty bindings (fail closed —
/// every name falls back to name-only resolution).
#[must_use]
pub fn extract_python_import_bindings(source: &str) -> ImportBindings {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .is_err()
    {
        return ImportBindings::default();
    }
    let Some(tree) = parser.parse(source, None) else {
        return ImportBindings::default();
    };

    let mut collector = Collector::default();
    collect_module_scope(tree.root_node(), source, &mut collector);
    collector.finalize()
}

/// Accumulates raw binding candidates, positioned poison markers, and star
/// invalidator positions before fail-closed finalization.
#[derive(Default)]
struct Collector {
    firm: HashMap<String, Vec<ImportBinding>>,
    markers: HashMap<String, usize>,
    stars: Vec<usize>,
}

impl Collector {
    fn add_firm(&mut self, name: String, binding: ImportBinding) {
        self.firm.entry(name).or_default().push(binding);
    }

    fn add_marker(&mut self, name: String, position: usize) {
        self.markers
            .entry(name)
            .and_modify(|p| *p = (*p).min(position))
            .or_insert(position);
    }

    fn add_star(&mut self, position: usize) {
        self.stars.push(position);
    }

    /// Collapse candidates into firm bindings and fail-closed markers (M1): a
    /// name is firm only with exactly one candidate and no poison marker.
    fn finalize(self) -> ImportBindings {
        let Collector {
            firm,
            markers,
            mut stars,
        } = self;
        stars.sort_unstable();

        let mut module = HashMap::new();
        let mut ambiguous: HashMap<String, usize> = HashMap::new();

        for (name, mut candidates) in firm {
            let min_pos = candidates
                .iter()
                .map(|b| b.position)
                .min()
                .unwrap_or(usize::MAX);
            let marker_pos = markers.get(&name).copied();
            if candidates.len() == 1 && marker_pos.is_none() {
                if let Some(binding) = candidates.pop() {
                    module.insert(name, binding);
                }
            } else {
                let pos = marker_pos.map_or(min_pos, |mp| mp.min(min_pos));
                ambiguous.insert(name, pos);
            }
        }

        for (name, position) in markers {
            if !module.contains_key(&name) {
                ambiguous.entry(name).or_insert(position);
            }
        }

        ImportBindings {
            module,
            ambiguous,
            stars,
        }
    }
}

/// Walk the direct children of the module root: unconditional module-scope
/// imports become firm candidates; imports nested under a module-level
/// control-flow statement are conditional (T-c); `function`/`class` bodies are
/// inner scopes owned by T2b and skipped here.
fn collect_module_scope(root: Node<'_>, source: &str, collector: &mut Collector) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "import_statement" => process_import(child, source, collector, false),
            "import_from_statement" => process_import_from(child, source, collector, false),
            "function_definition" | "class_definition" | "decorated_definition" => {}
            _ => collect_conditional(child, source, collector),
        }
    }
}

/// Recurse into a module-level compound statement, recording any imports it
/// contains as conditional markers (T-c). Stops at nested `function`/`class`
/// definitions (their imports are inner-scope, owned by T2b).
fn collect_conditional(node: Node<'_>, source: &str, collector: &mut Collector) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "import_statement" => process_import(child, source, collector, true),
            "import_from_statement" => process_import_from(child, source, collector, true),
            "function_definition" | "class_definition" | "decorated_definition" => {}
            _ => collect_conditional(child, source, collector),
        }
    }
}

/// Process `import a.b` / `import a.b as c` (possibly comma-separated). An
/// aliased import binds the alias to the full dotted module; a plain dotted
/// import binds the *root* package name to itself.
fn process_import(node: Node<'_>, source: &str, collector: &mut Collector, conditional: bool) {
    let position = node.start_byte();
    let mut cursor = node.walk();
    for name_node in node.children_by_field_name("name", &mut cursor) {
        match name_node.kind() {
            "aliased_import" => {
                let alias = name_node.child_by_field_name("alias");
                let module = name_node.child_by_field_name("name");
                if let (Some(alias), Some(module)) = (alias, module) {
                    if alias.kind() == "identifier" {
                        let local = node_text(alias, source).to_owned();
                        let canonical = dotted_text(module, source);
                        record(
                            collector,
                            conditional,
                            local,
                            canonical,
                            BindingKind::ModuleImport,
                            position,
                        );
                    }
                }
            }
            "dotted_name" => {
                if let Some(root_id) = name_node.named_child(0) {
                    if root_id.kind() == "identifier" {
                        let local = node_text(root_id, source).to_owned();
                        let canonical = local.clone();
                        record(
                            collector,
                            conditional,
                            local,
                            canonical,
                            BindingKind::ModuleImport,
                            position,
                        );
                    }
                }
            }
            "identifier" => {
                let local = node_text(name_node, source).to_owned();
                let canonical = local.clone();
                record(
                    collector,
                    conditional,
                    local,
                    canonical,
                    BindingKind::ModuleImport,
                    position,
                );
            }
            _ => {}
        }
    }
}

/// Process `from N import ...`. A relative `module_name` fails closed (T-b); a
/// `from N import *` records a star invalidator (F4); otherwise each imported
/// symbol binds `local -> "N.symbol"` as a [`BindingKind::FromImportSymbol`].
fn process_import_from(node: Node<'_>, source: &str, collector: &mut Collector, conditional: bool) {
    let position = node.start_byte();
    let module_node = node.child_by_field_name("module_name");
    let module_dotted = module_node.and_then(|m| {
        if m.kind() == "dotted_name" {
            Some(dotted_text(m, source))
        } else {
            None
        }
    });

    let mut star_cursor = node.walk();
    if node
        .children(&mut star_cursor)
        .any(|c| c.kind() == "wildcard_import")
    {
        collector.add_star(position);
        return;
    }

    let mut cursor = node.walk();
    for name_node in node.children_by_field_name("name", &mut cursor) {
        let local = match name_node.kind() {
            "aliased_import" => match name_node.child_by_field_name("alias") {
                Some(alias) if alias.kind() == "identifier" => node_text(alias, source).to_owned(),
                _ => continue,
            },
            "dotted_name" => match name_node.named_child(0) {
                Some(id) if id.kind() == "identifier" => node_text(id, source).to_owned(),
                _ => continue,
            },
            "identifier" => node_text(name_node, source).to_owned(),
            _ => continue,
        };
        let imported = match name_node.kind() {
            "aliased_import" => match name_node.child_by_field_name("name") {
                Some(orig) => dotted_text(orig, source),
                None => continue,
            },
            _ => local.clone(),
        };
        match (&module_dotted, conditional) {
            (Some(module), false) => {
                let canonical = format!("{module}.{imported}");
                collector.add_firm(
                    local,
                    ImportBinding {
                        canonical_path: canonical,
                        kind: BindingKind::FromImportSymbol,
                        position,
                    },
                );
            }
            // Relative module_name or a conditional import: fail closed (T-b/T-c).
            _ => collector.add_marker(local, position),
        }
    }
}

/// Record a firm binding (unconditional) or a positioned poison marker
/// (conditional / T-c) for `local`.
fn record(
    collector: &mut Collector,
    conditional: bool,
    local: String,
    canonical_path: String,
    kind: BindingKind,
    position: usize,
) {
    if conditional {
        collector.add_marker(local, position);
    } else {
        collector.add_firm(
            local,
            ImportBinding {
                canonical_path,
                kind,
                position,
            },
        );
    }
}

/// Join the `identifier` children of a `dotted_name` with `.` (e.g. `a.b`).
fn dotted_text(node: Node<'_>, source: &str) -> String {
    let mut parts = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            parts.push(node_text(child, source));
        }
    }
    if parts.is_empty() {
        node_text(node, source).to_owned()
    } else {
        parts.join(".")
    }
}

/// Text of a tree-sitter node.
fn node_text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}
