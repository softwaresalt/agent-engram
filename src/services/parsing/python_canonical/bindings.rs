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

use std::collections::{HashMap, HashSet};

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

/// The scope-resolved disposition of a call name at a source position (T2b).
///
/// `LocalImport` and `Poisoned` are decided entirely by the enclosing
/// **top-level** function scope (Anchor A); `ModuleScope` defers to the
/// module-scope accessors ([`ImportBindings::module_binding`] et al.) that
/// T5b/T5c layer the competing / star / RFS contests onto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallResolution<'a> {
    /// A firm function-local import is effective at the call site (the call
    /// occurs *after* the import; F1 order). Resolve to its canonical binding.
    LocalImport(&'a ImportBinding),
    /// The name is provably shadowed, rebound, conditionally imported, or used
    /// before its function-local import (fail closed, 013-D): drop the
    /// canonical target *and* suppress any name-only fallback for this call.
    Poisoned,
    /// Not decided by function scope (no enclosing scope, a `global`/`nonlocal`
    /// redirect, or an unbound local name): the caller resolves via the
    /// module-scope accessors.
    ModuleScope,
}

/// Module-scope import bindings, positioned fail-closed markers, and the
/// per-top-level-function scopes used for order-aware local resolution (T2b).
#[derive(Debug, Default, Clone)]
pub struct ImportBindings {
    module: HashMap<String, ImportBinding>,
    ambiguous: HashMap<String, usize>,
    competing: HashSet<String>,
    stars: Vec<usize>,
    functions: Vec<FunctionScope>,
    dynamic_rebinds: HashSet<String>,
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

    /// Whether `name` has *observed* competing firm bindings: two or more firm
    /// module-scope imports of the same local name, or a firm import plus a
    /// conditional/relative marker. Unlike a lone relative/conditional marker —
    /// which stays recall-safe (T5b.4) — observed competition must fail closed
    /// with no name-only fallback, since the sole indexed same-name symbol is
    /// not provably the effective (last-wins) binding (M1, 013-D).
    #[must_use]
    pub fn is_competing(&self, name: &str) -> bool {
        self.competing.contains(name)
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

    /// Resolve the call name `name` invoked at byte offset `call_position`
    /// against the enclosing **top-level** function scope (T2b). Function-local
    /// shadowing and source order (F1/Y1) are decided here; module-scope
    /// contests are deferred to the module accessors via
    /// [`CallResolution::ModuleScope`].
    #[must_use]
    pub fn resolve_call(&self, call_position: usize, name: &str) -> CallResolution<'_> {
        let Some(scope) = self.enclosing_scope(call_position) else {
            return self.module_scope_resolution(name);
        };
        // A `global`/`nonlocal`-declared name redirects to module scope.
        if scope.declared.contains(name) {
            return self.module_scope_resolution(name);
        }
        match scope.locals.get(name) {
            Some(LocalBinding::Import(binding)) => {
                if call_position > binding.position {
                    CallResolution::LocalImport(binding)
                } else {
                    // Call precedes its function-local import (UnboundLocalError):
                    // fail closed and do NOT fall through to module scope (F8).
                    CallResolution::Poisoned
                }
            }
            Some(LocalBinding::Poison) => CallResolution::Poisoned,
            None => self.module_scope_resolution(name),
        }
    }

    /// Whether `name` is module-scope poisoned by a dynamic rebind through a
    /// `global`/`nonlocal` write in some function (Anchor C, T-d).
    #[must_use]
    pub fn is_dynamically_rebound(&self, name: &str) -> bool {
        self.dynamic_rebinds.contains(name)
    }

    /// The innermost top-level function scope whose byte range contains
    /// `position`, if any. Top-level function ranges are disjoint.
    fn enclosing_scope(&self, position: usize) -> Option<&FunctionScope> {
        self.functions
            .iter()
            .find(|scope| position >= scope.start && position < scope.end)
    }

