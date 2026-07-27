//! Code graph indexing orchestration service.
//!
//! Coordinates file discovery, parallel parsing, tiered embedding,
//! incremental sync, and concerns edge management.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};
use tree_sitter::{Node, Parser};
use uuid::Uuid;

use crate::db::connect_db;
use crate::db::queries::{
    CodeGraphQueries, NoCanonicalTargetReason, ReresolveResult, StagedCallProvenanceRecord,
};
use crate::errors::EngramError;
use crate::models::code_file::CodeFile;
use crate::models::config::CodeGraphConfig;
use crate::services::embedding;
use crate::services::parsing::canonical;
use crate::services::parsing::python_canonical::{
    BindingKind, ImportBindings, extract_python_import_bindings, python_module_path_for_file,
};
use crate::services::parsing::{ExtractedEdge, ExtractedSymbol, Language, parse_source};

type RustCanonicalContext = (canonical::ModulePath, canonical::UseGraph);

/// Cached Rust canonical context produced by the global unsafe-module pre-pass.
#[derive(Debug, Clone)]
struct CachedRustCanonicalContext {
    content_hash: String,
    context: Option<RustCanonicalContext>,
}

/// Global unsafe-module pre-pass output for a single index or sync run.
#[derive(Debug, Default)]
pub(crate) struct UnsafeModulePrepass {
    unsafe_prefixes: HashSet<String>,
    rust_contexts: HashMap<String, CachedRustCanonicalContext>,
}

/// Build the per-file canonical resolution context for a Rust source file, or
/// `None` for non-Rust files and for layouts whose module path is not
/// deterministically derivable (fail-closed → empty `canonical_path`).
///
/// Part of Option C Unit A / A6 (precision-neutral): produces identity data
/// only; no call edges are emitted.
fn rust_canonical_ctx(
    crates: &canonical::WorkspaceCrates,
    lang: Language,
    rel_path: &str,
    source: &str,
) -> Option<RustCanonicalContext> {
    if lang != Language::Rust {
        return None;
    }
    let module = canonical::module_path_for_file(crates, rel_path)?;
    Some((module, canonical::extract_use_graph(source)))
}

/// Collect module prefixes whose default filesystem identity is unsafe because
/// a top-level `mod` declaration remaps or conditionally gates that module.
pub(crate) async fn unsafe_module_prepass(
    ws_path: &Path,
    crates: &canonical::WorkspaceCrates,
    files: &[std::path::PathBuf],
) -> UnsafeModulePrepass {
    let mut prepass = UnsafeModulePrepass::default();
    for file_path in files {
        if language_from_path(file_path) != "rust" {
            continue;
        }
        let Ok(rel) = file_path.strip_prefix(ws_path) else {
            continue;
        };
        let rel_path = rel.to_string_lossy().replace('\\', "/");
        let Ok(source) = tokio::fs::read_to_string(file_path).await else {
            continue;
        };
        let content_hash = sha256_hex(&source);
        let context = rust_canonical_ctx(crates, Language::Rust, &rel_path, &source);
        if let Some((module, use_graph)) = &context {
            prepass
                .unsafe_prefixes
                .extend(use_graph.non_default_mod_roots().iter().map(|root| {
                    // `root` may be a nested relative path (`outer::inner`) captured from
                    // a `#[path]`/`#[cfg]` remap inside an inline module body, so descend
                    // it segment-by-segment: the unsafe prefix is the full nested module.
                    root.split("::")
                        .fold(module.clone(), |m, seg| m.child(seg))
                        .to_canonical()
                }));
        }
        prepass.rust_contexts.insert(
            rel_path,
            CachedRustCanonicalContext {
                content_hash,
                context,
            },
        );
    }
    prepass
}

/// Return the pre-pass-cached Rust canonical context on a content-hash match,
/// otherwise recompute it.
///
/// On a hash mismatch this recomputes only the file's per-file context
/// (`ModulePath` / `UseGraph`); it intentionally does not refresh the global
/// `unsafe_prefixes`, which are a snapshot taken during `unsafe_module_prepass`.
/// This keeps canonical edge output byte-identical to the pre-cache baseline
/// (the load-bearing invariant): on a match the context and prefixes come from
/// one snapshot; on a mismatch the recompute reproduces exactly what the main
/// pass computed before context caching existed. The pre-pass/main-pass
/// `TOCTOU` on cross-file unsafe-prefix discovery — a file gaining a `#[path]`
/// or `#[cfg] mod` remap between the two reads — is a pre-existing property of
/// the two-phase pre-pass architecture, unchanged by this cache; closing it
/// would require a single-snapshot rebuild and is out of scope here.
fn rust_ctx_from_prepass_cache(
    rust_contexts: &HashMap<String, CachedRustCanonicalContext>,
    rel_path: &str,
    content_hash: &str,
    force_cache_miss: bool,
    compute: impl FnOnce() -> Option<RustCanonicalContext>,
) -> Option<RustCanonicalContext> {
    if !force_cache_miss {
        if let Some(entry) = rust_contexts
            .get(rel_path)
            .filter(|entry| entry.content_hash == content_hash)
        {
            return entry.context.clone();
        }
    }
    compute()
}

pub(crate) fn is_under_unsafe_module_prefix(path: &str, unsafe_prefixes: &HashSet<String>) -> bool {
    unsafe_prefixes.iter().any(|prefix| {
        path == prefix
            || path
                .strip_prefix(prefix)
                .is_some_and(|suffix| suffix.starts_with("::"))
    })
}

/// Resolve a parsed function/method's additive `canonical_path`, or `""` when it
/// cannot be resolved (never a canonical match target — D4).
///
/// Dispatches on `language`: Rust uses the module/use-graph canonical resolver;
/// Python (096-F/T3) uses its module-namespace path `"<module>.<name>"` when the
/// file sits on a provable regular-package chain (`python_module` is `Some`),
/// else `""` (fail closed). Other languages carry no canonical identity yet.
fn canonical_path_for_function(
    language: Language,
    crates: &canonical::WorkspaceCrates,
    rust_ctx: Option<&(canonical::ModulePath, canonical::UseGraph)>,
    python_module: Option<&str>,
    unsafe_prefixes: &HashSet<String>,
    name: &str,
) -> String {
    match language {
        Language::Rust => rust_canonical_path_for_function(crates, rust_ctx, unsafe_prefixes, name),
        Language::Python => python_module
            .map(|module| format!("{module}.{name}"))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// Rust canonical-path resolution via the module + use-graph resolver, or `""`
/// when the layout is not deterministically resolvable or resolves under an
/// unsafe module prefix (fail-closed — never a canonical match target, D4).
fn rust_canonical_path_for_function(
    crates: &canonical::WorkspaceCrates,
    ctx: Option<&(canonical::ModulePath, canonical::UseGraph)>,
    unsafe_prefixes: &HashSet<String>,
    name: &str,
) -> String {
    let Some((module, use_graph)) = ctx else {
        return String::new();
    };
    let rctx = canonical::ResolveContext {
        module,
        crates,
        use_graph,
    };
    let Some(path) =
        canonical::canonical_path_for_def(&rctx, name).map(canonical::CanonicalId::into_string)
    else {
        return String::new();
    };
    if is_under_unsafe_module_prefix(&path, unsafe_prefixes) {
        String::new()
    } else {
        path
    }
}

/// Derive the workspace-relative directories that are provable Python **regular
/// packages** — i.e. contain an `__init__.py` — from the discovered file list.
/// Returned as forward-slash `/`-joined paths (root package → `""`), matching
/// the ancestor-dir spelling `python_module_path_for_file` queries.
fn python_package_dirs(files: &[std::path::PathBuf], ws_path: &Path) -> HashSet<String> {
    let mut set = HashSet::new();
    for f in files {
        let Ok(rel) = f.strip_prefix(ws_path) else {
            continue;
        };
        let norm = rel.to_string_lossy().replace('\\', "/");
        if let Some(dir) = norm.strip_suffix("/__init__.py") {
            set.insert(dir.to_owned());
        } else if norm == "__init__.py" {
            set.insert(String::new());
        }
    }
    set
}

/// Whether `rel` names a Python source file by extension (case-insensitive).
fn is_py_path(rel: &str) -> bool {
    std::path::Path::new(rel)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("py"))
}

/// Whether the workspace-relative path `rel` is a `.py` file nested under the
/// package directory `dir`. Used to force-recompute descendant canonical paths
/// when `dir`'s regular-package status flips (C6-1). A root package (`dir ==
/// ""`) contains every top-level `.py` file.
fn is_python_descendant(rel: &str, dir: &str) -> bool {
    if !is_py_path(rel) {
        return false;
    }
    if dir.is_empty() {
        return true;
    }
    rel.strip_prefix(dir)
        .is_some_and(|rest| rest.starts_with('/'))
}

/// Resolve the canonical enclosing type for an impl-method caller.
fn enclosing_canonical_type_for_function(
    crates: &canonical::WorkspaceCrates,
    ctx: Option<&(canonical::ModulePath, canonical::UseGraph)>,
    unsafe_prefixes: &HashSet<String>,
    name: &str,
) -> String {
    if !name.contains("::") {
        return String::new();
    }
    rust_canonical_path_for_function(crates, ctx, unsafe_prefixes, name)
        .rsplit_once("::")
        .map(|(ty, _)| ty.to_owned())
        .unwrap_or_default()
}

/// Whether a method/qualified call is allowed into Unit-B staging.
fn should_stage_provenance_call(is_method: bool, is_qualified: bool, raw_qualifier: &str) -> bool {
    if is_method {
        raw_qualifier == "self"
    } else {
        is_qualified
    }
}

/// Shared per-file Python lexical-shadow scan used by T5a's coarse contest
/// gate and reusable by T5c's future order-aware winner selection.
struct PythonShadowIndex {
    imports: ImportBindings,
    module_binding_counts: HashMap<String, usize>,
    function_locals: HashMap<String, HashSet<String>>,
    /// Module-scope function-definition byte positions grouped by name.
    module_defs: HashMap<String, Vec<usize>>,
    /// Module-scope opaque rebind byte positions grouped by name.
    module_rebinds: HashMap<String, Vec<usize>>,
}

impl PythonShadowIndex {
    /// Build import signals, module binding counts, and top-level function-local
    /// binding sets with one amortized Python parse for the file.
    fn build(source: &str) -> Self {
        let mut index = Self {
            imports: extract_python_import_bindings(source),
            module_binding_counts: HashMap::new(),
            function_locals: HashMap::new(),
            module_defs: HashMap::new(),
            module_rebinds: HashMap::new(),
        };
        let mut parser = Parser::new();
        if parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .is_err()
        {
            return index;
        }
        let Some(tree) = parser.parse(source, None) else {
            return index;
        };
        scan_python_module_scope(tree.root_node(), source, &mut index);
        index
    }

    /// Return whether any import, module rebind, or caller-local binding contests
    /// the matched same-file definition for `callee_name`.
    fn is_contested(&self, callee_name: &str, caller_fn_name: &str) -> bool {
        self.imports.module_binding(callee_name).is_some()
            || self.imports.is_ambiguous(callee_name)
            || !self.imports.star_invalidators().is_empty()
            || self.imports.is_dynamically_rebound(callee_name)
            || self
                .module_binding_counts
                .get(callee_name)
                .is_some_and(|count| *count > 1)
            || self
                .function_locals
                .get(caller_fn_name)
                .is_some_and(|locals| locals.contains(callee_name))
    }
}

/// Walk module lexical scope through compound statements while stopping at
/// function, class, and lambda bodies.
fn scan_python_module_scope(root: Node<'_>, source: &str, index: &mut PythonShadowIndex) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        scan_python_module_node(child, source, index);
    }
}

/// Record one module-scope node's bindings and recurse only where Python scope
/// rules keep the descendants in module scope.
fn scan_python_module_node(node: Node<'_>, source: &str, index: &mut PythonShadowIndex) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = python_definition_name(node, source) {
                increment_python_binding(&mut index.module_binding_counts, &name);
                index
                    .module_defs
                    .entry(name.clone())
                    .or_default()
                    .push(node.start_byte());
                let locals = collect_python_function_locals(node, source);
                index
                    .function_locals
                    .entry(name)
                    .or_default()
                    .extend(locals);
            }
            return;
        }
        "class_definition" => {
            if let Some(name) = python_definition_name(node, source) {
                increment_python_binding(&mut index.module_binding_counts, &name);
                index
                    .module_rebinds
                    .entry(name)
                    .or_default()
                    .push(node.start_byte());
            }
            return;
        }
        "decorated_definition" => {
            if let Some(definition) = node.child_by_field_name("definition") {
                scan_python_module_node(definition, source, index);
            }
            return;
        }
        "lambda" | "import_statement" | "import_from_statement" => return,
        _ => {
            for name in collect_python_node_rebinds(node, source) {
                increment_python_binding(&mut index.module_binding_counts, &name);
                index
                    .module_rebinds
                    .entry(name)
                    .or_default()
                    .push(node.start_byte());
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        scan_python_module_node(child, source, index);
    }
}

/// Collect every local name bound by a top-level function's parameters or body,
/// stopping at nested callable and class bodies after recording their names.
fn collect_python_function_locals(function: Node<'_>, source: &str) -> HashSet<String> {
    let mut locals = HashSet::new();
    if let Some(parameters) = function.child_by_field_name("parameters") {
        collect_python_parameter_names(parameters, source, &mut locals);
    }
    if let Some(body) = function.child_by_field_name("body") {
        scan_python_function_scope(body, source, &mut locals);
    }
    locals
}

/// Walk one function body scope, including local imports and rebind forms but
/// excluding nested function, class, and lambda bodies.
fn scan_python_function_scope(node: Node<'_>, source: &str, locals: &mut HashSet<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_definition" | "class_definition" => {
                if let Some(name) = python_definition_name(child, source) {
                    locals.insert(name);
                }
            }
            "decorated_definition" => {
                if let Some(definition) = child.child_by_field_name("definition") {
                    if let Some(name) = python_definition_name(definition, source) {
                        locals.insert(name);
                    }
                }
            }
            "lambda" => {}
            "import_statement" | "import_from_statement" => {
                collect_python_import_names(child, source, locals);
            }
            _ => {
                collect_python_node_bindings_into_set(child, source, locals);
                scan_python_function_scope(child, source, locals);
            }
        }
    }
}

/// Return the opaque module rebind targets introduced directly by `node`.
fn collect_python_node_rebinds(node: Node<'_>, source: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    collect_python_node_bindings_into_set(node, source, &mut names);
    names
}

/// Add the binding targets introduced directly by `node` to a lexical scope.
fn collect_python_node_bindings_into_set(
    node: Node<'_>,
    source: &str,
    names: &mut HashSet<String>,
) {
    match node.kind() {
        "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => {
            if let Some(left) = node.child_by_field_name("left") {
                collect_python_target_names(left, source, names);
            }
        }
        "with_statement" | "except_clause" | "except_group_clause" => {
            collect_python_as_aliases(node, source, names);
        }
        "delete_statement" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_python_target_names(child, source, names);
            }
        }
        "named_expression" => {
            if let Some(name) = node.child_by_field_name("name") {
                collect_python_target_names(name, source, names);
            }
        }
        "case_clause" => collect_python_case_captures(node, source, names),
        _ => {}
    }
}

/// Increment a module-scope binding count for one name.
fn increment_python_binding(counts: &mut HashMap<String, usize>, name: &str) {
    *counts.entry(name.to_owned()).or_default() += 1;
}

/// Return a function or class definition's simple name.
fn python_definition_name(node: Node<'_>, source: &str) -> Option<String> {
    node.child_by_field_name("name")
        .filter(|name| name.kind() == "identifier")
        .map(|name| python_node_text(name, source).to_owned())
}

/// Collect parameter names without treating annotation identifiers as binders.
fn collect_python_parameter_names(parameters: Node<'_>, source: &str, names: &mut HashSet<String>) {
    let mut cursor = parameters.walk();
    for child in parameters.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                names.insert(python_node_text(child, source).to_owned());
            }
            "typed_parameter" | "list_splat_pattern" | "dictionary_splat_pattern" => {
                if let Some(identifier) = child.named_child(0) {
                    if identifier.kind() == "identifier" {
                        names.insert(python_node_text(identifier, source).to_owned());
                    }
                }
            }
            "default_parameter" | "typed_default_parameter" => {
                if let Some(name) = child.child_by_field_name("name") {
                    collect_python_target_names(name, source, names);
                }
            }
            _ => {}
        }
    }
}

/// Collect names bound by a function-local import statement.
fn collect_python_import_names(node: Node<'_>, source: &str, names: &mut HashSet<String>) {
    let mut cursor = node.walk();
    for imported in node.children_by_field_name("name", &mut cursor) {
        match imported.kind() {
            "aliased_import" => {
                if let Some(alias) = imported.child_by_field_name("alias") {
                    collect_python_target_names(alias, source, names);
                }
            }
            "dotted_name" => {
                if let Some(root) = imported.named_child(0) {
                    collect_python_target_names(root, source, names);
                }
            }
            "identifier" => {
                names.insert(python_node_text(imported, source).to_owned());
            }
            _ => {}
        }
    }
}

/// Collect alias identifiers from `as_pattern` descendants of `with`/`except`.
fn collect_python_as_aliases(node: Node<'_>, source: &str, names: &mut HashSet<String>) {
    if node.kind() == "as_pattern" {
        if let Some(alias) = node.child_by_field_name("alias") {
            collect_python_target_names(alias, source, names);
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "block" {
            collect_python_as_aliases(child, source, names);
        }
    }
}

/// Conservatively collect identifiers in a match-case pattern before its body.
fn collect_python_case_captures(node: Node<'_>, source: &str, names: &mut HashSet<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            break;
        }
        collect_python_target_names(child, source, names);
    }
}

/// Collect simple-name binding targets while excluding attribute, subscript,
/// and call expressions that do not bind a local name.
fn collect_python_target_names(node: Node<'_>, source: &str, names: &mut HashSet<String>) {
    match node.kind() {
        "identifier" => {
            names.insert(python_node_text(node, source).to_owned());
        }
        "attribute" | "subscript" | "call" => {}
        "as_pattern" => {
            if let Some(alias) = node.child_by_field_name("alias") {
                collect_python_target_names(alias, source, names);
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_python_target_names(child, source, names);
            }
        }
    }
}

/// Return a node's UTF-8 source slice, or an empty string for an invalid range.
fn python_node_text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    source.get(node.byte_range()).unwrap_or_default()
}

/// Resolve a staged Unit-B call to a canonical target string, or fail closed.
fn canonical_target_for_staged_call(
    crates: &canonical::WorkspaceCrates,
    module: &canonical::ModulePath,
    use_graph: &canonical::UseGraph,
    unsafe_prefixes: &HashSet<String>,
    call: &StagedCallProvenanceRecord,
) -> Option<String> {
    if use_graph.has_nested_use() || use_graph.has_non_default_mod_mapping() {
        return None;
    }
    let ctx = canonical::ResolveContext {
        module,
        crates,
        use_graph,
    };
    let callee = call.callee_name.as_str();
    let target = match call.qualifier_kind.as_str() {
        "module" | "type" => canonical::resolve_qualifier(
            &ctx,
            None,
            &canonical::Qualifier::Path(call.raw_qualifier.clone()),
            &[callee],
        ),
        "self" if call.raw_qualifier == "Self" => {
            let enclosing = non_empty_str(&call.enclosing_canonical_type)?;
            canonical::resolve_qualifier(
                &ctx,
                Some(enclosing),
                &canonical::Qualifier::SelfType,
                &[callee],
            )
        }
        "method" if call.raw_qualifier == "self" => {
            let enclosing = non_empty_str(&call.enclosing_canonical_type)?;
            canonical::resolve_qualifier(
                &ctx,
                Some(enclosing),
                &canonical::Qualifier::SelfType,
                &[callee],
            )
        }
        _ => None,
    }?;
    let path = target.into_string();
    if is_under_unsafe_module_prefix(&path, unsafe_prefixes) {
        None
    } else {
        Some(path)
    }
}

fn non_empty_str(value: &str) -> Option<&str> {
    if value.is_empty() { None } else { Some(value) }
}

async fn rust_ctx_for_staged_file(
    ws_path: &Path,
    crates: &canonical::WorkspaceCrates,
    rel_path: &str,
) -> Option<(canonical::ModulePath, canonical::UseGraph)> {
    let full = ws_path.join(rel_path);
    let source = tokio::fs::read_to_string(full).await.ok()?;
    rust_canonical_ctx(crates, Language::Rust, rel_path, &source)
}

/// Per-file Python staged-call context: the caller module path (if derivable),
/// the lexical-shadow index, and the caller function-id → simple-name map used
/// by the T5c shadow guard.
type PythonStagedContext = (Option<String>, PythonShadowIndex, HashMap<String, String>);

/// Build Python module, shadow, and caller-name context for one staged-call
/// source file. The caller-name map (function id → simple name) lets the
/// resolver consult the calling function's lexical scope and definition
/// position (T5c shadow guard) without a per-call database query.
async fn python_ctx_for_staged_file(
    ws_path: &Path,
    python_packages: &HashSet<String>,
    queries: &CodeGraphQueries,
    rel_path: &str,
) -> Option<PythonStagedContext> {
    let full = ws_path.join(rel_path);
    let source = tokio::fs::read_to_string(full).await.ok()?;
    let is_regular_package =
        |p: &Path| python_packages.contains(&p.to_string_lossy().replace('\\', "/"));
    let module_path = python_module_path_for_file(rel_path, &is_regular_package);
    let caller_names = queries
        .get_functions_by_file(rel_path)
        .await
        .ok()?
        .into_iter()
        .map(|function| (function.id, function.name))
        .collect();
    Some((module_path, PythonShadowIndex::build(&source), caller_names))
}

/// The smallest module-scope definition position of `caller_name`, or
/// `usize::MAX` when it is not a top-level definition (nested / method callers
/// are a v1 non-goal, so their module-order winner is left unconstrained).
fn python_caller_position(shadow: &PythonShadowIndex, caller_name: Option<&str>) -> usize {
    caller_name
        .and_then(|name| shadow.module_defs.get(name))
        .and_then(|positions| positions.iter().min().copied())
        .unwrap_or(usize::MAX)
}

/// Whether `name` is bound anywhere in the calling function's body (T-a): any
/// in-function binder makes the name function-local for the whole body, so a
/// call to it can never resolve to a module-scope import or def (fail closed).
fn python_name_is_function_local(
    shadow: &PythonShadowIndex,
    caller_name: Option<&str>,
    name: &str,
) -> bool {
    caller_name
        .and_then(|caller| shadow.function_locals.get(caller))
        .is_some_and(|locals| locals.contains(name))
}

/// Whether the module receiver `name` is rebound by any later module-scope def,
/// opaque rebind, star import, or ambiguous import (order-aware: a rebind after
/// the import proves the receiver is no longer that module → fail closed).
fn python_receiver_rebound_after(
    shadow: &PythonShadowIndex,
    name: &str,
    import_position: usize,
) -> bool {
    let mut positions: Vec<usize> = Vec::new();
    positions.extend(shadow.module_defs.get(name).into_iter().flatten().copied());
    positions.extend(
        shadow
            .module_rebinds
            .get(name)
            .into_iter()
            .flatten()
            .copied(),
    );
    positions.extend(shadow.imports.star_invalidators().iter().copied());
    if shadow.imports.is_ambiguous(name) {
        if let Some(position) = shadow.imports.ambiguity_position(name) {
            positions.push(position);
        }
    }
    positions.iter().any(|position| *position > import_position)
}

/// Resolve a bare `callee()` call to its exact canonical target, order-aware
/// over module source positions and anchored on the caller's definition point.
fn python_bare_target(
    module: &str,
    shadow: &PythonShadowIndex,
    caller_name: Option<&str>,
    callee: &str,
) -> Result<String, NoCanonicalTargetReason> {
    let imports = &shadow.imports;
    // T-d dynamic write and T-a function-local binder both fail closed.
    if imports.is_dynamically_rebound(callee)
        || python_name_is_function_local(shadow, caller_name, callee)
    {
        return Err(NoCanonicalTargetReason::Shadowed);
    }
    // A module receiver used as a bare callee is not a function.
    if let Some(binding) = imports.module_binding(callee) {
        if binding.kind == BindingKind::ModuleImport {
            return Err(NoCanonicalTargetReason::CompetingBindings);
        }
    }

    let caller_position = python_caller_position(shadow, caller_name);

    // Resolvable bindings in effect at the caller's definition point (strictly
    // before it in module source order): the from-import symbol and each
    // in-module def. A binding after the caller cannot be proven effective.
    let mut resolvable: Vec<(usize, String)> = Vec::new();
    if let Some(binding) = imports.module_binding(callee) {
        if binding.kind == BindingKind::FromImportSymbol && binding.position < caller_position {
            resolvable.push((binding.position, binding.canonical_path.clone()));
        }
    }
    for position in shadow.module_defs.get(callee).into_iter().flatten() {
        if *position < caller_position {
            resolvable.push((*position, format!("{module}.{callee}")));
        }
    }

    // Every module-scope binding position of the name (contest candidates): a
    // binding after the winner makes the call-time value undecidable (C7-1).
    let mut all_positions: Vec<usize> = Vec::new();
    if let Some(binding) = imports.module_binding(callee) {
        if binding.kind == BindingKind::FromImportSymbol {
            all_positions.push(binding.position);
        }
    }
    all_positions.extend(
        shadow
            .module_defs
            .get(callee)
            .into_iter()
            .flatten()
            .copied(),
    );
    all_positions.extend(
        shadow
            .module_rebinds
            .get(callee)
            .into_iter()
            .flatten()
            .copied(),
    );
    all_positions.extend(imports.star_invalidators().iter().copied());
    if imports.is_ambiguous(callee) {
        if let Some(position) = imports.ambiguity_position(callee) {
            all_positions.push(position);
        }
    }

    if let Some((winner_position, winner_target)) =
        resolvable.iter().max_by_key(|(position, _)| *position)
    {
        if all_positions
            .iter()
            .any(|position| position > winner_position)
        {
            return Err(NoCanonicalTargetReason::Shadowed);
        }
        return Ok(winner_target.clone());
    }

    // No resolvable binding before the caller: an opaque rebind fails closed;
    // star / relative / unbound names keep the recall-safe name-only fallback.
    if shadow
        .module_rebinds
        .get(callee)
        .is_some_and(|positions| !positions.is_empty())
    {
        return Err(NoCanonicalTargetReason::Shadowed);
    }
    Err(NoCanonicalTargetReason::UnsupportedImportForm)
}