    /// Module-scope disposition for `name`: a dynamic rebind poisons it (T-d),
    /// otherwise the caller resolves via the module-scope accessors.
    fn module_scope_resolution(&self, name: &str) -> CallResolution<'_> {
        if self.dynamic_rebinds.contains(name) {
            CallResolution::Poisoned
        } else {
            CallResolution::ModuleScope
        }
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
    let mut functions = Vec::new();
    let mut dynamic_rebinds: HashSet<String> = HashSet::new();
    let root = tree.root_node();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "import_statement" => process_import(child, source, &mut collector, false),
            "import_from_statement" => process_import_from(child, source, &mut collector, false),
            "function_definition" => {
                let (scope, dynamic) = collect_function_scope(child, source);
                functions.push(scope);
                dynamic_rebinds.extend(dynamic);
            }
            "decorated_definition" => {
                if let Some(defn) = child.child_by_field_name("definition") {
                    if defn.kind() == "function_definition" {
                        let (scope, dynamic) = collect_function_scope(defn, source);
                        functions.push(scope);
                        dynamic_rebinds.extend(dynamic);
                    }
                }
            }
            // Class bodies contain no extractor-reachable callers (Anchor A).
            "class_definition" => {}
            // Module-level control flow: nested imports are conditional (T-c).
            _ => collect_conditional(child, source, &mut collector),
        }
    }

    let mut bindings = collector.finalize();
    bindings.functions = functions;
    bindings.dynamic_rebinds = dynamic_rebinds;
    bindings
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
        let mut competing: HashSet<String> = HashSet::new();

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
                // Observed competition: 2+ firm candidates, or a firm binding
                // plus a conditional/relative marker of the same name. The
                // effective target is undecidable, so suppress the name-only
                // fallback in addition to the module-scope binding (M1, 013-D).
                let pos = marker_pos.map_or(min_pos, |mp| mp.min(min_pos));
                ambiguous.insert(name.clone(), pos);
                competing.insert(name);
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
            competing,
            stars,
            ..Default::default()
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

// ---------------------------------------------------------------------------
// T2b — Scope-aware binding isolation (096.009-T)
// ---------------------------------------------------------------------------

/// A firm function-local import (order-aware) or an unconditional poison of a
/// function-local name (shadowed / rebound / conditionally imported).
#[derive(Debug, Clone)]
enum LocalBinding {
    Import(ImportBinding),
    Poison,
}

/// The binding scope of one top-level function body (Anchor A: reachable
/// callers are top-level function bodies; the applicable scope chain is
/// `{function-local -> module}`).
#[derive(Debug, Clone)]
struct FunctionScope {
    start: usize,
    end: usize,
    /// `global` / `nonlocal`-declared names — reads redirect to module scope.
    declared: HashSet<String>,
    /// Names bound function-locally (not `global`/`nonlocal`).
    locals: HashMap<String, LocalBinding>,
}

/// A single binding produced by an import statement, before scope placement.
enum ImportEvent {
    Bind {
        local: String,
        canonical: String,
        kind: BindingKind,
    },
    /// A relative-import name (unresolvable canonical — fail closed).
    RelativeMarker { local: String },
    /// A `from N import *` wildcard (binds unknown names).
    Star,
}

/// Accumulates a function body's binders before fail-closed finalization.
#[derive(Default)]
struct ScopeBuilder {
    /// `global` / `nonlocal`-declared names.
    declared: HashSet<String>,
    /// Local name -> its import candidates, each flagged conditional (branchy).
    imports: HashMap<String, Vec<(ImportBinding, bool)>>,
    /// Non-import binders (parameter / assignment / for / with / except /
    /// walrus / del / nested def or class name) — any of these poisons the name.
    binders: HashSet<String>,
}

impl ScopeBuilder {
    /// Collapse the accumulated binders into a [`FunctionScope`] plus the list
    /// of names dynamically rebound through a `global`/`nonlocal` write (T-d).
    /// A name is a firm function-local import only when a single unconditional
    /// import binds it and no other binder competes (reuses the M1 rule).
    fn finalize(self, start: usize, end: usize) -> (FunctionScope, Vec<String>) {
        let ScopeBuilder {
            declared,
            imports,
            binders,
        } = self;
        let mut locals: HashMap<String, LocalBinding> = HashMap::new();
        let mut dynamic = Vec::new();

        for (name, candidates) in imports {
            if declared.contains(&name) {
                // A module rebind through a global/nonlocal write (T-d).
                dynamic.push(name);
                continue;
            }
            let firm = !binders.contains(&name) && candidates.len() == 1 && !candidates[0].1;
            if firm {
                if let Some((binding, _)) = candidates.into_iter().next() {
                    locals.insert(name, LocalBinding::Import(binding));
                }
            } else {
                locals.insert(name, LocalBinding::Poison);
            }
        }

        for name in binders {
            if declared.contains(&name) {
                dynamic.push(name);
            } else {
                locals.entry(name).or_insert(LocalBinding::Poison);
            }
        }

        (
            FunctionScope {
                start,
                end,
                declared,
                locals,
            },
            dynamic,
        )
    }
}

/// Build the binding scope of a top-level function (Anchor A). Parameters and
/// body binders poison their names; imports directly in the body block are
/// unconditional while imports nested under control flow are conditional (fail
/// closed). Nested `def`/`class`/`lambda` bodies are isolated (F5) — only their
/// bound name registers here.
fn collect_function_scope(func: Node<'_>, source: &str) -> (FunctionScope, Vec<String>) {
    let mut builder = ScopeBuilder::default();
    if let Some(params) = func.child_by_field_name("parameters") {
        collect_param_names(params, source, &mut builder.binders);
    }
    if let Some(body) = func.child_by_field_name("body") {
        scan_scope(body, source, false, &mut builder);
    }
    builder.finalize(func.start_byte(), func.end_byte())
}

/// Recursively scan a function body for binders. `conditional` is true when the
/// current node sits under control flow (imports here are not proven to execute
/// in order). Recursion stops at nested `def`/`class`/`lambda` (isolation, F5).
fn scan_scope(node: Node<'_>, source: &str, conditional: bool, b: &mut ScopeBuilder) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "import_statement" | "import_from_statement" => {
                add_import_candidates(child, source, conditional, b);
            }
            "global_statement" | "nonlocal_statement" => add_declared(child, source, b),
            "function_definition" | "class_definition" => add_definition_name(child, source, b),
            "decorated_definition" => {
                if let Some(defn) = child.child_by_field_name("definition") {
                    add_definition_name(defn, source, b);
                }
            }
            // Lambdas own their scope; their body binders never leak here.
            "lambda" => {}
            "assignment" | "augmented_assignment" => {
                if let Some(left) = child.child_by_field_name("left") {
                    collect_target_names(left, source, &mut b.binders);
                }
                scan_scope(child, source, conditional, b);
            }
            "named_expression" => {
                if let Some(name) = child.child_by_field_name("name") {
                    if name.kind() == "identifier" {
                        b.binders.insert(node_text(name, source).to_owned());
                    }
                }
                scan_scope(child, source, conditional, b);
            }
            "for_statement" => {
                if let Some(left) = child.child_by_field_name("left") {
                    collect_target_names(left, source, &mut b.binders);
                }
                scan_scope(child, source, true, b);
            }
            "with_statement" => {
                add_with_targets(child, source, b);
                scan_scope(child, source, true, b);
            }
            "except_clause" | "except_group_clause" => {
                add_except_target(child, source, b);
                scan_scope(child, source, true, b);
            }
            "delete_statement" => {
                let mut inner = child.walk();
                for id in child.children(&mut inner) {
                    if id.kind() == "identifier" {
                        b.binders.insert(node_text(id, source).to_owned());
                    }
                }
            }
            "if_statement" | "elif_clause" | "else_clause" | "while_statement"
            | "try_statement" | "finally_clause" | "match_statement" | "case_clause" => {
                scan_scope(child, source, true, b);
            }
            _ => scan_scope(child, source, conditional, b),
        }
    }
}

/// Record the local name bound by a nested `def`/`class` without recursing into
/// its body (isolation, F5).
fn add_definition_name(defn: Node<'_>, source: &str, b: &mut ScopeBuilder) {
    if let Some(name) = defn.child_by_field_name("name") {
        if name.kind() == "identifier" {
            b.binders.insert(node_text(name, source).to_owned());
        }
    }
}

/// Record a `global`/`nonlocal` declaration's identifier children.
fn add_declared(node: Node<'_>, source: &str, b: &mut ScopeBuilder) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            b.declared.insert(node_text(child, source).to_owned());
        }
    }
}

/// Extend `b` with the function-local import candidates of an import statement.
/// A relative or star import binds unresolvable names, so it poisons them.
fn add_import_candidates(node: Node<'_>, source: &str, conditional: bool, b: &mut ScopeBuilder) {
    let (events, position) = import_events(node, source);
    for event in events {
        match event {
            ImportEvent::Bind {
                local,
                canonical,
                kind,
            } => {
                b.imports.entry(local).or_default().push((
                    ImportBinding {
                        canonical_path: canonical,
                        kind,
                        position,
                    },
                    conditional,
                ));
            }
            ImportEvent::RelativeMarker { local } => {
                b.binders.insert(local);
            }
            // `from N import *` inside a function binds unknown names; it cannot
            // occur in valid Python 3 (SyntaxError), so nothing is recorded.
            ImportEvent::Star => {}
        }
    }
}

/// Record the alias targets of a `with` clause (`with a() as t:` binds `t`).
fn add_with_targets(node: Node<'_>, source: &str, b: &mut ScopeBuilder) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "with_clause" {
            collect_as_pattern_targets(child, source, &mut b.binders);
        }
    }
}

/// Record the alias target of an `except ... as e:` clause header.
fn add_except_target(node: Node<'_>, source: &str, b: &mut ScopeBuilder) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "block" {
            collect_as_pattern_targets(child, source, &mut b.binders);
        }
    }
}

/// Collect the bare-identifier targets under any `as_pattern_target` in a
/// `with`/`except` header (skips attribute/subscript targets).
fn collect_as_pattern_targets(node: Node<'_>, source: &str, set: &mut HashSet<String>) {
    if node.kind() == "as_pattern_target" {
        collect_target_names(node, source, set);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_as_pattern_targets(child, source, set);
    }
}