/// Resolve one staged Python call to an exact canonical target, or return the
/// typed reason that controls whether name-only fallback is recall-safe. The
/// T5c shadow guard (function-local poison and order-aware receiver / bare-name
/// rebinds) is applied here, downstream of and around T5b's target selection.
fn python_target_for_staged_call(
    module_path: Option<&str>,
    shadow: &PythonShadowIndex,
    caller_name: Option<&str>,
    call: &StagedCallProvenanceRecord,
) -> Result<String, NoCanonicalTargetReason> {
    let callee = call.callee_name.as_str();
    match call.qualifier_kind.as_str() {
        "module" => {
            let Some(binding) = shadow.imports.module_binding(&call.raw_qualifier) else {
                return Err(NoCanonicalTargetReason::CompetingBindings);
            };
            if binding.kind != BindingKind::ModuleImport
                || python_name_is_function_local(shadow, caller_name, &call.raw_qualifier)
                || python_receiver_rebound_after(shadow, &call.raw_qualifier, binding.position)
            {
                return Err(NoCanonicalTargetReason::CompetingBindings);
            }
            Ok(format!("{}.{}", binding.canonical_path, callee))
        }
        "python_bare" => match module_path {
            Some(module) => python_bare_target(module, shadow, caller_name, callee),
            None => Err(NoCanonicalTargetReason::NoModuleContext),
        },
        _ => Err(NoCanonicalTargetReason::CompetingBindings),
    }
}

/// Full-index post-pass for both legacy bare-name staging and Unit-B canonical
/// qualified / known-receiver staging.
async fn reresolve_calls_edges_with_canonical_context(
    queries: &CodeGraphQueries,
    ws_path: &Path,
    crates: &canonical::WorkspaceCrates,
    unsafe_prefixes: &HashSet<String>,
    python_packages: &HashSet<String>,
) -> Result<ReresolveResult, EngramError> {
    let mut result = queries.reresolve_calls_edges().await?;
    let staged: Vec<_> = queries
        .list_staged_calls_with_provenance()
        .await?
        .into_iter()
        .filter(|call| !call.qualifier_kind.is_empty())
        .collect();
    if staged.is_empty() {
        return Ok(result);
    }

    // Snapshot pre-existing edges BEFORE any retraction. A non-forced full index
    // does not retract every caller's edges, so this dedup set stops a merely
    // re-asserted edge from being counted as newly `resolved`. The retraction
    // loop below deletes exactly the CANONICAL edges of callers with staged
    // calls; seeding this set AFTER retraction would miss those edges and
    // re-count them as new. Singleton edges (produced by the bare-name pass above
    // and already counted in `result.resolved`, plus any pre-existing ones) are
    // seeded too: a canonical target that coincides with an existing
    // `(caller, callee)` singleton pair is a provenance UPGRADE of the same edge,
    // not a newly resolved edge, so it must not double-count. Only genuinely new
    // `(caller, target)` pairs increment `result.resolved`.
    let mut created: HashSet<(String, String)> = queries
        .list_calls_edges_by_resolution("calls_resolved_canonical")
        .await?
        .into_iter()
        .collect();
    created.extend(
        queries
            .list_calls_edges_by_resolution("calls_resolved_singleton")
            .await?,
    );
    // Snapshot `direct` pairs (genuine in-file calls resolved at parse time). A
    // caller that reaches the same in-file target BOTH bare (`foo()`, a `direct`
    // edge) and via a qualified path (`crate::m::foo()`, staged for this pass)
    // must NOT have that `direct` edge overwritten with canonical provenance:
    // `calls_edge` is keyed by `(from, to)`, so the canonical `:put` would
    // replace `direct` in place. That double-counts the pair in `edges_created`
    // (it was already counted when the direct edge was created) and — because
    // the down-migration rollback retracts canonical edges — would delete an edge
    // that represents a real direct call. The bare-name singleton pass cannot hit
    // this: a staged call's callee was, by construction, unresolved in-file, so
    // it never targets an in-file function the caller also calls directly.
    let direct_pairs: HashSet<(String, String)> = queries
        .list_calls_edges_by_resolution("direct")
        .await?
        .into_iter()
        .collect();

    let mut callers = HashSet::new();
    for call in &staged {
        if callers.insert(call.caller_id.clone()) {
            queries
                .retract_canonical_edges_from_caller(&call.caller_id)
                .await?;
        }
    }

    let canonical_index = queries.function_ids_by_canonical_path().await?;
    let mut context_cache: HashMap<String, Option<(canonical::ModulePath, canonical::UseGraph)>> =
        HashMap::new();
    let mut python_context_cache: HashMap<String, Option<PythonStagedContext>> = HashMap::new();

    for call in &staged {
        result.lookups += 1;
        if is_py_path(&call.source_file) {
            if !python_context_cache.contains_key(&call.source_file) {
                let ctx = python_ctx_for_staged_file(
                    ws_path,
                    python_packages,
                    queries,
                    &call.source_file,
                )
                .await;
                python_context_cache.insert(call.source_file.clone(), ctx);
            }
            let Some(Some((module_path, shadow, caller_names))) =
                python_context_cache.get(&call.source_file)
            else {
                continue;
            };
            let caller_name = caller_names.get(&call.caller_id).map(String::as_str);
            match python_target_for_staged_call(module_path.as_deref(), shadow, caller_name, call) {
                Ok(target) => {
                    let target_id = match canonical_index.get(&target) {
                        Some(ids) if ids.len() == 1 => ids[0].clone(),
                        _ => continue,
                    };
                    let pair = (call.caller_id.clone(), target_id);
                    if direct_pairs.contains(&pair) {
                        continue;
                    }
                    queries
                        .create_calls_edge_with_resolution(
                            &pair.0,
                            &pair.1,
                            "calls_resolved_canonical",
                        )
                        .await?;
                    if created.insert(pair) {
                        result.resolved += 1;
                    }
                }
                Err(reason) if reason.allows_name_only_fallback() => {
                    let ids = queries
                        .function_ids_by_name(&call.callee_name, "python")
                        .await?;
                    if ids.len() != 1 {
                        continue;
                    }
                    let pair = (call.caller_id.clone(), ids[0].clone());
                    if direct_pairs.contains(&pair) {
                        continue;
                    }
                    queries
                        .create_calls_edge_with_resolution(
                            &pair.0,
                            &pair.1,
                            "calls_resolved_singleton",
                        )
                        .await?;
                    if created.insert(pair) {
                        result.resolved += 1;
                    }
                }
                Err(_) => {}
            }
            continue;
        }
        if !context_cache.contains_key(&call.source_file) {
            let ctx = rust_ctx_for_staged_file(ws_path, crates, &call.source_file).await;
            context_cache.insert(call.source_file.clone(), ctx);
        }
        let target_id = {
            let Some(Some((module, use_graph))) = context_cache.get(&call.source_file) else {
                continue;
            };
            let Some(target) =
                canonical_target_for_staged_call(crates, module, use_graph, unsafe_prefixes, call)
            else {
                continue;
            };
            match canonical_index.get(&target) {
                Some(ids) if ids.len() == 1 => Some(ids[0].clone()),
                _ => None,
            }
        };
        if let Some(target_id) = target_id {
            let pair = (call.caller_id.clone(), target_id);
            // A higher-confidence `direct` edge for this exact pair outranks a
            // canonical resolution: leave it untouched so its provenance (and the
            // rollback / count invariants above) stay correct.
            if direct_pairs.contains(&pair) {
                continue;
            }
            queries
                .create_calls_edge_with_resolution(&pair.0, &pair.1, "calls_resolved_canonical")
                .await?;
            if created.insert(pair) {
                result.resolved += 1;
            }
        }
    }

    Ok(result)
}

/// Summary returned by [`index_workspace`] after indexing completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResult {
    /// Number of source files successfully parsed.
    pub files_parsed: usize,
    /// Number of files skipped (unsupported, too large, or unchanged).
    pub files_skipped: usize,
    /// Number of files skipped specifically because they exceeded
    /// [`CodeGraphConfig::max_file_size_bytes`].
    ///
    /// Counted separately from `files_skipped` so callers can distinguish
    /// capacity-policy skips from parse errors and unchanged-file skips.
    /// Oversized files are not added to `errors` — they are a normal
    /// policy outcome, not a processing failure.
    pub oversized_files_skipped: usize,
    /// Number of function records upserted.
    pub functions_indexed: usize,
    /// Number of class (struct) records upserted.
    pub classes_indexed: usize,
    /// Number of interface (trait) records upserted.
    pub interfaces_indexed: usize,
    /// Number of edge records created.
    pub edges_created: usize,
    /// Number of embedding vectors generated.
    pub embeddings_generated: usize,
    /// Count of Tier 1 (`explicit_code`) symbols.
    pub tier1_count: usize,
    /// Count of Tier 2 (`summary_pointer`) symbols.
    pub tier2_count: usize,
    /// Number of cross-file import/call edges dropped (deferred to future phase).
    pub cross_file_edges_dropped: usize,
    /// Per-file errors encountered (non-fatal).
    pub errors: Vec<FileError>,
    /// Total indexing duration in milliseconds.
    pub duration_ms: u64,
}

/// A non-fatal error encountered while indexing a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileError {
    /// Workspace-relative file path.
    pub file: String,
    /// Error description.
    pub error: String,
}

/// Callback invoked with `(completed, total)` file progress during index or sync.
pub type ProgressCallback<'a> = dyn FnMut(u64, u64) + Send + 'a;

fn emit_progress(progress: &mut Option<&mut ProgressCallback<'_>>, completed: u64, total: u64) {
    if let Some(callback) = progress.as_deref_mut() {
        callback(completed, total);
    }
}

fn advance_progress(
    progress: &mut Option<&mut ProgressCallback<'_>>,
    completed: &mut u64,
    total: u64,
) {
    *completed += 1;
    emit_progress(progress, *completed, total);
}

/// Discover, parse, and index all supported source files in the workspace.
///
/// Uses the `ignore` crate for .gitignore-aware file traversal, filters by
/// supported languages and file size, parses via tree-sitter, assigns tiered
/// embeddings, and persists all nodes and edges to CozoDB.
///
/// `SQLITE_BUSY` retries are handled at the individual `run_script` level
/// inside `upsert_function`, `upsert_class`, and `upsert_interface`, so
/// per-symbol writes retry independently without skipping files whose
/// `content_hash` was already committed.
///
/// # Errors
///
/// Returns `EngramError` on database connection failure or fatal I/O errors.
/// Per-file parse errors are collected in `IndexResult::errors` (non-fatal).
pub async fn index_workspace(
    ws_path: &Path,
    data_dir: &Path,
    branch: &str,
    config: &CodeGraphConfig,
    force: bool,
) -> Result<IndexResult, EngramError> {
    index_workspace_with_progress(ws_path, data_dir, branch, config, force, None).await
}

/// Discover, parse, and index all supported source files while reporting
/// progress snapshots to interested callers.
pub async fn index_workspace_with_progress(
    ws_path: &Path,
    data_dir: &Path,
    branch: &str,
    config: &CodeGraphConfig,
    force: bool,
    progress: Option<&mut ProgressCallback<'_>>,
) -> Result<IndexResult, EngramError> {
    index_workspace_impl(ws_path, data_dir, branch, config, force, progress, false).await
}

async fn index_workspace_impl(
    ws_path: &Path,
    data_dir: &Path,
    branch: &str,
    config: &CodeGraphConfig,
    force: bool,
    mut progress: Option<&mut ProgressCallback<'_>>,
    force_prepass_cache_miss: bool,
) -> Result<IndexResult, EngramError> {
    let start = std::time::Instant::now();

    let db = connect_db(data_dir, branch).await?;
    let queries = CodeGraphQueries::new(db);

    queries.clear_index_canonical_workspace_snapshot().await?;

    // Option C Unit A / A6: workspace crate set for canonical-identity derivation
    // (computed once per index run; Rust-only, precision-neutral).
    let crates = canonical::discover_workspace_crates(ws_path);

    // ── Step 1: Discover files ──────────────────────────────────────
    let files = discover_files(ws_path, config);
    let prepass = unsafe_module_prepass(ws_path, &crates, &files).await;
    let unsafe_prefixes = prepass.unsafe_prefixes.clone();
    // 096-F/T3: provable Python regular-package directories, computed once and
    // persisted in the snapshot so a later sync can detect topology drift.
    let python_packages = python_package_dirs(&files, ws_path);
    let is_regular_package =
        |p: &Path| python_packages.contains(&p.to_string_lossy().replace('\\', "/"));
    let canonical_workspace = canonical::CanonicalWorkspace {
        crates: crates.clone(),
        unsafe_prefixes: unsafe_prefixes.clone(),
        python_packages: python_packages.clone(),
    };
    info!(
        files_found = files.len(),
        "code graph: discovered source files"
    );
    let total_files = files.len() as u64;
    let mut completed_files = 0_u64;
    emit_progress(&mut progress, completed_files, total_files);

    let mut result = IndexResult {
        files_parsed: 0,
        files_skipped: 0,
        oversized_files_skipped: 0,
        functions_indexed: 0,
        classes_indexed: 0,
        interfaces_indexed: 0,
        edges_created: 0,
        embeddings_generated: 0,
        tier1_count: 0,
        tier2_count: 0,
        cross_file_edges_dropped: 0,
        errors: Vec::new(),
        duration_ms: 0,
    };

    // ── Step 2: Process each file ───────────────────────────────────
    for file_path in &files {
        'file: {
            let rel_path = if let Ok(p) = file_path.strip_prefix(ws_path) {
                p.to_string_lossy().replace('\\', "/")
            } else {
                warn!(path = %file_path.display(), "code graph: file outside workspace root, skipping");
                result.files_skipped += 1;
                break 'file;
            };

            // ── Early size check via filesystem metadata ────────────────
            // Avoids reading large files into memory only to discard them.
            // A metadata I/O failure is non-fatal: fall through to the
            // content-based check that follows the file read.
            if let Ok(meta) = tokio::fs::metadata(file_path).await {
                if meta.len() > config.max_file_size_bytes {
                    let stale_file_id = format!("code_file:{}", sha256_short(&rel_path));
                    let _orphaned =
                        handle_deleted_file(&queries, &rel_path, &stale_file_id).await?;
                    warn!(
                        path = %rel_path,
                        size_bytes = meta.len(),
                        limit_bytes = config.max_file_size_bytes,
                        "code graph: skipping oversized file"
                    );
                    result.oversized_files_skipped += 1;
                    result.files_skipped += 1;
                    break 'file;
                }
            }

            // Read file contents.
            let source = match tokio::fs::read_to_string(file_path).await {
                Ok(s) => s,
                Err(e) => {
                    result.errors.push(FileError {
                        file: rel_path.clone(),
                        error: format!("read error: {e}"),
                    });
                    result.files_skipped += 1;
                    break 'file;
                }
            };

            // Secondary size guard: protects against metadata races (TOCTOU).
            let size_bytes = source.len() as u64;
            if size_bytes > config.max_file_size_bytes {
                let stale_file_id = format!("code_file:{}", sha256_short(&rel_path));
                let _orphaned = handle_deleted_file(&queries, &rel_path, &stale_file_id).await?;
                warn!(
                    path = %rel_path,
                    size_bytes,
                    limit_bytes = config.max_file_size_bytes,
                    "code graph: skipping oversized file (content check)"
                );
                result.oversized_files_skipped += 1;
                result.files_skipped += 1;
                break 'file;
            }

            // Compute content hash.
            let content_hash = sha256_hex(&source);

            // Skip unchanged files (unless forced).
            if !force {
                if let Ok(Some(existing)) = queries.get_code_file_by_path(&rel_path).await {
                    if existing.content_hash == content_hash {
                        debug!(path = %rel_path, "code graph: skipping unchanged file");
                        result.files_skipped += 1;
                        break 'file;
                    }
                }
            }

            // Detect language from extension.
            let lang = language_from_path(file_path);
            if !config.supported_languages.contains(&lang) {
                result.files_skipped += 1;
                break 'file;
            }

            // ── Parse via tree-sitter (CPU-bound, run in blocking task) ─
            let source_clone = source.clone();
            let lang_enum = match Language::try_from(lang.as_str()) {
                Ok(l) => l,
                Err(e) => {
                    result.errors.push(FileError {
                        file: rel_path.clone(),
                        error: e.to_string(),
                    });
                    result.files_skipped += 1;
                    break 'file;
                }
            };
            let parse_result =
                match tokio::task::spawn_blocking(move || parse_source(&source_clone, lang_enum))
                    .await
                {
                    Ok(Ok(pr)) => pr,
                    Ok(Err(e)) => {
                        result.errors.push(FileError {
                            file: rel_path.clone(),
                            error: e.to_string(),
                        });
                        result.files_skipped += 1;
                        break 'file;
                    }
                    Err(e) => {
                        result.errors.push(FileError {
                            file: rel_path.clone(),
                            error: format!("task join error: {e}"),
                        });
                        result.files_skipped += 1;
                        break 'file;
                    }
                };

            // ── Upsert code file node ───────────────────────────────────
            let file_id = format!("code_file:{}", sha256_short(&rel_path));
            let code_file = CodeFile {
                id: file_id.clone(),
                path: rel_path.clone(),
                language: lang.clone(),
                size_bytes,
                content_hash: content_hash.clone(),
                last_indexed_at: chrono::Utc::now().to_rfc3339(),
            };
            queries.upsert_code_file(&code_file).await?;

            // Clear previous edges from this file.
            // 082.009-T: retract this file's prior calls_resolved_singleton
            // edges (caller or callee in this file) WHILE the old symbol IDs
            // still exist, and clear its staged calls, before deleting symbols
            // and re-staging.
            queries
                .retract_resolved_calls_edges_for_file(&rel_path)
                .await?;
            queries.clear_staged_calls_for_file(&rel_path).await?;
            queries.delete_functions_by_file(&rel_path).await?;
            queries.delete_classes_by_file(&rel_path).await?;
            queries.delete_interfaces_by_file(&rel_path).await?;
            queries.delete_edges_from_file("defines", &file_id).await?;
            queries
                .delete_edges_from_file("references", &file_id)
                .await?;

            // ── Collect symbols for embedding ───────────────────────────
            let token_limit = config.embedding.token_limit;
            let mut embed_texts: Vec<String> = Vec::new();
            let mut embed_ids: Vec<String> = Vec::new();

            // Track symbol IDs for edge creation.
            let mut function_ids: Vec<(String, String)> = Vec::new(); // (name, id)
            let mut class_ids: Vec<(String, String)> = Vec::new();
            let mut interface_ids: Vec<(String, String)> = Vec::new();

            // A6: per-file canonical context (Rust-only; None → empty canonical_path).
            let rust_ctx = rust_ctx_from_prepass_cache(
                &prepass.rust_contexts,
                &rel_path,
                &content_hash,
                force_prepass_cache_miss,
                || rust_canonical_ctx(&crates, lang_enum, &rel_path, &source),
            );

            // 096-F/T3: per-file Python module namespace (Some only on a provable
            // regular-package chain; None → fail-closed empty canonical_path).
            let python_module = if matches!(lang_enum, Language::Python) {
                python_module_path_for_file(&rel_path, &is_regular_package)
            } else {
                None
            };
            let py_shadow = if matches!(lang_enum, Language::Python) {
                Some(PythonShadowIndex::build(&source))
            } else {
                None
            };

            for symbol in &parse_result.symbols {
                match symbol {
                    ExtractedSymbol::Function(f) => {
                        let sym_id = format!("function:{}", Uuid::new_v4());
                        let (embed_type, summary) = tier_classification(
                            f.token_count as usize,
                            token_limit,
                            &f.body,
                            &f.signature,
                            f.docstring.as_deref(),
                        );
                        embed_texts.push(summary.clone());
                        embed_ids.push(sym_id.clone());

                        let func = crate::models::function::Function {
                            id: sym_id.clone(),
                            name: f.name.clone(),
                            file_path: rel_path.clone(),
                            line_start: f.line_start,
                            line_end: f.line_end,
                            signature: f.signature.clone(),
                            docstring: f.docstring.clone(),
                            body: f.body.clone(),
                            body_hash: f.body_hash.clone(),
                            token_count: f.token_count,
                            embed_type: embed_type.to_owned(),
                            embedding: vec![0.0_f32; embedding::EMBEDDING_DIM],
                            summary,
                        };
                        let canonical_path = canonical_path_for_function(
                            lang_enum,
                            &crates,
                            rust_ctx.as_ref(),
                            python_module.as_deref(),
                            &unsafe_prefixes,
                            &f.name,
                        );
                        queries
                            .upsert_function_with_canonical(&func, &canonical_path)
                            .await?;
                        function_ids.push((f.name.clone(), sym_id.clone()));

                        if embed_type == "explicit_code" {
                            result.tier1_count += 1;
                        } else {
                            result.tier2_count += 1;
                        }
                        result.functions_indexed += 1;

                        // Create defines edge.
                        queries
                            .create_defines_edge(&file_id, "function", &sym_id)
                            .await?;
                        result.edges_created += 1;
                    }
                    ExtractedSymbol::Class(c) => {
                        let sym_id = format!("class:{}", Uuid::new_v4());
                        let (embed_type, summary) = tier_classification(
                            c.token_count as usize,
                            token_limit,
                            &c.body,
                            "",
                            c.docstring.as_deref(),
                        );
                        embed_texts.push(summary.clone());
                        embed_ids.push(sym_id.clone());

                        let class = crate::models::class::Class {
                            id: sym_id.clone(),
                            name: c.name.clone(),
                            file_path: rel_path.clone(),
                            line_start: c.line_start,
                            line_end: c.line_end,
                            docstring: c.docstring.clone(),
                            body: c.body.clone(),
                            body_hash: c.body_hash.clone(),
                            token_count: c.token_count,
                            embed_type: embed_type.to_owned(),
                            embedding: vec![0.0_f32; embedding::EMBEDDING_DIM],
                            summary,
                        };
                        queries.upsert_class(&class).await?;
                        class_ids.push((c.name.clone(), sym_id.clone()));

                        if embed_type == "explicit_code" {
                            result.tier1_count += 1;
                        } else {
                            result.tier2_count += 1;
                        }
                        result.classes_indexed += 1;

                        queries
                            .create_defines_edge(&file_id, "class", &sym_id)
                            .await?;
                        result.edges_created += 1;
                    }
                    ExtractedSymbol::Interface(i) => {
                        let sym_id = format!("interface:{}", Uuid::new_v4());
                        let (embed_type, summary) = tier_classification(
                            i.token_count as usize,
                            token_limit,
                            &i.body,
                            "",
                            i.docstring.as_deref(),
                        );
                        embed_texts.push(summary.clone());
                        embed_ids.push(sym_id.clone());

                        let iface = crate::models::interface::Interface {
                            id: sym_id.clone(),
                            name: i.name.clone(),
                            file_path: rel_path.clone(),
                            line_start: i.line_start,
                            line_end: i.line_end,
                            docstring: i.docstring.clone(),
                            body: i.body.clone(),
                            body_hash: i.body_hash.clone(),
                            token_count: i.token_count,
                            embed_type: embed_type.to_owned(),
                            embedding: vec![0.0_f32; embedding::EMBEDDING_DIM],
                            summary,
                        };
                        queries.upsert_interface(&iface).await?;
                        interface_ids.push((i.name.clone(), sym_id.clone()));

                        if embed_type == "explicit_code" {
                            result.tier1_count += 1;
                        } else {
                            result.tier2_count += 1;
                        }
                        result.interfaces_indexed += 1;

                        queries
                            .create_defines_edge(&file_id, "interface", &sym_id)
                            .await?;
                        result.edges_created += 1;
                    }
                }
            }

            // ── Batch embed (non-fatal if model not loaded) ─────────────
            if !embed_texts.is_empty() {
                match embedding::embed_texts(&embed_texts) {
                    Ok(vectors) => {
                        result.embeddings_generated += vectors.len();
                        for (sym_id, vector) in embed_ids.iter().zip(vectors) {
                            if let Err(e) = queries.update_symbol_embedding(sym_id, vector).await {
                                debug!(error = %e, sym_id = %sym_id, "code graph: embedding write-back failed");
                            }
                        }
                        debug!(
                            count = result.embeddings_generated,
                            "code graph: generated and stored embeddings for file"
                        );
                    }
                    Err(e) => {
                        debug!(error = %e, "code graph: embedding unavailable, skipping");
                    }
                }
            }

            // ── Create edges from extracted relationships ───────────────
            for edge in &parse_result.edges {
                match edge {
                    ExtractedEdge::Calls {
                        caller,
                        callee,
                        is_method,
                        is_qualified,
                        raw_qualifier,
                        qualifier_kind,
                    } => {
                        if *is_method || *is_qualified {
                            if !should_stage_provenance_call(
                                *is_method,
                                *is_qualified,
                                raw_qualifier,
                            ) {
                                continue;
                            }
                            if let Some(from_id) = find_function_id(&function_ids, caller) {
                                let enclosing_canonical_type =
                                    enclosing_canonical_type_for_function(
                                        &crates,
                                        rust_ctx.as_ref(),
                                        &unsafe_prefixes,
                                        caller,
                                    );
                                queries
                                    .put_staged_call_with_provenance(
                                        &from_id,
                                        callee,
                                        &rel_path,
                                        raw_qualifier,
                                        qualifier_kind,
                                        &enclosing_canonical_type,
                                    )
                                    .await?;
                            }
                            continue;
                        }
                        // Resolve names to IDs within this file's symbols. A
                        // callee resolved locally becomes a direct edge; a
                        // caller-resolved but callee-unresolved (cross-file)
                        // call is staged for the deferred post-pass (082.002-T)
                        // instead of being silently dropped.
                        match (
                            find_function_id(&function_ids, caller),
                            find_function_id(&function_ids, callee),
                        ) {
                            (Some(from_id), Some(to_id)) => {
                                if matches!(lang_enum, Language::Python)
                                    && py_shadow
                                        .as_ref()
                                        .is_some_and(|shadow| shadow.is_contested(callee, caller))
                                {
                                    queries
                                        .put_staged_call_with_provenance(
                                            &from_id,
                                            callee,
                                            &rel_path,
                                            "",
                                            "python_bare",
                                            "",
                                        )
                                        .await?;
                                } else {
                                    queries.create_calls_edge(&from_id, &to_id).await?;
                                    result.edges_created += 1;
                                }
                            }
                            (Some(from_id), None) => {
                                if matches!(lang_enum, Language::Python) {
                                    queries
                                        .put_staged_call_with_provenance(
                                            &from_id,
                                            callee,
                                            &rel_path,
                                            "",
                                            "python_bare",
                                            "",
                                        )
                                        .await?;
                                } else {
                                    queries.put_staged_call(&from_id, callee, &rel_path).await?;
                                }
                            }
                            _ => {}
                        }
                    }
                    ExtractedEdge::InheritsFrom {
                        struct_name,
                        trait_name,
                    } => {
                        if let Some(child_id) = find_class_id(&class_ids, struct_name) {
                            if let Some(parent_id) = find_interface_id(&interface_ids, trait_name) {
                                queries
                                    .create_inherits_edge(
                                        "class",
                                        &child_id,
                                        "interface",
                                        &parent_id,
                                    )
                                    .await?;
                                result.edges_created += 1;
                            }
                        }
                    }
                    // Defines already created above; Imports are cross-file (deferred, counted).
                    ExtractedEdge::Imports { .. } => {
                        result.cross_file_edges_dropped += 1;
                    }
                    // Defines edge already handled during symbol upsert.
                    ExtractedEdge::Defines { .. } => {}
                    // SQL References: resolve target to a Class node or self-loop (033.001-T).
                    ExtractedEdge::References { target, .. } => {
                        let resolved_id = queries.resolve_reference_target(target).await?;
                        if let Some(class_id) = resolved_id {
                            queries
                                .create_references_edge(&file_id, &class_id, Some(target))
                                .await?;
                        } else {
                            queries
                                .create_references_edge(&file_id, &file_id, Some(target))
                                .await?;
                        }
                        result.edges_created += 1;
                    }
                }
            }

            result.files_parsed += 1;
            // Record file hash for offline change detection (TASK-009.09).
            // Non-fatal: a hash recording failure degrades offline detection but
            // does not invalidate the indexed code graph.
            if let Err(e) =
                crate::services::file_tracker::record_file_hash(&rel_path, file_path, &queries)
                    .await
            {
                debug!(
                    error = %e,
                    path = %rel_path,
                    "code graph: file hash recording failed — offline detection may report false changes"
                );
            }
            debug!(path = %rel_path, "code graph: indexed file");
        }
        // Progress counts completed file decisions, not only successfully indexed
        // files, so skips and non-fatal per-file errors still advance the caller's
        // view of total work completed.
        advance_progress(&mut progress, &mut completed_files, total_files);
    }

    // ── Post-pass: re-resolve unresolved references edges ───────────
    // Files are processed in filesystem order. A reference to `public.users`
    // may be processed before the `users` class is created, leaving a self-loop.
    // Now that all symbols exist, retry resolution for those edges.
    let reresolved = queries.reresolve_references_edges().await?;
    if reresolved.resolved > 0 {
        debug!(
            count = reresolved.resolved,
            "code graph: re-resolved deferred references edges"
        );
    }

    // ── Post-pass: resolve staged cross-file calls (082.008-T) ──────
    // Full / --force index only — NOT the incremental sync path (performance
    // gate). Staged calls (082.002-T) whose callee name is unambiguous
    // (exactly one workspace-global definition) become calls_resolved_singleton
    // edges; ambiguous / unmatched names are skipped to bound false edges.
    let resolved_calls = reresolve_calls_edges_with_canonical_context(
        &queries,
        ws_path,
        &crates,
        &unsafe_prefixes,
        &python_packages,
    )
    .await?;
    // Post-pass singletons are real edge records: include them in the reported
    // edges_created so full-index CLI/API responses do not underreport (082.008-T).
    result.edges_created += resolved_calls.resolved;
    if resolved_calls.resolved > 0 {
        debug!(
            count = resolved_calls.resolved,
            lookups = resolved_calls.lookups,
            "code graph: resolved cross-file singleton calls edges"
        );
    }
    queries
        .replace_index_canonical_workspace_snapshot(&canonical_workspace)
        .await?;

    #[allow(clippy::cast_possible_truncation)]
    let elapsed = start.elapsed().as_millis() as u64;
    result.duration_ms = elapsed;

    info!(
        files_parsed = result.files_parsed,
        files_skipped = result.files_skipped,
        oversized_files_skipped = result.oversized_files_skipped,
        functions = result.functions_indexed,
        classes = result.classes_indexed,
        interfaces = result.interfaces_indexed,
        edges = result.edges_created,
        duration_ms = result.duration_ms,
        "code graph: indexing complete"
    );

    Ok(result)
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Summary returned by [`sync_workspace`] after incremental sync completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Number of files that were modified and re-indexed.
    pub files_modified: usize,
    /// Number of new files added and indexed.
    pub files_added: usize,
    /// Number of files deleted (nodes removed).
    pub files_deleted: usize,
    /// Number of files unchanged (skipped).
    pub files_unchanged: usize,
    /// Number of symbols that were re-embedded because their body changed.
    pub symbols_reembedded: usize,
    /// Number of symbols that kept existing embeddings (body unchanged).
    pub symbols_reused: usize,
    /// Number of `concerns` edges re-linked to new symbol nodes (FR-124).
    pub concerns_relinked: usize,
    /// Number of `concerns` edges orphaned and removed (FR-112).
    pub concerns_orphaned: usize,
    /// Number of edge records created (defines, references, concerns).
    pub edges_created: usize,
    /// Number of cross-file import/call edges dropped (deferred to future phase).
    pub cross_file_edges_dropped: usize,
    /// Number of files skipped specifically because they exceeded
    /// [`CodeGraphConfig::max_file_size_bytes`].
    ///
    /// Oversized files are not added to `errors` — they are a normal
    /// policy outcome, not a processing failure.
    pub oversized_files_skipped: usize,
    /// Per-file errors encountered (non-fatal).
    pub errors: Vec<FileError>,
    /// Total sync duration in milliseconds.
    pub duration_ms: u64,
}