/// Collect the bare-identifier names bound by an assignment/`for` target,
/// recursing through tuple/list destructuring but skipping attribute and
/// subscript targets (they do not bind a bare name).
fn collect_target_names(node: Node<'_>, source: &str, set: &mut HashSet<String>) {
    match node.kind() {
        "identifier" => {
            set.insert(node_text(node, source).to_owned());
        }
        "attribute" | "subscript" | "call" => {}
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_target_names(child, source, set);
            }
        }
    }
}

/// Collect the parameter names of a `parameters` node as binders (a parameter
/// shadows any module function of the same name — a precision floor).
fn collect_param_names(params: Node<'_>, source: &str, set: &mut HashSet<String>) {
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                set.insert(node_text(child, source).to_owned());
            }
            "typed_parameter" | "list_splat_pattern" | "dictionary_splat_pattern" => {
                if let Some(id) = child.named_child(0) {
                    if id.kind() == "identifier" {
                        set.insert(node_text(id, source).to_owned());
                    }
                }
            }
            "default_parameter" | "typed_default_parameter" => {
                if let Some(name) = child.child_by_field_name("name") {
                    if name.kind() == "identifier" {
                        set.insert(node_text(name, source).to_owned());
                    }
                }
            }
            _ => {}
        }
    }
}

/// Extract the binding events and source position of an `import_statement` /
/// `import_from_statement` (grammar-driven; scope/conditional handling is the
/// caller's). Mirrors the module-scope navigation in [`process_import`] /
/// [`process_import_from`] for function-local reuse.
fn import_events(node: Node<'_>, source: &str) -> (Vec<ImportEvent>, usize) {
    let position = node.start_byte();
    let mut events = Vec::new();
    match node.kind() {
        "import_statement" => {
            let mut cursor = node.walk();
            for name_node in node.children_by_field_name("name", &mut cursor) {
                match name_node.kind() {
                    "aliased_import" => {
                        if let (Some(alias), Some(module)) = (
                            name_node.child_by_field_name("alias"),
                            name_node.child_by_field_name("name"),
                        ) {
                            if alias.kind() == "identifier" {
                                events.push(ImportEvent::Bind {
                                    local: node_text(alias, source).to_owned(),
                                    canonical: dotted_text(module, source),
                                    kind: BindingKind::ModuleImport,
                                });
                            }
                        }
                    }
                    "dotted_name" => {
                        if let Some(root) = name_node.named_child(0) {
                            if root.kind() == "identifier" {
                                let local = node_text(root, source).to_owned();
                                events.push(ImportEvent::Bind {
                                    canonical: local.clone(),
                                    local,
                                    kind: BindingKind::ModuleImport,
                                });
                            }
                        }
                    }
                    "identifier" => {
                        let local = node_text(name_node, source).to_owned();
                        events.push(ImportEvent::Bind {
                            canonical: local.clone(),
                            local,
                            kind: BindingKind::ModuleImport,
                        });
                    }
                    _ => {}
                }
            }
        }
        "import_from_statement" => {
            let module_dotted = node
                .child_by_field_name("module_name")
                .and_then(|m| (m.kind() == "dotted_name").then(|| dotted_text(m, source)));

            let mut star_cursor = node.walk();
            if node
                .children(&mut star_cursor)
                .any(|c| c.kind() == "wildcard_import")
            {
                events.push(ImportEvent::Star);
                return (events, position);
            }

            let mut cursor = node.walk();
            for name_node in node.children_by_field_name("name", &mut cursor) {
                let (local, imported) = match name_node.kind() {
                    "aliased_import" => match (
                        name_node.child_by_field_name("alias"),
                        name_node.child_by_field_name("name"),
                    ) {
                        (Some(alias), Some(orig)) if alias.kind() == "identifier" => (
                            node_text(alias, source).to_owned(),
                            dotted_text(orig, source),
                        ),
                        _ => continue,
                    },
                    "dotted_name" => match name_node.named_child(0) {
                        Some(id) if id.kind() == "identifier" => {
                            let text = node_text(id, source).to_owned();
                            (text.clone(), text)
                        }
                        _ => continue,
                    },
                    "identifier" => {
                        let text = node_text(name_node, source).to_owned();
                        (text.clone(), text)
                    }
                    _ => continue,
                };
                match &module_dotted {
                    Some(module) => events.push(ImportEvent::Bind {
                        local,
                        canonical: format!("{module}.{imported}"),
                        kind: BindingKind::FromImportSymbol,
                    }),
                    None => events.push(ImportEvent::RelativeMarker { local }),
                }
            }
        }
        _ => {}
    }
    (events, position)
}