/// Incrementally sync the code graph with changes on disk.
///
/// Detects changed, added, and deleted files since the last index
/// and updates only affected nodes. Uses two-level hashing:
///
/// 1. **File-level** – `content_hash` on `code_file` nodes identifies
///    which files changed on disk.
/// 2. **Symbol-level** – `body_hash` on function/class/interface nodes
///    identifies which symbols within a changed file actually need
///    re-embedding.
///
/// Preserves `concerns` edges across file moves via hash-resilient
/// identity matching on `(name, body_hash)` tuples (FR-124).
///
/// If no prior index exists, falls back to a full index (same outcome
/// as calling `index_workspace`).
///
/// # Errors
///
/// Returns `EngramError` on database connection failure or fatal I/O errors.
/// Per-file parse errors are collected in `SyncResult::errors` (non-fatal).
pub async fn sync_workspace(
    ws_path: &Path,
    data_dir: &Path,
    branch: &str,
    config: &CodeGraphConfig,
) -> Result<SyncResult, EngramError> {
    sync_workspace_with_progress(ws_path, data_dir, branch, config, None).await
}

/// Incrementally sync the code graph while reporting `(completed, total)` file
/// progress to callers that want streamed startup visibility.
pub async fn sync_workspace_with_progress(
    ws_path: &Path,
    data_dir: &Path,
    branch: &str,
    config: &CodeGraphConfig,
    mut progress: Option<&mut ProgressCallback<'_>>,
) -> Result<SyncResult, EngramError> {
    let start = std::time::Instant::now();

    let db = connect_db(data_dir, branch).await?;
    let queries = CodeGraphQueries::new(db);

    let previous_canonical_workspace = queries.load_index_canonical_workspace_snapshot().await?;
    queries.clear_index_canonical_workspace_snapshot().await?;

    // Option C Unit A / A6: workspace crate set for canonical-identity derivation.
    let crates = canonical::discover_workspace_crates(ws_path);

    // Discover current files on disk.
    let current_files = discover_files(ws_path, config);
    let prepass = unsafe_module_prepass(ws_path, &crates, &current_files).await;
    let unsafe_prefixes = prepass.unsafe_prefixes.clone();
    // 096-F/T3: current provable Python regular-package directories.
    let python_packages = python_package_dirs(&current_files, ws_path);
    let is_regular_package =
        |p: &Path| python_packages.contains(&p.to_string_lossy().replace('\\', "/"));
    let canonical_workspace = canonical::CanonicalWorkspace {
        crates: crates.clone(),
        unsafe_prefixes: unsafe_prefixes.clone(),
        python_packages: python_packages.clone(),
    };

    // Load all indexed code files from DB.
    let indexed_files = queries.list_code_files().await?;
    let indexed_map: HashMap<String, CodeFile> = indexed_files
        .into_iter()
        .map(|f| (f.path.clone(), f))
        .collect();

    // 096-F/C6-1: force per-file canonical recompute for Python files whose
    // package ancestry changed since the last index/sync. The content-hash skip
    // below would otherwise leave a descendant's `canonical_path` stale when an
    // `__init__.py` is added or removed without the descendant's bytes changing
    // (an empty `__init__.py` never persists as a code file, so its add/remove is
    // invisible to `indexed_map` — the persisted snapshot is the only witness).
    let force_python_recompute: std::collections::HashSet<String> =
        match previous_canonical_workspace.as_ref() {
            Some(previous) => {
                let changed: Vec<&String> = python_packages
                    .symmetric_difference(&previous.python_packages)
                    .collect();
                if changed.is_empty() {
                    std::collections::HashSet::new()
                } else {
                    indexed_map
                        .keys()
                        .filter(|rel| changed.iter().any(|dir| is_python_descendant(rel, dir)))
                        .cloned()
                        .collect()
                }
            }
            // No snapshot (legacy DB, or the snapshot self-erased on prior drift):
            // fail closed — recompute every indexed `.py` file this sync.
            None => indexed_map
                .keys()
                .filter(|rel| is_py_path(rel))
                .cloned()
                .collect(),
        };

    // Build a set of current relative paths for deletion detection.
    let current_rel_paths: std::collections::HashSet<String> = current_files
        .iter()
        .filter_map(|p| {
            p.strip_prefix(ws_path)
                .ok()
                .map(|r| r.to_string_lossy().replace('\\', "/"))
        })
        .collect();
    let deleted_paths: Vec<_> = indexed_map
        .iter()
        .filter(|(indexed_path, _)| !current_rel_paths.contains(*indexed_path))
        .collect();
    let total_files = (deleted_paths.len() + current_files.len()) as u64;
    let mut completed_files = 0_u64;
    emit_progress(&mut progress, completed_files, total_files);

    let mut result = SyncResult {
        files_modified: 0,
        files_added: 0,
        files_deleted: 0,
        files_unchanged: 0,
        symbols_reembedded: 0,
        symbols_reused: 0,
        concerns_relinked: 0,
        concerns_orphaned: 0,
        edges_created: 0,
        cross_file_edges_dropped: 0,
        oversized_files_skipped: 0,
        errors: Vec::new(),
        duration_ms: 0,
    };

    // ── Phase 1: Detect and remove deleted files ────────────────────
    for (indexed_path, indexed_file) in deleted_paths {
        // File deleted — collect concerns edges before removing symbols.
        let orphaned = handle_deleted_file(&queries, indexed_path, &indexed_file.id).await?;
        result.concerns_orphaned += orphaned;
        result.files_deleted += 1;
        // Progress counts completed file decisions, including unchanged/skipped
        // files, so callers see steady forward motion through the full sync set.
        advance_progress(&mut progress, &mut completed_files, total_files);
    }

    // C8-1: track whether any changed file altered `#[path]`/`#[cfg]` mod mapping
    // (which can make a module prefix newly unsafe). If so, canonical edges are
    // swept after the loop so no stale edge survives under a now-unsafe prefix.
    let mut mod_mapping_changed = false;

    // ── Phase 2: Process current files (add / modify / skip) ────────
    for file_path in &current_files {
        'file: {
            let rel_path = if let Ok(p) = file_path.strip_prefix(ws_path) {
                p.to_string_lossy().replace('\\', "/")
            } else {
                warn!(path = %file_path.display(), "code graph sync: file outside workspace root, skipping");
                break 'file;
            };

            // ── Early size check via filesystem metadata ────────────────
            // Avoids reading large files into memory only to discard them.
            // A metadata I/O failure is non-fatal: fall through to the
            // content-based checks that follow the file read.
            if let Ok(meta) = tokio::fs::metadata(file_path).await {
                let meta_size = meta.len();
                if meta_size == 0 {
                    result.files_unchanged += 1;
                    break 'file;
                }
                if meta_size > config.max_file_size_bytes {
                    let stale_file_id = format!("code_file:{}", sha256_short(&rel_path));
                    let orphaned = handle_deleted_file(&queries, &rel_path, &stale_file_id).await?;
                    result.concerns_orphaned += orphaned;
                    warn!(
                        path = %rel_path,
                        size_bytes = meta_size,
                        limit_bytes = config.max_file_size_bytes,
                        "code graph sync: skipping oversized file"
                    );
                    result.oversized_files_skipped += 1;
                    break 'file;
                }
            }

            // Read file contents.
            let source = match tokio::fs::read_to_string(file_path).await {
                Ok(s) => s,
                Err(e) => {
                    result.errors.push(FileError {
                        file: rel_path.clone(),
                        error: format!("read error: {e}"),
                    });
                    break 'file;
                }
            };

            let size_bytes = source.len() as u64;
            if size_bytes == 0 {
                // Skip empty files (handles TOCTOU race between metadata and read).
                result.files_unchanged += 1;
                break 'file;
            }
            // Secondary size guard: protects against metadata races (TOCTOU).
            if size_bytes > config.max_file_size_bytes {
                let stale_file_id = format!("code_file:{}", sha256_short(&rel_path));
                let orphaned = handle_deleted_file(&queries, &rel_path, &stale_file_id).await?;
                result.concerns_orphaned += orphaned;
                warn!(
                    path = %rel_path,
                    size_bytes,
                    limit_bytes = config.max_file_size_bytes,
                    "code graph sync: skipping oversized file (content check)"
                );
                result.oversized_files_skipped += 1;
                break 'file;
            }

            // Language check.
            let lang = language_from_path(file_path);
            if !config.supported_languages.contains(&lang) {
                break 'file;
            }

            // File-level hash comparison (level 1).
            let content_hash = sha256_hex(&source);
            let is_new = !indexed_map.contains_key(&rel_path);

            if !is_new {
                let existing = &indexed_map[&rel_path];
                // C6-1: a package-topology change forces recompute even when the
                // file bytes are unchanged, so a descendant's canonical_path is not
                // left stale after an `__init__.py` add/remove.
                if existing.content_hash == content_hash
                    && !force_python_recompute.contains(&rel_path)
                {
                    // File unchanged — skip entirely.
                    result.files_unchanged += 1;
                    break 'file;
                }
            }

            // ── File changed or new: collect pre-sync concerns info ─────
            let pre_sync_identities = if is_new {
                Vec::new()
            } else {
                queries.get_symbol_identities_for_file(&rel_path).await?
            };
            let pre_sync_concerns = if is_new {
                Vec::new()
            } else {
                queries.get_concerns_edges_for_file(&rel_path).await?
            };

            // Enrich concerns edges with symbol name + body_hash.
            let enriched_concerns: Vec<_> = pre_sync_concerns
                .into_iter()
                .map(|mut c| {
                    if let Some(ident) = pre_sync_identities.iter().find(|i| i.id == c.symbol_id) {
                        c.symbol_name = ident.name.clone();
                        c.symbol_body_hash = ident.body_hash.clone();
                    }
                    c
                })
                .collect();

            // ── Parse file ──────────────────────────────────────────────
            let source_clone = source.clone();
            let lang_enum = match Language::try_from(lang.as_str()) {
                Ok(l) => l,
                Err(e) => {
                    result.errors.push(FileError {
                        file: rel_path.clone(),
                        error: e.to_string(),
                    });
                    break 'file;
                }
            };
            let parse_result =
                match tokio::task::spawn_blocking(move || parse_source(&source_clone, lang_enum))
                    .await
                {
                    Ok(Ok(pr)) => pr,
                    Ok(Err(e)) => {
                        result.errors.push(FileError {
                            file: rel_path.clone(),
                            error: e.to_string(),
                        });
                        break 'file;
                    }
                    Err(e) => {
                        result.errors.push(FileError {
                            file: rel_path.clone(),
                            error: format!("task join error: {e}"),
                        });
                        break 'file;
                    }
                };

            // ── Upsert code file node ───────────────────────────────────
            let file_id = format!("code_file:{}", sha256_short(&rel_path));
            let code_file = CodeFile {
                id: file_id.clone(),
                path: rel_path.clone(),
                language: lang.clone(),
                size_bytes,
                content_hash: content_hash.clone(),
                last_indexed_at: chrono::Utc::now().to_rfc3339(),
            };
            queries.upsert_code_file(&code_file).await?;

            // Record file hash for offline change detection using the already-computed
            // content_hash and size_bytes — avoids re-reading from disk.
            // Non-fatal: a hash recording failure degrades offline detection but
            // does not invalidate the synced code graph.
            if let Err(e) = crate::services::file_tracker::record_file_hash_precomputed(
                &rel_path,
                &content_hash,
                size_bytes,
                &queries,
            )
            .await
            {
                debug!(
                    error = %e,
                    path = %rel_path,
                    "code graph sync: file hash recording failed — offline detection may report false changes"
                );
            }

            // Clear previous symbols and defines edges for this file.
            // 082.009-T: retract this file's prior calls_resolved_singleton
            // edges and clear its staged calls WHILE the old symbol IDs still
            // exist, before deleting the function metadata — mirroring the
            // full-index and file-deletion paths so an incremental sync never
            // leaves a stale/dangling cross-file edge.
            queries
                .retract_resolved_calls_edges_for_file(&rel_path)
                .await?;
            queries.clear_staged_calls_for_file(&rel_path).await?;
            queries.delete_functions_by_file(&rel_path).await?;
            queries.delete_classes_by_file(&rel_path).await?;
            queries.delete_interfaces_by_file(&rel_path).await?;
            queries.delete_edges_from_file("defines", &file_id).await?;
            queries
                .delete_edges_from_file("references", &file_id)
                .await?;

            // ── Build map of old symbols by (name, body_hash) for reuse ─
            let old_sym_map: HashMap<(String, String), &crate::db::queries::SymbolIdentity> =
                pre_sync_identities
                    .iter()
                    .map(|s| ((s.name.clone(), s.body_hash.clone()), s))
                    .collect();

            // ── Insert new symbols, re-embed only if body changed ───────
            let token_limit = config.embedding.token_limit;
            let mut new_function_ids: Vec<(String, String)> = Vec::new();
            let mut new_class_ids: Vec<(String, String)> = Vec::new();
            let mut new_interface_ids: Vec<(String, String)> = Vec::new();
            let mut embed_texts: Vec<String> = Vec::new();
            let mut embed_ids: Vec<String> = Vec::new();

            // A6: per-file canonical context (Rust-only; None → empty canonical_path).
            let rust_ctx = rust_ctx_from_prepass_cache(
                &prepass.rust_contexts,
                &rel_path,
                &content_hash,
                false,
                || rust_canonical_ctx(&crates, lang_enum, &rel_path, &source),
            );
            // C8-1: note whether this changed file carries a non-default `#[path]`/
            // `#[cfg]` mod mapping, which can make a module prefix newly UNSAFE and
            // strand stale canonical edges from other unchanged callers under it.
            mod_mapping_changed |= rust_ctx
                .as_ref()
                .is_some_and(|(_, ug)| ug.has_non_default_mod_mapping());

            // 096-F/T3: per-file Python module namespace (Some only on a provable
            // regular-package chain; None → fail-closed empty canonical_path).
            let python_module = if matches!(lang_enum, Language::Python) {
                python_module_path_for_file(&rel_path, &is_regular_package)
            } else {
                None
            };
            let py_shadow = if matches!(lang_enum, Language::Python) {
                Some(PythonShadowIndex::build(&source))
            } else {
                None
            };

            for symbol in &parse_result.symbols {
                match symbol {
                    ExtractedSymbol::Function(f) => {
                        let sym_id = format!("function:{}", Uuid::new_v4());
                        let (embed_type, summary) = tier_classification(
                            f.token_count as usize,
                            token_limit,
                            &f.body,
                            &f.signature,
                            f.docstring.as_deref(),
                        );

                        // Check if body_hash matches an old symbol and carry its embedding forward.
                        // Without this, reused symbols would be written with a zero-vector, causing
                        // NaN cosine scores on the next KNN search.
                        let old_embedding = old_sym_map
                            .get(&(f.name.clone(), f.body_hash.clone()))
                            .filter(|s| embedding::has_meaningful_embedding(&s.embedding))
                            .map(|s| s.embedding.clone());

                        let reused = old_embedding.is_some();
                        if reused {
                            result.symbols_reused += 1;
                        } else {
                            embed_texts.push(summary.clone());
                            embed_ids.push(sym_id.clone());
                            result.symbols_reembedded += 1;
                        }

                        let func = crate::models::function::Function {
                            id: sym_id.clone(),
                            name: f.name.clone(),
                            file_path: rel_path.clone(),
                            line_start: f.line_start,
                            line_end: f.line_end,
                            signature: f.signature.clone(),
                            docstring: f.docstring.clone(),
                            body: f.body.clone(),
                            body_hash: f.body_hash.clone(),
                            token_count: f.token_count,
                            embed_type: embed_type.to_owned(),
                            embedding: old_embedding
                                .unwrap_or_else(|| vec![0.0_f32; embedding::EMBEDDING_DIM]),
                            summary,
                        };
                        let canonical_path = canonical_path_for_function(
                            lang_enum,
                            &crates,
                            rust_ctx.as_ref(),
                            python_module.as_deref(),
                            &unsafe_prefixes,
                            &f.name,
                        );
                        queries
                            .upsert_function_with_canonical(&func, &canonical_path)
                            .await?;
                        new_function_ids.push((f.name.clone(), sym_id.clone()));
                        queries
                            .create_defines_edge(&file_id, "function", &sym_id)
                            .await?;
                    }
                    ExtractedSymbol::Class(c) => {
                        let sym_id = format!("class:{}", Uuid::new_v4());
                        let (embed_type, summary) = tier_classification(
                            c.token_count as usize,
                            token_limit,
                            &c.body,
                            "",
                            c.docstring.as_deref(),
                        );

                        let old_embedding = old_sym_map
                            .get(&(c.name.clone(), c.body_hash.clone()))
                            .filter(|s| embedding::has_meaningful_embedding(&s.embedding))
                            .map(|s| s.embedding.clone());

                        let reused = old_embedding.is_some();
                        if reused {
                            result.symbols_reused += 1;
                        } else {
                            embed_texts.push(summary.clone());
                            embed_ids.push(sym_id.clone());
                            result.symbols_reembedded += 1;
                        }

                        let class = crate::models::class::Class {
                            id: sym_id.clone(),
                            name: c.name.clone(),
                            file_path: rel_path.clone(),
                            line_start: c.line_start,
                            line_end: c.line_end,
                            docstring: c.docstring.clone(),
                            body: c.body.clone(),
                            body_hash: c.body_hash.clone(),
                            token_count: c.token_count,
                            embed_type: embed_type.to_owned(),
                            embedding: old_embedding
                                .unwrap_or_else(|| vec![0.0_f32; embedding::EMBEDDING_DIM]),
                            summary,
                        };
                        queries.upsert_class(&class).await?;
                        new_class_ids.push((c.name.clone(), sym_id.clone()));
                        queries
                            .create_defines_edge(&file_id, "class", &sym_id)
                            .await?;
                    }
                    ExtractedSymbol::Interface(i) => {
                        let sym_id = format!("interface:{}", Uuid::new_v4());
                        let (embed_type, summary) = tier_classification(
                            i.token_count as usize,
                            token_limit,
                            &i.body,
                            "",
                            i.docstring.as_deref(),
                        );

                        let old_embedding = old_sym_map
                            .get(&(i.name.clone(), i.body_hash.clone()))
                            .filter(|s| embedding::has_meaningful_embedding(&s.embedding))
                            .map(|s| s.embedding.clone());

                        let reused = old_embedding.is_some();
                        if reused {
                            result.symbols_reused += 1;
                        } else {
                            embed_texts.push(summary.clone());
                            embed_ids.push(sym_id.clone());
                            result.symbols_reembedded += 1;
                        }

                        let iface = crate::models::interface::Interface {
                            id: sym_id.clone(),
                            name: i.name.clone(),
                            file_path: rel_path.clone(),
                            line_start: i.line_start,
                            line_end: i.line_end,
                            docstring: i.docstring.clone(),
                            body: i.body.clone(),
                            body_hash: i.body_hash.clone(),
                            token_count: i.token_count,
                            embed_type: embed_type.to_owned(),
                            embedding: old_embedding
                                .unwrap_or_else(|| vec![0.0_f32; embedding::EMBEDDING_DIM]),
                            summary,
                        };
                        queries.upsert_interface(&iface).await?;
                        new_interface_ids.push((i.name.clone(), sym_id.clone()));
                        queries
                            .create_defines_edge(&file_id, "interface", &sym_id)
                            .await?;
                    }
                }
            }

            // ── Batch embed changed symbols ─────────────────────────────
            if !embed_texts.is_empty() {
                match embedding::embed_texts(&embed_texts) {
                    Ok(vectors) => {
                        for (sym_id, vector) in embed_ids.iter().zip(vectors) {
                            if let Err(e) = queries.update_symbol_embedding(sym_id, vector).await {
                                debug!(error = %e, sym_id = %sym_id, "code graph sync: embedding write-back failed");
                            }
                        }
                        debug!(
                            count = embed_ids.len(),
                            "code graph sync: generated and stored embeddings for changed symbols"
                        );
                    }
                    Err(e) => {
                        debug!(error = %e, "code graph sync: embedding unavailable, skipping");
                    }
                }
            }

            // ── Recreate edges from parse result ────────────────────────
            for edge in &parse_result.edges {
                match edge {
                    ExtractedEdge::Calls {
                        caller,
                        callee,
                        is_method,
                        is_qualified,
                        raw_qualifier,
                        qualifier_kind,
                    } => {
                        if *is_method || *is_qualified {
                            // Sync stages qualified/method provenance but does
                            // NOT run the canonical post-pass here: canonical
                            // resolution is workspace-global (O(all staged
                            // calls)) and is deliberately deferred to the
                            // full-index path (082 perf gate), mirroring how the
                            // base singleton resolver also only recreates
                            // cross-file edges on full index. On sync, this
                            // file's own prior canonical edges were already
                            // retracted by `retract_resolved_calls_edges_for_file`
                            // above; canonical edges from OTHER unchanged callers
                            // that a mod-mapping change could strand under a
                            // newly-unsafe prefix are swept after the loop (C8-1).
                            // Either way the outcome is fail-closed: edges
                            // reappear on the next full index, never left stale
                            // or false.
                            if !should_stage_provenance_call(
                                *is_method,
                                *is_qualified,
                                raw_qualifier,
                            ) {
                                continue;
                            }
                            if let Some(from_id) = find_function_id(&new_function_ids, caller) {
                                let enclosing_canonical_type =
                                    enclosing_canonical_type_for_function(
                                        &crates,
                                        rust_ctx.as_ref(),
                                        &unsafe_prefixes,
                                        caller,
                                    );
                                queries
                                    .put_staged_call_with_provenance(
                                        &from_id,
                                        callee,
                                        &rel_path,
                                        raw_qualifier,
                                        qualifier_kind,
                                        &enclosing_canonical_type,
                                    )
                                    .await?;
                            }
                            continue;
                        }
                        // Mirror the index-path behavior: resolve locally for a
                        // direct edge, else stage the cross-file call for the
                        // deferred post-pass (082.002-T).
                        match (
                            find_function_id(&new_function_ids, caller),
                            find_function_id(&new_function_ids, callee),
                        ) {
                            (Some(from_id), Some(to_id)) => {
                                if matches!(lang_enum, Language::Python)
                                    && py_shadow
                                        .as_ref()
                                        .is_some_and(|shadow| shadow.is_contested(callee, caller))
                                {
                                    queries
                                        .put_staged_call_with_provenance(
                                            &from_id,
                                            callee,
                                            &rel_path,
                                            "",
                                            "python_bare",
                                            "",
                                        )
                                        .await?;
                                } else {
                                    queries.create_calls_edge(&from_id, &to_id).await?;
                                }
                            }
                            (Some(from_id), None) => {
                                if matches!(lang_enum, Language::Python) {
                                    queries
                                        .put_staged_call_with_provenance(
                                            &from_id,
                                            callee,
                                            &rel_path,
                                            "",
                                            "python_bare",
                                            "",
                                        )
                                        .await?;
                                } else {
                                    queries.put_staged_call(&from_id, callee, &rel_path).await?;
                                }
                            }
                            _ => {}
                        }
                    }
                    ExtractedEdge::InheritsFrom {
                        struct_name,
                        trait_name,
                    } => {
                        if let Some(child_id) = find_class_id(&new_class_ids, struct_name) {
                            if let Some(parent_id) =
                                find_interface_id(&new_interface_ids, trait_name)
                            {
                                queries
                                    .create_inherits_edge(
                                        "class",
                                        &child_id,
                                        "interface",
                                        &parent_id,
                                    )
                                    .await?;
                            }
                        }
                    }
                    // Defines already created above; Imports are cross-file (deferred, counted).
                    ExtractedEdge::Imports { .. } => {
                        result.cross_file_edges_dropped += 1;
                    }
                    // Defines edge already handled during symbol upsert.
                    ExtractedEdge::Defines { .. } => {}
                    // SQL References: resolve target to a Class node or self-loop (033.001-T).
                    ExtractedEdge::References { target, .. } => {
                        let resolved_id = queries.resolve_reference_target(target).await?;
                        if let Some(class_id) = resolved_id {
                            queries
                                .create_references_edge(&file_id, &class_id, Some(target))
                                .await?;
                        } else {
                            queries
                                .create_references_edge(&file_id, &file_id, Some(target))
                                .await?;
                        }
                        result.edges_created += 1;
                    }
                }
            }

            // ── Delete old concerns edges before relinking (prevent duplicates) ──
            for edge in &enriched_concerns {
                let _ = queries
                    .delete_concerns_edges_for_symbol(&edge.symbol_table, &edge.symbol_id)
                    .await;
            }

            // ── Relink concerns edges (FR-124) ──────────────────────────
            let (relinked, orphaned) = relink_concerns_edges(
                &queries,
                &enriched_concerns,
                &new_function_ids,
                &new_class_ids,
                &new_interface_ids,
            )
            .await?;
            result.concerns_relinked += relinked;
            result.concerns_orphaned += orphaned;

            if is_new {
                result.files_added += 1;
            } else {
                result.files_modified += 1;
            }
            debug!(path = %rel_path, "code graph sync: re-indexed file");
        }
        advance_progress(&mut progress, &mut completed_files, total_files);
    }

    // ── C8-1: sweep stale canonical edges after a mod-mapping change ─
    // An incremental sync only retracts the CHANGED files' own edges, so a
    // `#[path]`/`#[cfg]` mod-declaration edit that makes a module prefix newly
    // UNSAFE would otherwise leave STALE canonical edges from OTHER unchanged
    // callers under that prefix (the full-index canonical post-pass that drops
    // edges under unsafe prefixes is deliberately deferred on sync for perf).
    // Retract ALL canonical edges so none can survive under a now-unsafe prefix;
    // they are re-derived (correctly prefix-filtered) on the next full index —
    // fail-closed, matching the sync contract that canonical edges reappear on
    // full index and are never left stale.
    if mod_mapping_changed {
        let retracted = queries.retract_all_calls_resolved_canonical_edges().await?;
        if retracted > 0 {
            debug!(
                count = retracted,
                "code graph sync: swept canonical edges after a mod-mapping change (fail-closed; re-derived on next full index)"
            );
        }
    }

    // ── Post-pass: re-resolve unresolved references edges ───────────
    // Same ordering issue as index_workspace: a reference may be processed
    // before its target class exists. Re-resolve self-loops now all symbols exist.
    let reresolved = queries.reresolve_references_edges().await?;
    if reresolved.resolved > 0 {
        debug!(
            count = reresolved.resolved,
            "code graph sync: re-resolved deferred references edges"
        );
    }
    if previous_canonical_workspace.as_ref() == Some(&canonical_workspace) {
        queries
            .replace_index_canonical_workspace_snapshot(&canonical_workspace)
            .await?;
    } else {
        debug!(
            had_previous = previous_canonical_workspace.is_some(),
            "code graph sync: canonical workspace context drifted or lacks a full-index baseline; retrieval-eval collapse remains disabled"
        );
    }

    // ── Record sync summary ──────────────────────────────────────────
    let sync_summary = format!(
        "Code graph sync: {} modified, {} added, {} deleted, {} unchanged",
        result.files_modified, result.files_added, result.files_deleted, result.files_unchanged,
    );
    info!("{sync_summary}");

    #[allow(clippy::cast_possible_truncation)]
    let elapsed = start.elapsed().as_millis() as u64;
    result.duration_ms = elapsed;

    Ok(result)
}

/// Handle removal of an indexed file from the code graph.
///
/// Used when a file is deleted from disk or when we intentionally evict stale
/// indexed state, such as when a previously indexed file now exceeds the size
/// policy and should disappear from search and graph results.
///
/// Returns the number of concerns edges orphaned.
async fn handle_deleted_file(
    queries: &CodeGraphQueries,
    file_path: &str,
    file_id: &str,
) -> Result<usize, EngramError> {
    // Collect and delete concerns edges targeting symbols in this file.
    let concerns = queries.get_concerns_edges_for_file(file_path).await?;
    let mut orphaned = 0;
    for edge in &concerns {
        let deleted = queries
            .delete_concerns_edges_for_symbol(&edge.symbol_table, &edge.symbol_id)
            .await?;
        orphaned += deleted;
    }

    // Delete all symbol nodes, outbound file edges, and metadata for this file.
    // 082.009-T: retract this file's calls_resolved_singleton edges and clear
    // its staged calls WHILE the symbol IDs still exist, before deleting the
    // function metadata they are keyed against.
    queries
        .retract_resolved_calls_edges_for_file(file_path)
        .await?;
    queries.clear_staged_calls_for_file(file_path).await?;
    queries.delete_functions_by_file(file_path).await?;
    queries.delete_classes_by_file(file_path).await?;
    queries.delete_interfaces_by_file(file_path).await?;
    queries.delete_edges_from_file("defines", file_id).await?;
    queries
        .delete_edges_from_file("references", file_id)
        .await?;
    queries.delete_code_file(file_path).await?;
    queries.delete_file_hash_by_path(file_path).await?;

    if orphaned > 0 {
        warn!(
            file_path,
            orphaned, "code graph: orphaned concerns edges from removed file"
        );
    }

    Ok(orphaned)
}

/// Re-link `concerns` edges after re-indexing a modified file (FR-124).
///
/// For each concerns edge that existed before the re-index:
///   - If a new symbol with the same `(name, body_hash)` exists in ANY file,
///     re-create the concerns edge pointing to the new symbol ID.
///   - If no match is found, the edge is orphaned and removed.
///
/// Returns `(relinked, orphaned)` counts.
async fn relink_concerns_edges(
    queries: &CodeGraphQueries,
    pre_sync_concerns: &[crate::db::queries::ConcernsEdgeInfo],
    new_function_ids: &[(String, String)],
    new_class_ids: &[(String, String)],
    new_interface_ids: &[(String, String)],
) -> Result<(usize, usize), EngramError> {
    let mut relinked = 0;
    let mut orphaned = 0;

    for edge in pre_sync_concerns {
        if edge.symbol_name.is_empty() {
            // Cannot relink without a name — treat as orphan.
            orphaned += 1;
            continue;
        }

        // Try to find the new symbol by (name, body_hash) across all tables.
        let matches = queries
            .find_symbols_by_name_and_hash(&edge.symbol_name, &edge.symbol_body_hash)
            .await?;

        if matches.is_empty() {
            // Also try within the same file's new symbols by name only
            // (the symbol may have been modified, changing its body_hash).
            let in_file_match = match edge.symbol_table.as_str() {
                "function" => find_function_id(new_function_ids, &edge.symbol_name),
                "class" => find_class_id(new_class_ids, &edge.symbol_name),
                "interface" => find_interface_id(new_interface_ids, &edge.symbol_name),
                _ => None,
            };

            if let Some(new_id) = in_file_match {
                // Re-link to the new symbol (same name, different body).
                queries
                    .create_concerns_edge(
                        &edge.task_id,
                        &edge.symbol_table,
                        &new_id,
                        &edge.linked_by,
                    )
                    .await?;
                relinked += 1;
                debug!(
                    task = %edge.task_id,
                    symbol = %edge.symbol_name,
                    "concerns edge re-linked (name match, body changed)"
                );
            } else {
                orphaned += 1;
                warn!(
                    task = %edge.task_id,
                    symbol = %edge.symbol_name,
                    "concerns edge orphaned — symbol no longer exists"
                );
            }
        } else {
            // Re-link to the first matching new symbol.
            let target = &matches[0];
            queries
                .create_concerns_edge(&edge.task_id, &target.table, &target.id, &edge.linked_by)
                .await?;
            relinked += 1;
            debug!(
                task = %edge.task_id,
                symbol = %edge.symbol_name,
                new_path = %target.file_path,
                "concerns edge re-linked via hash-resilient identity"
            );
        }
    }

    Ok((relinked, orphaned))
}

/// Discover all source files in the workspace using `.gitignore`-aware traversal.
pub(crate) fn discover_files(ws_path: &Path, config: &CodeGraphConfig) -> Vec<std::path::PathBuf> {
    let mut builder = ignore::WalkBuilder::new(ws_path);
    builder
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .follow_links(false);

    // Add custom exclude patterns from config using a single OverrideBuilder.
    if !config.exclude_patterns.is_empty() {
        let mut ob = ignore::overrides::OverrideBuilder::new(ws_path);
        for pattern in &config.exclude_patterns {
            // Ignore patterns that fail to parse; log and continue.
            if ob.add(&format!("!{pattern}")).is_err() {
                warn!(pattern = %pattern, "code graph: invalid exclude pattern, skipping");
            }
        }
        match ob.build() {
            Ok(overrides) => {
                builder.overrides(overrides);
            }
            Err(e) => {
                warn!(error = %e, "code graph: failed to build exclude overrides, patterns ignored");
            }
        }
    }

    let supported: std::collections::HashSet<&str> = config
        .supported_languages
        .iter()
        .map(String::as_str)
        .collect();

    let mut files = Vec::new();
    for entry in builder.build().flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let lang = language_from_path(path);
        if supported.contains(lang.as_str()) {
            files.push(path.to_path_buf());
        }
    }

    files.sort();
    files
}

/// Map a file extension to a language identifier.
///
/// This is the **canonical** language vocabulary: the indexer stores the result
/// in `file_node.language`, and the retrieval-eval semantic gate
/// ([`crate::services::retrieval_eval::language_of`]) delegates here so the
/// semantic and graph gates never diverge (084.005-T and its generalization).
/// Unrecognized extensions fall through to the raw extension; a path with no
/// extension is `"unknown"`.
pub(crate) fn language_from_path(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| match ext {
            "rs" => "rust",
            "py" => "python",
            "js" | "jsx" => "javascript",
            "ts" => "typescript",
            "tsx" => "tsx",
            "go" => "go",
            "cs" => "csharp",
            "c" | "h" => "c",
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" | "h++" => "cpp",
            "swift" => "swift",
            "sql" => "sql",
            "kt" | "kts" => "kotlin",
            "md" => "markdown",
            _ => ext,
        })
        .unwrap_or("unknown")
        .to_owned()
}

/// Determine tier classification based on token count.
///
/// Returns `(embed_type, summary_text)`.
fn tier_classification(
    token_count: usize,
    token_limit: usize,
    body: &str,
    signature: &str,
    docstring: Option<&str>,
) -> (&'static str, String) {
    if token_count <= token_limit {
        ("explicit_code", body.to_owned())
    } else {
        let summary = match docstring {
            Some(doc) if !doc.is_empty() => format!("{signature}\n\n{doc}"),
            _ => {
                // No docstring: include first 5 lines / 256 chars of body as preview.
                let preview: String = body.lines().take(5).collect::<Vec<_>>().join("\n");
                let preview = if preview.len() > 256 {
                    // Safe truncation at char boundary.
                    let end = preview
                        .char_indices()
                        .nth(256)
                        .map_or(preview.len(), |(i, _)| i);
                    &preview[..end]
                } else {
                    &preview
                };
                if preview.is_empty() {
                    signature.to_owned()
                } else {
                    format!("{signature}\n\n{preview}")
                }
            }
        };
        ("summary_pointer", summary)
    }
}

/// SHA-256 hex digest of a string.
fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Short SHA-256 for IDs (first 16 hex chars).
fn sha256_short(input: &str) -> String {
    sha256_hex(input)[..16].to_owned()
}

/// Find a function ID by name.
fn find_function_id(ids: &[(String, String)], name: &str) -> Option<String> {
    ids.iter()
        .find(|(n, _)| n == name)
        .map(|(_, id)| id.clone())
}

/// Find a class ID by name.
fn find_class_id(ids: &[(String, String)], name: &str) -> Option<String> {
    ids.iter()
        .find(|(n, _)| n == name)
        .map(|(_, id)| id.clone())
}

/// Find an interface ID by name.
fn find_interface_id(ids: &[(String, String)], name: &str) -> Option<String> {
    ids.iter()
        .find(|(n, _)| n == name)
        .map(|(_, id)| id.clone())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct CanonicalFixtureOutput {
        unsafe_prefixes: BTreeSet<String>,
        canonical_edges: BTreeSet<(String, String)>,
    }

    fn cached_context(crate_name: &str) -> RustCanonicalContext {
        (
            canonical::ModulePath {
                crate_name: crate_name.to_owned(),
                segments: Vec::new(),
            },
            canonical::UseGraph::default(),
        )
    }

    fn write_file_result(ws: &Path, rel: &str, content: &str) -> std::io::Result<()> {
        let full = ws.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full, content)
    }

    fn write_manifest_result(ws: &Path) -> std::io::Result<()> {
        write_file_result(
            ws,
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
    }

    fn repo_local_temp_root() -> anyhow::Result<PathBuf> {
        let root = std::env::current_dir()?
            .join("target")
            .join("prepass-cache-tests");
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    fn test_db_params(path: &Path) -> (PathBuf, String) {
        let branch_source = path.to_string_lossy().to_lowercase();
        let branch = format!("{:x}", Sha256::digest(branch_source.as_bytes()));
        (path.join(".engram-test-data"), branch)
    }

    async fn stable_canonical_edges(
        queries: &CodeGraphQueries,
    ) -> anyhow::Result<BTreeSet<(String, String)>> {
        let functions_by_id: HashMap<String, String> = queries
            .all_functions()
            .await?
            .into_iter()
            .map(|function| {
                (
                    function.id,
                    format!(
                        "{}:{}:{}-{}",
                        function.file_path, function.name, function.line_start, function.line_end
                    ),
                )
            })
            .collect();
        let mut edges = BTreeSet::new();
        for (from, to) in queries
            .list_calls_edges_by_resolution("calls_resolved_canonical")
            .await?
        {
            let from_identity = functions_by_id
                .get(&from)
                .ok_or_else(|| anyhow::anyhow!("missing caller id {from}"))?
                .clone();
            let to_identity = functions_by_id
                .get(&to)
                .ok_or_else(|| anyhow::anyhow!("missing callee id {to}"))?
                .clone();
            edges.insert((from_identity, to_identity));
        }
        Ok(edges)
    }

    async fn index_canonical_fixture(
        force_prepass_cache_miss: bool,
    ) -> anyhow::Result<CanonicalFixtureOutput> {
        let temp_root = repo_local_temp_root()?;
        let tmp = tempfile::Builder::new()
            .prefix("prepass-cache-")
            .tempdir_in(temp_root)?;
        let ws = tmp.path();
        write_manifest_result(ws)?;
        write_file_result(
            ws,
            "src/lib.rs",
            "#[path = \"actual.rs\"]\npub mod remapped;\npub mod caller;\npub mod util;\npub mod widget;\n",
        )?;
        write_file_result(ws, "src/actual.rs", "pub fn target() {}\n")?;
        write_file_result(ws, "src/remapped.rs", "pub fn target() {}\n")?;
        write_file_result(ws, "src/util.rs", "pub fn helper() {}\n")?;
        write_file_result(
            ws,
            "src/widget.rs",
            "pub struct Widget;\nimpl Widget { pub fn build() {} }\n",
        )?;
        write_file_result(
            ws,
            "src/caller.rs",
            "pub fn caller() { crate::util::helper(); crate::widget::Widget::build(); }\npub fn second() { crate::util::helper(); }\n",
        )?;

        let config = CodeGraphConfig::default();
        let (data_dir, branch) = test_db_params(ws);
        index_workspace_impl(
            ws,
            &data_dir,
            &branch,
            &config,
            false,
            None,
            force_prepass_cache_miss,
        )
        .await?;
        let db = connect_db(&data_dir, &branch).await?;
        let queries = CodeGraphQueries::new(db);
        let snapshot = queries
            .load_index_canonical_workspace_snapshot()
            .await?
            .ok_or_else(|| anyhow::anyhow!("missing canonical workspace snapshot"))?;
        let unsafe_prefixes = snapshot.unsafe_prefixes.into_iter().collect();
        let canonical_edges = stable_canonical_edges(&queries).await?;
        Ok(CanonicalFixtureOutput {
            unsafe_prefixes,
            canonical_edges,
        })
    }

    #[test]
    fn matching_prepass_cache_reuses_context_without_recomputing() {
        let mut rust_contexts = HashMap::new();
        rust_contexts.insert(
            "src/lib.rs".to_owned(),
            CachedRustCanonicalContext {
                content_hash: "cached-hash".to_owned(),
                context: Some(cached_context("cached")),
            },
        );

        let context =
            rust_ctx_from_prepass_cache(&rust_contexts, "src/lib.rs", "cached-hash", false, || {
                panic!("cache hits must not recompute canonical context")
            });

        assert_eq!(
            context.map(|(module, _)| module.to_canonical()),
            Some("cached".to_owned())
        );
    }

    #[test]
    fn stale_prepass_cache_recomputes_context() {
        let mut rust_contexts = HashMap::new();
        rust_contexts.insert(
            "src/lib.rs".to_owned(),
            CachedRustCanonicalContext {
                content_hash: "old-hash".to_owned(),
                context: Some(cached_context("cached")),
            },
        );

        let context =
            rust_ctx_from_prepass_cache(&rust_contexts, "src/lib.rs", "new-hash", false, || {
                Some(cached_context("recomputed"))
            });

        assert_eq!(
            context.map(|(module, _)| module.to_canonical()),
            Some("recomputed".to_owned())
        );
    }

    #[test]
    fn forced_prepass_cache_miss_recomputes_matching_hash() {
        let mut rust_contexts = HashMap::new();
        rust_contexts.insert(
            "src/lib.rs".to_owned(),
            CachedRustCanonicalContext {
                content_hash: "cached-hash".to_owned(),
                context: Some(cached_context("cached")),
            },
        );

        let context =
            rust_ctx_from_prepass_cache(&rust_contexts, "src/lib.rs", "cached-hash", true, || {
                Some(cached_context("forced-recompute"))
            });

        assert_eq!(
            context.map(|(module, _)| module.to_canonical()),
            Some("forced-recompute".to_owned()),
            "control: the forced-miss seam must exercise the recompute branch even when hashes match"
        );
    }

    #[tokio::test]
    async fn forced_prepass_cache_miss_preserves_prefixes_and_edges() -> anyhow::Result<()> {
        let cache_reuse = index_canonical_fixture(false).await?;
        let recompute_fallback = index_canonical_fixture(true).await?;

        assert!(
            cache_reuse.unsafe_prefixes.contains("demo::remapped"),
            "fixture must keep unsafe prefixes non-empty"
        );
        assert!(
            !cache_reuse.canonical_edges.is_empty(),
            "fixture must produce canonical edges so equivalence is non-vacuous"
        );
        assert_eq!(
            cache_reuse, recompute_fallback,
            "cache reuse and forced recompute fallback must index identical unsafe prefixes and canonical edges"
        );
        Ok(())
    }
}
