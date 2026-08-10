//! Tree-sitter Python grammar parser.
//!
//! Extracts top-level functions, classes, and import edges from Python source
//! files. Method bodies are not yet indexed (Tier 1 implementation).

use tree_sitter::{Node, Parser};

use super::python_canonical::{
    BindingKind, CallResolution, ImportBindings, extract_python_import_bindings,
};
use super::{
    ExtractedClass, ExtractedEdge, ExtractedFunction, ExtractedInterface, ExtractedSymbol,
    ParseResult,
};

/// Parse a Python source file and extract symbols and edges.
///
/// # Errors
///
/// Returns [`crate::errors::EngramError`] if the grammar cannot be loaded or
/// tree-sitter fails to produce a valid parse tree.
pub(super) fn parse_python_source(source: &str) -> Result<ParseResult, crate::errors::EngramError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .map_err(|e| {
            crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
                reason: format!("Failed to set Python grammar: {e}"),
            })
        })?;

    let tree = parser.parse(source, None).ok_or_else(|| {
        crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
            reason: "tree-sitter returned no parse tree for Python source".to_owned(),
        })
    })?;

    let root = tree.root_node();
    let mut symbols = Vec::new();
    let mut edges = Vec::new();

    // 099.004-T (P1-760): capture the file's import bindings once so
    // `extract_calls_from_body` can promote a PROVABLE function-local import
    // (order-aware, fail-closed) to an exact canonical target.
    let bindings = extract_python_import_bindings(source);
    extract_top_level(root, source, &bindings, &mut symbols, &mut edges);

    Ok(ParseResult { symbols, edges })
}

fn extract_top_level(
    root: Node<'_>,
    source: &str,
    bindings: &ImportBindings,
    symbols: &mut Vec<ExtractedSymbol>,
    edges: &mut Vec<ExtractedEdge>,
) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                if let Some(func) = extract_function(child, source) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: func.name.clone(),
                    });
                    // Attribute call edges only to the owning top-level function
                    // (mirrors rust.rs placement after the Defines push).
                    extract_calls_from_body(child, source, &func.name, bindings, edges);
                    symbols.push(ExtractedSymbol::Function(func));
                }
            }
            "class_definition" => {
                if let Some(class) = extract_class(child, source) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: class.name.clone(),
                    });
                    symbols.push(ExtractedSymbol::Class(class));
                }
            }
            "import_statement" | "import_from_statement" => {
                edges.push(ExtractedEdge::Imports {
                    import_path: extract_import(child, source),
                });
            }
            _ => {}
        }
    }
}

fn extract_function(node: Node<'_>, source: &str) -> Option<ExtractedFunction> {
    let name = node
        .child_by_field_name("name")
        .map(|n| super::node_text(n, source))?;

    let body = super::node_text(node, source);
    let body_hash = super::sha256_hex(&body);
    #[allow(clippy::cast_possible_truncation)]
    let line_start = (node.start_position().row + 1) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let line_end = (node.end_position().row + 1) as u32;
    let signature = extract_signature(node, source);
    let docstring = extract_docstring(node, source);
    #[allow(clippy::cast_possible_truncation)]
    let token_count = (body.len() / 4) as u32;

    Some(ExtractedFunction {
        name,
        line_start,
        line_end,
        signature,
        docstring,
        body,
        body_hash,
        token_count,
    })
}

fn extract_class(node: Node<'_>, source: &str) -> Option<ExtractedClass> {
    let name = node
        .child_by_field_name("name")
        .map(|n| super::node_text(n, source))?;

    let body = super::node_text(node, source);
    let body_hash = super::sha256_hex(&body);
    #[allow(clippy::cast_possible_truncation)]
    let line_start = (node.start_position().row + 1) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let line_end = (node.end_position().row + 1) as u32;
    let docstring = extract_docstring(node, source);
    #[allow(clippy::cast_possible_truncation)]
    let token_count = (body.len() / 4) as u32;

    // Python classes do not map to an interface concept; use Class.
    Some(ExtractedClass {
        name,
        line_start,
        line_end,
        docstring,
        body,
        body_hash,
        token_count,
    })
}

// Suppress dead_code: ExtractedInterface imported for type coherence with other parsers.
#[allow(dead_code)]
fn _use_interface(_: ExtractedInterface) {}

fn extract_import(node: Node<'_>, source: &str) -> String {
    super::node_text(node, source)
}

fn extract_signature(node: Node<'_>, source: &str) -> String {
    // Python signature: everything up to (but not including) the body block.
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" {
            let sig_end = child.start_byte();
            let sig_start = node.start_byte();
            return source[sig_start..sig_end].trim().to_owned();
        }
    }
    super::node_text(node, source)
}

fn extract_docstring(node: Node<'_>, source: &str) -> Option<String> {
    // Python docstrings are the first expression_statement child with a string.
    let body_node = node.child_by_field_name("body")?;
    let mut cursor = body_node.walk();
    if let Some(child) = body_node.children(&mut cursor).next() {
        if child.kind() == "expression_statement" {
            if let Some(string_node) = child.child(0) {
                if string_node.kind() == "string" {
                    let raw = super::node_text(string_node, source);
                    let cleaned = raw
                        .trim_start_matches("\"\"\"")
                        .trim_end_matches("\"\"\"")
                        .trim_start_matches("'''")
                        .trim_end_matches("'''")
                        .trim()
                        .to_owned();
                    return Some(cleaned);
                }
            }
        }
    }
    None
}

/// Builtin/idiomatic Python callees that add graph noise without navigational
/// value. Mirrors the intent of `rust.rs`'s `CALL_BLOCKLIST`. Conservative by
/// design; tuned via integration/eval evidence rather than assumption.
const PYTHON_CALL_BLOCKLIST: &[&str] = &[
    "print",
    "len",
    "str",
    "int",
    "float",
    "bool",
    "list",
    "dict",
    "set",
    "tuple",
    "range",
    "super",
    "isinstance",
    "issubclass",
    "getattr",
    "setattr",
    "hasattr",
    "enumerate",
    "zip",
    "map",
    "filter",
    "open",
    "type",
    "repr",
    "format",
    "sorted",
    "sum",
    "min",
    "max",
    "abs",
    "next",
    "iter",
    "id",
    "vars",
    "dir",
];

/// A resolved Python call site. `is_qualified` is set only for a
/// module-qualified call `mod.func()` whose receiver `mod` is a simple
/// identifier that is not `self`/`cls` (096-F/T4); the resolver later fails
/// closed if `mod` is not a bound module. Python has no Rust-style `::` path
/// form, so no `scoped_*` helpers are needed.
struct ResolvedCallName {
    callee: String,
    is_method: bool,
    is_qualified: bool,
    raw_qualifier: String,
    qualifier_kind: String,
}

/// DFS over a top-level function's BODY emitting `Calls` edges, stopping at
/// nested `function_definition`, `lambda`, and `class_definition` boundaries so
/// calls are attributed only to their owning top-level function.
///
/// The walk is seeded with the children of the function's `body` field only.
/// Parameter default values, parameter/return annotations, and decorators are
/// intentionally excluded: their calls (e.g. `def f(x=build_default()): ...`)
/// run at DEFINITION time in the enclosing scope, not when the function
/// executes, so attributing them to this function would emit a false edge
/// (013-D no-false-edge invariant). A function with no `body` field yields no
/// edges (fails closed, panic-free).
fn extract_calls_from_body(
    node: Node<'_>,
    source: &str,
    caller_name: &str,
    bindings: &ImportBindings,
    edges: &mut Vec<ExtractedEdge>,
) {
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };
    let mut stack: Vec<Node<'_>> = Vec::new();
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        stack.push(child);
    }
    while let Some(current) = stack.pop() {
        // Do not descend into nested callable/class scopes: their calls belong
        // to that inner scope, not the owning top-level function.
        if matches!(
            current.kind(),
            "function_definition" | "lambda" | "class_definition"
        ) {
            continue;
        }
        if current.kind() == "call" {
            if let Some(mut call) = resolve_call_name(current, source) {
                promote_function_local_import(&mut call, current.start_byte(), bindings);
                edges.push(ExtractedEdge::Calls {
                    caller: caller_name.to_owned(),
                    callee: call.callee,
                    is_method: call.is_method,
                    is_qualified: call.is_qualified,
                    raw_qualifier: call.raw_qualifier,
                    qualifier_kind: call.qualifier_kind,
                });
            }
        }
        let mut child_cursor = current.walk();
        for child in current.children(&mut child_cursor) {
            stack.push(child);
        }
    }
}

/// 099.004-T (P1-760) — promote a call whose name is a PROVABLE function-local
/// import to an exact canonical target, resolved through
/// [`ImportBindings::resolve_call`].
///
/// `resolve_call` returns `LocalImport` only for a firm, order-correct
/// function-local import in the enclosing top-level function scope (F1). Every
/// adversarial vector — use-before-import, a conditional/`try`-guarded import, a
/// name rebound after its import, or a `global`/`nonlocal` dynamic rebind —
/// returns `Poisoned`/`ModuleScope`, leaving the call untouched so it stays
/// fail-closed (013-D). Top-level module imports also resolve to `ModuleScope`
/// and keep flowing through the existing `"module"` / `"python_bare"` resolver
/// arms, so this pass is purely additive for the function-local case.
///
/// On a hit the call is re-encoded as `qualifier_kind:"python_local"` with the
/// full canonical dotted target in `raw_qualifier`; `python_target_for_staged_call`
/// then trusts it directly (firmness + order were already proven here).
fn promote_function_local_import(
    call: &mut ResolvedCallName,
    call_position: usize,
    bindings: &ImportBindings,
) {
    match call.qualifier_kind.as_str() {
        // Bare call `f()` originating from `from m import f` → canonical "m.f".
        "" => {
            if let CallResolution::LocalImport(binding) =
                bindings.resolve_call(call_position, &call.callee)
            {
                if binding.kind == BindingKind::FromImportSymbol {
                    call.is_qualified = true;
                    call.raw_qualifier = binding.canonical_path.clone();
                    "python_local".clone_into(&mut call.qualifier_kind);
                }
            }
        }
        // Module-qualified call `m.f()` originating from `import m` → "m.f".
        "module" => {
            if let CallResolution::LocalImport(binding) =
                bindings.resolve_call(call_position, &call.raw_qualifier)
            {
                if binding.kind == BindingKind::ModuleImport {
                    call.raw_qualifier = format!("{}.{}", binding.canonical_path, call.callee);
                    "python_local".clone_into(&mut call.qualifier_kind);
                }
            }
        }
        _ => {}
    }
}

/// Classify a Python `call` node's `function` child.
///
/// * `identifier` (`foo()`) → bare call, promoted (`is_method:false`).
/// * `attribute` (`mod.foo()`, `self.bar()`, `obj.attr.y()`) → the callee is the
///   `attribute` field text (NOT Rust's `field`). Its `object` (receiver)
///   decides staging (096-F/T4, M2):
///   * a **simple identifier** `r` that is **not** `self`/`cls` → a candidate
///     module-qualified call: `is_method:false, is_qualified:true,
///     raw_qualifier:r, qualifier_kind:"module"` (the resolver fails closed if
///     `r` is not a bound module).
///   * `self`/`cls` (instance/class receivers, need type inference) → dropped
///     (`is_method:true`, EMPTY `raw_qualifier`, so
///     `should_stage_provenance_call(true, false, "")` returns `false`).
///   * a **non-simple-identifier** receiver (`obj.attr.y()`, `a().b()`) → also
///     dropped with an empty qualifier (fails closed).
/// * any other kind (`subscript` `d[k]()`, chained `a().b()` whose function is a
///   call) → skipped in v1 (`None`), forward-compatible and panic-free.
///
/// Blocklisted callees resolve to `None`.
fn resolve_call_name(node: Node<'_>, source: &str) -> Option<ResolvedCallName> {
    let function_node = node.child_by_field_name("function")?;
    let call = match function_node.kind() {
        "identifier" => ResolvedCallName {
            callee: super::node_text(function_node, source),
            is_method: false,
            is_qualified: false,
            raw_qualifier: String::new(),
            qualifier_kind: String::new(),
        },
        "attribute" => {
            let callee = function_node
                .child_by_field_name("attribute")
                .map(|n| super::node_text(n, source))?;
            // M2: a simple-identifier receiver that is not self/cls is a candidate
            // module-qualified call; every other receiver (self/cls, obj.attr.y(),
            // a().b()) stays dropped with an empty qualifier (fails closed).
            let module_receiver = function_node
                .child_by_field_name("object")
                .filter(|o| o.kind() == "identifier")
                .map(|o| super::node_text(o, source))
                .filter(|r| r.as_str() != "self" && r.as_str() != "cls");
            match module_receiver {
                Some(receiver) => ResolvedCallName {
                    callee,
                    is_method: false,
                    is_qualified: true,
                    raw_qualifier: receiver,
                    qualifier_kind: "module".to_owned(),
                },
                None => ResolvedCallName {
                    callee,
                    is_method: true,
                    is_qualified: false,
                    // Empty on purpose: fails closed at should_stage_provenance_call.
                    raw_qualifier: String::new(),
                    qualifier_kind: "method".to_owned(),
                },
            }
        }
        _ => return None,
    };
    if PYTHON_CALL_BLOCKLIST.contains(&call.callee.as_str()) {
        None
    } else {
        Some(call)
    }
}

/// PySpark data-lineage extraction (095-F, plan Unit U2a).
///
/// The pure extraction + authority-resolution half of the Python lineage path.
/// It recognises the whitelisted Spark read/write method chains, reads their
/// string-literal argument, and resolves it to an authority-bound
/// [`LineageEndpoint`] against the [`LineageAuthorityContext`]. It produces
/// call-level resolved data (role + endpoint + chain-root receiver + source
/// order + enclosing scope) — it does **not** link reads to writes, build
/// [`crate::models::lineage::LineageEdgeCandidate`]s, or persist anything. The
/// read→write dataflow join is U2b; the assignment/scope event stream is U2c.
///
/// Fail-closed (013-D): non-literal / f-string / concatenated args,
/// relative-path literals, 1-/2-part table names, config/widget-derived names,
/// and a 3-part literal with no trusted authority all yield **no** resolved
/// endpoint. `spark.sql(...)` is deferred out of v1 (removed from the
/// whitelist); `createOrReplaceTempView(...)` is content-only (no edge).
/// U2a→U2c→U2b lineage extractors, re-exported for the notebook router (U4a).
pub(crate) use spark_lineage::{extract_python_lineage, resolve_cell_candidates};

// `#[allow(dead_code)]`: this is the U2a → U2c seam. Its consumer, the
// `extract_python_lineage` event stream, arrives in U2c (095.013-T); until then
// the resolver + value types are exercised only by the U2a unit tests.
#[allow(dead_code)]
pub(super) mod spark_lineage {
    use tree_sitter::{Node, Parser};

    use crate::errors::{CodeGraphError, EngramError};
    use crate::models::lineage::{LineageAuthorityContext, LineageEndpoint};

    /// The lineage role of a whitelisted Spark call.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum SparkCallRole {
        /// A dataset read (`spark.table`, `spark.read.<fmt>`, `spark.read.load`).
        Read,
        /// A dataset write (`df.write.saveAsTable`, `df.write.save`).
        Write,
    }

    /// The enclosing lexical scope of a Spark call relative to the cell body
    /// (AR-07). Only [`LineageScope::TopLevel`] calls are eligible for v1 edges;
    /// nested calls are still resolved but flagged for the downstream drop.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum LineageScope {
        /// A direct statement of the cell/module body.
        TopLevel,
        /// Nested inside a block (if/for/while/with/try/def/class),
        /// comprehension, or lambda.
        Nested,
    }

    /// A whitelisted Spark read/write call resolved to call-level lineage data.
    ///
    /// This is the U2a → U2c seam. It is intentionally **not** a
    /// [`crate::models::lineage::LineageEdgeCandidate`]: candidates are produced
    /// downstream by U2b's single-cell dataflow join.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct ResolvedSparkCall {
        /// Whether this call reads or writes a dataset.
        pub role: SparkCallRole,
        /// The resolved authority-bound endpoint, or `None` when the argument
        /// fails the fail-closed predicate (013-D).
        pub endpoint: Option<LineageEndpoint>,
        /// The base simple-name at the chain root (AR-13), or `None` when the
        /// chain root is not a simple identifier.
        pub receiver: Option<String>,
        /// Source-order key (the call node's start byte offset).
        pub order: usize,
        /// The enclosing lexical scope of the call.
        pub scope: LineageScope,
    }

    /// Parse `source` and resolve every whitelisted Spark read/write call.
    ///
    /// # Errors
    ///
    /// Returns [`EngramError`] if the Python grammar cannot be loaded or
    /// tree-sitter fails to produce a parse tree.
    pub(crate) fn resolve_spark_calls(
        source: &str,
        authority_ctx: &LineageAuthorityContext,
    ) -> Result<Vec<ResolvedSparkCall>, EngramError> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .map_err(|e| {
                EngramError::CodeGraph(CodeGraphError::ParseFailed {
                    reason: format!("Failed to set Python grammar: {e}"),
                })
            })?;
        let tree = parser.parse(source, None).ok_or_else(|| {
            EngramError::CodeGraph(CodeGraphError::ParseFailed {
                reason: "tree-sitter returned no parse tree for Python source".to_owned(),
            })
        })?;

        let mut resolved = Vec::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "call" {
                if let Some(call) = resolve_one_call(node, source, authority_ctx) {
                    resolved.push(call);
                }
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                stack.push(child);
            }
        }
        resolved.sort_by_key(|c| c.order);
        Ok(resolved)
    }

    /// The dataset kind a whitelisted method targets — selects the resolver.
    #[derive(Clone, Copy)]
    enum TargetKind {
        Table,
        Path,
    }

    /// Terminal `spark.read.<method>` reader methods that yield a path endpoint.
    const READ_PATH_TERMINALS: &[&str] = &[
        "parquet", "csv", "json", "orc", "text", "avro", "jdbc", "delta", "load",
    ];

    /// Slice `source` by a node's byte range (self-contained; no `super::`).
    fn node_text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
        &source[node.byte_range()]
    }

    /// PySpark session identifiers trusted as lineage read roots.
    ///
    /// A lineage READ must originate from a genuine Spark session; any other
    /// object exposing a read-shaped method (e.g. a REST `client.table(...)`)
    /// fails closed so it can never mint a false read binding (C1, the
    /// zero-false-edge invariant). `spark` is the conventional PySpark
    /// entry-point name.
    const TRUSTED_SPARK_SESSIONS: &[&str] = &["spark"];

    /// Whether `root` is a trusted Spark-session identifier (C1).
    fn is_trusted_spark_session(root: &str) -> bool {
        TRUSTED_SPARK_SESSIONS.contains(&root)
    }

    /// Resolve a single `call` node to a [`ResolvedSparkCall`], or `None` when it
    /// is not a whitelisted Spark read/write chain.
    fn resolve_one_call(
        call: Node<'_>,
        source: &str,
        authority_ctx: &LineageAuthorityContext,
    ) -> Option<ResolvedSparkCall> {
        let function = call.child_by_field_name("function")?;
        if function.kind() != "attribute" {
            return None;
        }
        let terminal = node_text(function.child_by_field_name("attribute")?, source).to_owned();
        let (root, chain) = walk_chain(function, source);
        let (role, target) = classify_chain(&chain, &terminal)?;
        // C1 (zero-false-edge): a lineage READ must originate from a trusted
        // Spark session identifier. Any object exposing a read-shaped method
        // (e.g. `client.table("c.s.t")`) would otherwise mint a false read
        // binding that a downstream write could join into a spurious edge.
        if role == SparkCallRole::Read && !root.as_deref().is_some_and(is_trusted_spark_session) {
            return None;
        }
        let endpoint = first_positional_string(call, source)
            .and_then(|literal| resolve_endpoint(&literal, target, authority_ctx));
        Some(ResolvedSparkCall {
            role,
            endpoint,
            receiver: root,
            order: call.start_byte(),
            scope: scope_of(call),
        })
    }

    /// Walk the receiver object-chain of an attribute `function` node from the
    /// terminal down to the root, collecting the intermediate method names
    /// (root-outward, excluding the terminal) and the base simple-name root.
    ///
    /// Intermediate method calls (e.g. `.mode("x")`) are stepped through. When
    /// the chain root is a call result (a bare function call producing the
    /// receiver) or any non-identifier, the root name is `None` (AR-13).
    fn walk_chain(function: Node<'_>, source: &str) -> (Option<String>, Vec<String>) {
        let mut methods = Vec::new();
        let mut current = function.child_by_field_name("object");
        while let Some(node) = current {
            match node.kind() {
                "attribute" => {
                    if let Some(name) = node.child_by_field_name("attribute") {
                        methods.push(node_text(name, source).to_owned());
                    }
                    current = node.child_by_field_name("object");
                }
                "call" => match node.child_by_field_name("function") {
                    // An intermediate method call in the chain (e.g. `.mode(x)`).
                    Some(f) if f.kind() == "attribute" => current = Some(f),
                    // Root receiver is a call result → not a simple name (AR-13).
                    _ => {
                        methods.reverse();
                        return (None, methods);
                    }
                },
                "identifier" => {
                    let root = node_text(node, source).to_owned();
                    methods.reverse();
                    return (Some(root), methods);
                }
                _ => {
                    methods.reverse();
                    return (None, methods);
                }
            }
        }
        methods.reverse();
        (None, methods)
    }

    /// Match a method chain against the Spark read/write whitelist.
    ///
    /// `chain` is the root-outward list of intermediate method names (excluding
    /// the `terminal`). `spark.sql` is intentionally absent (deferred out of v1)
    /// and `createOrReplaceTempView` is not a write (content-only, no edge).
    fn classify_chain(chain: &[String], terminal: &str) -> Option<(SparkCallRole, TargetKind)> {
        let first = chain.first().map(String::as_str);
        match (first, terminal) {
            // `spark.table("c.s.t")` — read table (no intermediate methods).
            (None, "table") => Some((SparkCallRole::Read, TargetKind::Table)),
            // `spark.read.<fmt|load>(path)` — read path.
            (Some("read"), t) if READ_PATH_TERMINALS.contains(&t) => {
                Some((SparkCallRole::Read, TargetKind::Path))
            }
            // `df.write[.mode/.option/...].saveAsTable(table)` — write table.
            (Some("write"), "saveAsTable") => Some((SparkCallRole::Write, TargetKind::Table)),
            // `df.write[.mode/.option/...].save(path)` — write path.
            (Some("write"), "save") => Some((SparkCallRole::Write, TargetKind::Path)),
            _ => None,
        }
    }

    /// Resolve a string literal to an endpoint via the authority context.
    fn resolve_endpoint(
        literal: &str,
        target: TargetKind,
        authority_ctx: &LineageAuthorityContext,
    ) -> Option<LineageEndpoint> {
        match target {
            TargetKind::Table => authority_ctx.resolve_table(literal),
            TargetKind::Path => authority_ctx.resolve_path(literal),
        }
    }

    /// Read the first positional argument of a `call` as a plain string literal.
    ///
    /// Returns `None` (fail closed) for keyword args, f-strings, concatenated /
    /// binary-operator expressions, identifiers, or any non-`string` argument.
    fn first_positional_string(call: Node<'_>, source: &str) -> Option<String> {
        let args = call.child_by_field_name("arguments")?;
        let mut cursor = args.walk();
        let first = args
            .children(&mut cursor)
            .find(|child| !matches!(child.kind(), "(" | ")" | "," | "comment"))?;
        read_string_literal(first, source)
    }

    /// Extract the content of a plain (non-f, non-bytes, non-escaped) string
    /// literal.
    ///
    /// Fails closed for f-strings (an `f`/`F` prefix or any `interpolation`
    /// child), for bytes literals (a `b`/`B` prefix — T2), for any literal
    /// containing an escape sequence or backslash (T3, where the source spelling
    /// would differ from the runtime value), and for any node that is not a bare
    /// `string`.
    fn read_string_literal(node: Node<'_>, source: &str) -> Option<String> {
        if node.kind() != "string" {
            return None;
        }
        let mut cursor = node.walk();
        let mut content: Option<String> = None;
        for child in node.children(&mut cursor) {
            match child.kind() {
                "string_start" => {
                    let prefix = node_text(child, source);
                    // Fail closed for f-strings and bytes literals: neither names a
                    // plain `str` table/path the runtime would resolve (T2).
                    if prefix.contains('f')
                        || prefix.contains('F')
                        || prefix.contains('b')
                        || prefix.contains('B')
                    {
                        return None;
                    }
                }
                // An `interpolation` is an f-string fragment; an `escape_sequence`
                // means the source spelling differs from the runtime string value
                // (a wrong dataset identity, T3). Both fail closed rather than mint
                // a mislabeled/guessed edge.
                "interpolation" | "escape_sequence" => return None,
                "string_content" => {
                    let text = node_text(child, source);
                    // A raw string keeps backslashes literally in `string_content`;
                    // fail closed on any backslash for the same identity reason (T3).
                    if text.contains('\\') {
                        return None;
                    }
                    content = Some(text.to_owned());
                }
                _ => {}
            }
        }
        // A well-formed empty string ("") has no string_content child; treat as
        // empty, which every resolver rejects (fail closed anyway).
        Some(content.unwrap_or_default())
    }

    /// Determine whether `call` is a top-level statement or nested inside a
    /// block / comprehension / lambda (AR-07).
    fn scope_of(call: Node<'_>) -> LineageScope {
        let mut current = call.parent();
        while let Some(node) = current {
            match node.kind() {
                "module" => return LineageScope::TopLevel,
                "block"
                | "list_comprehension"
                | "dictionary_comprehension"
                | "set_comprehension"
                | "generator_expression"
                | "lambda" => {
                    return LineageScope::Nested;
                }
                _ => {}
            }
            current = node.parent();
        }
        LineageScope::TopLevel
    }

    // ── U2c: assignment/scope event emission (SparkLineageEvent stream) ────────

    /// The lexical form a lineage-relevant statement/binding came from (AR-07).
    ///
    /// Records which form each event came from so U2b can honor top-level
    /// binds/writes and treat every other form as an invalidation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum EventScope {
        /// A direct statement of the module/cell body (the only edge-eligible
        /// scope).
        TopLevel,
        /// Inside an `if`/`elif`/`else` branch block.
        Branch,
        /// Inside a `for`/`while` loop block.
        Loop,
        /// Inside a comprehension or generator expression.
        Comprehension,
        /// Inside a `with`/`try`/`except`/`finally` block, or a `with`/`except`
        /// `as` target.
        WithExcept,
        /// Inside a nested `def`/`class` block, or a `def`/`class` name binding.
        NestedDef,
        /// An augmented assignment (`df += …`).
        AugmentedAssign,
        /// A walrus binding (`df := …`).
        Walrus,
        /// A `del` of the name.
        Del,
        /// An `import` binding of the name.
        Import,
        /// A `for`-statement loop-target binding.
        ForTarget,
        /// A `with`-item `as` target binding.
        WithTarget,
        /// Inside a `lambda` body — never top-level, so a read/write there is
        /// not edge-eligible (AR-07 / C2).
        Lambda,
    }

    /// A tagged assignment/scope lineage event — the shared U2 → U2b contract.
    ///
    /// Every kind carries a source-order key and enclosing [`EventScope`] so
    /// U2b's single-cell dataflow join can honor top-level binds/writes (AR-07)
    /// and invalidate a tracked binding on any other rebind form (F2).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) enum SparkLineageEvent {
        /// A resolved Spark read bound to a receiver variable as ONE atomic
        /// event (`df = spark.read.…(literal)` / `df = spark.table(literal)`).
        /// The assignment *is* the read, so it never self-invalidates (AR-02).
        ReadBind {
            /// The assignment-target variable the read is bound to.
            variable: String,
            /// The read chain-root session receiver (e.g. `spark`), or `None`
            /// when the root is not a simple name. U2b drops the bind when this
            /// session was invalidated by a prior rebind (`spark = other`,
            /// AR-29).
            session: Option<String>,
            /// The U2a-resolved read endpoint, or `None` (fail closed).
            endpoint: Option<LineageEndpoint>,
            /// Source-order key (statement start byte).
            order: usize,
            /// Enclosing scope form (AR-07).
            scope: EventScope,
        },
        /// A Spark write call (`df.write.…(literal)`), carrying the chain-root
        /// base simple-name receiver (AR-13; `None` when the root is not a
        /// simple name).
        WriteCall {
            /// The chain-root base simple-name receiver, or `None`.
            receiver: Option<String>,
            /// The U2a-resolved target endpoint, or `None` (fail closed).
            endpoint: Option<LineageEndpoint>,
            /// Source-order key (call start byte).
            order: usize,
            /// Enclosing scope form (AR-07).
            scope: EventScope,
        },
        /// A non-Spark (re)binding / invalidation of a tracked name (`df =
        /// other`, `df = compute()`, augmented/walrus/del/import/for/with/
        /// comprehension rebind). Lets U2b invalidate a prior binding and fail
        /// closed at a later write (F2). `spark` is tracked, so `spark = other`
        /// invalidates (AR-29).
        Invalidate {
            /// The tracked name being rebound/invalidated.
            variable: String,
            /// Source-order key (statement start byte).
            order: usize,
            /// The rebinding form (AR-07).
            scope: EventScope,
        },
    }

    /// Walk `source` and emit the assignment/scope [`SparkLineageEvent`] stream.
    ///
    /// The public U2c entry point. Consumes U2a's per-call resolution
    /// ([`resolve_one_call`]) and correlates it with the surrounding
    /// assignment/scope structure. It does **not** link reads to writes or build
    /// [`crate::models::lineage::LineageEdgeCandidate`]s — that is U2b's
    /// single-cell dataflow join over this event stream.
    ///
    /// # Errors
    ///
    /// Returns [`EngramError`] if the Python grammar cannot be loaded or
    /// tree-sitter fails to produce a parse tree.
    pub(crate) fn extract_python_lineage(
        source: &str,
        authority_ctx: &LineageAuthorityContext,
    ) -> Result<Vec<SparkLineageEvent>, EngramError> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .map_err(|e| {
                EngramError::CodeGraph(CodeGraphError::ParseFailed {
                    reason: format!("Failed to set Python grammar: {e}"),
                })
            })?;
        let tree = parser.parse(source, None).ok_or_else(|| {
            EngramError::CodeGraph(CodeGraphError::ParseFailed {
                reason: "tree-sitter returned no parse tree for Python source".to_owned(),
            })
        })?;

        // T1 (fail-closed): reject a cell whose parse tree contains any ERROR or
        // missing node before collecting events. tree-sitter recovers malformed
        // constructs (e.g. a bad-args call) into partial nodes that would
        // otherwise join a read into a false edge; once the cell fails to parse
        // cleanly its dataflow cannot be trusted. Mirrors the SQL extractor's
        // `has_error()` guard (013-D).
        if tree.root_node().has_error() {
            return Ok(Vec::new());
        }

        let mut events = Vec::new();
        collect_events(
            tree.root_node(),
            source,
            authority_ctx,
            EventScope::TopLevel,
            &mut events,
        );
        events.sort_by_key(event_order);
        Ok(events)
    }

    /// The source-order key of any event kind.
    fn event_order(event: &SparkLineageEvent) -> usize {
        match event {
            SparkLineageEvent::ReadBind { order, .. }
            | SparkLineageEvent::WriteCall { order, .. }
            | SparkLineageEvent::Invalidate { order, .. } => *order,
        }
    }

    /// Recursively emit events for `node`, threading the enclosing scope.
    fn collect_events(
        node: Node<'_>,
        source: &str,
        ctx: &LineageAuthorityContext,
        enclosing: EventScope,
        out: &mut Vec<SparkLineageEvent>,
    ) {
        handle_node(node, source, ctx, enclosing, out);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let child_scope = if child.kind() == "block" {
                block_scope(node.kind(), enclosing)
            } else if is_comprehension(node.kind()) {
                EventScope::Comprehension
            } else if node.kind() == "lambda" {
                // C2: a lambda body is never top-level. Preserve an already
                // non-top-level enclosing scope; otherwise mark it Lambda so a
                // read/write inside the lambda is not treated as edge-eligible.
                if enclosing == EventScope::TopLevel {
                    EventScope::Lambda
                } else {
                    enclosing
                }
            } else if matches!(node.kind(), "boolean_operator" | "conditional_expression") {
                // V3: a write guarded by a short-circuit boolean (`… and …` /
                // `… or …`) or a ternary (`… if cond else …`) may never execute, so
                // it is not a direct cell-body statement. Mark it a Branch exactly
                // like if/match/case (fail-closed), preserving an already
                // non-top-level enclosing scope.
                if enclosing == EventScope::TopLevel {
                    EventScope::Branch
                } else {
                    enclosing
                }
            } else {
                enclosing
            };
            collect_events(child, source, ctx, child_scope, out);
        }
    }

    /// The scope a `block` child introduces, given its parent's kind.
    fn block_scope(parent_kind: &str, current: EventScope) -> EventScope {
        match parent_kind {
            "if_statement" | "elif_clause" | "else_clause" | "match_statement" | "case_clause" => {
                EventScope::Branch
            }
            "for_statement" | "while_statement" => EventScope::Loop,
            "with_statement"
            | "try_statement"
            | "except_clause"
            | "except_group_clause"
            | "finally_clause" => EventScope::WithExcept,
            "function_definition" | "class_definition" => EventScope::NestedDef,
            _ => current,
        }
    }

    /// Whether a node kind is a comprehension / generator expression.
    fn is_comprehension(kind: &str) -> bool {
        matches!(
            kind,
            "list_comprehension"
                | "set_comprehension"
                | "dictionary_comprehension"
                | "generator_expression"
        )
    }

    /// Emit the event(s) contributed by a single node (no recursion).
    fn handle_node(
        node: Node<'_>,
        source: &str,
        ctx: &LineageAuthorityContext,
        enclosing: EventScope,
        out: &mut Vec<SparkLineageEvent>,
    ) {
        match node.kind() {
            "assignment" => handle_assignment(node, source, ctx, enclosing, out),
            "augmented_assignment" => {
                emit_target_invalidations(
                    node.child_by_field_name("left"),
                    source,
                    node.start_byte(),
                    EventScope::AugmentedAssign,
                    out,
                );
            }
            "named_expression" => {
                if let Some(name) = node
                    .child_by_field_name("name")
                    .and_then(|n| simple_identifier(n, source))
                {
                    push_invalidate(out, name, node.start_byte(), EventScope::Walrus);
                }
            }
            "delete_statement" => {
                for name in delete_targets(node, source) {
                    push_invalidate(out, name, node.start_byte(), EventScope::Del);
                }
            }
            "import_statement" | "import_from_statement" => {
                for name in import_bound_names(node, source) {
                    push_invalidate(out, name, node.start_byte(), EventScope::Import);
                }
            }
            "for_statement" => {
                emit_target_invalidations(
                    node.child_by_field_name("left"),
                    source,
                    node.start_byte(),
                    EventScope::ForTarget,
                    out,
                );
            }
            "for_in_clause" => {
                emit_target_invalidations(
                    node.child_by_field_name("left"),
                    source,
                    node.start_byte(),
                    EventScope::Comprehension,
                    out,
                );
            }
            "with_statement" => {
                for name in with_clause_targets(node, source) {
                    push_invalidate(out, name, node.start_byte(), EventScope::WithTarget);
                }
            }
            "except_clause" => {
                for name in except_target(node, source) {
                    push_invalidate(out, name, node.start_byte(), EventScope::WithExcept);
                }
            }
            "function_definition" | "class_definition" => {
                if let Some(name) = node
                    .child_by_field_name("name")
                    .and_then(|n| simple_identifier(n, source))
                {
                    push_invalidate(out, name, node.start_byte(), EventScope::NestedDef);
                }
            }
            "call" => {
                if let Some(resolved) = resolve_one_call(node, source, ctx) {
                    if resolved.role == SparkCallRole::Write {
                        out.push(SparkLineageEvent::WriteCall {
                            receiver: resolved.receiver,
                            endpoint: resolved.endpoint,
                            order: node.start_byte(),
                            scope: enclosing,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    /// Emit a [`SparkLineageEvent::ReadBind`] for a top-level-resolvable Spark
    /// read assignment, otherwise emit an [`SparkLineageEvent::Invalidate`] for
    /// every simple-name target (non-Spark rebind, F2).
    fn handle_assignment(
        node: Node<'_>,
        source: &str,
        ctx: &LineageAuthorityContext,
        enclosing: EventScope,
        out: &mut Vec<SparkLineageEvent>,
    ) {
        let Some(left) = node.child_by_field_name("left") else {
            return;
        };
        let names = collect_target_names(left, source);
        if let (1, Some(right)) = (names.len(), node.child_by_field_name("right")) {
            if right.kind() == "call" {
                if let Some(resolved) = resolve_one_call(right, source, ctx) {
                    if resolved.role == SparkCallRole::Read {
                        out.push(SparkLineageEvent::ReadBind {
                            variable: names[0].clone(),
                            session: resolved.receiver.clone(),
                            endpoint: resolved.endpoint,
                            order: node.start_byte(),
                            scope: enclosing,
                        });
                        return;
                    }
                }
            }
        }
        for name in names {
            push_invalidate(out, name, node.start_byte(), enclosing);
        }
    }

    /// Push an invalidation event for every simple-name target of `target`.
    fn emit_target_invalidations(
        target: Option<Node<'_>>,
        source: &str,
        order: usize,
        scope: EventScope,
        out: &mut Vec<SparkLineageEvent>,
    ) {
        if let Some(target) = target {
            for name in collect_target_names(target, source) {
                push_invalidate(out, name, order, scope);
            }
        }
    }

    /// Append an [`SparkLineageEvent::Invalidate`].
    fn push_invalidate(
        out: &mut Vec<SparkLineageEvent>,
        variable: String,
        order: usize,
        scope: EventScope,
    ) {
        out.push(SparkLineageEvent::Invalidate {
            variable,
            order,
            scope,
        });
    }

    /// Return the identifier text of `node` when it is a bare `identifier`.
    fn simple_identifier(node: Node<'_>, source: &str) -> Option<String> {
        (node.kind() == "identifier").then(|| node_text(node, source).to_owned())
    }

    /// Collect the simple-name targets of an assignment/loop target node.
    ///
    /// Handles a bare `identifier` and (recursively) nested tuple/list/starred
    /// destructuring patterns; subscript and attribute targets bind no simple
    /// name and yield nothing.
    fn collect_target_names(target: Node<'_>, source: &str) -> Vec<String> {
        match target.kind() {
            "identifier" => vec![node_text(target, source).to_owned()],
            "pattern_list" | "tuple_pattern" | "list_pattern" | "tuple" | "list"
            | "list_splat_pattern" | "list_splat" => {
                // T4 (fail-closed): descend recursively so a nested destructuring
                // rebind — e.g. `(x, (df, y)) = …` or `[a, *rest] = …` — invalidates
                // every inner name. A missed rebind would leave a stale read binding
                // and mint false lineage on a later `df.write`. Over-collecting is
                // the safe direction (it only drops edges Python actually rebinds).
                let mut names = Vec::new();
                let mut cursor = target.walk();
                for child in target.children(&mut cursor) {
                    names.extend(collect_target_names(child, source));
                }
                names
            }
            _ => Vec::new(),
        }
    }

    /// Collect the simple-name targets of a `del` statement.
    fn delete_targets(node: Node<'_>, source: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "identifier" => names.push(node_text(child, source).to_owned()),
                "expression_list" | "pattern_list" | "tuple" => {
                    let mut inner = child.walk();
                    for grand in child.children(&mut inner) {
                        if grand.kind() == "identifier" {
                            names.push(node_text(grand, source).to_owned());
                        }
                    }
                }
                _ => {}
            }
        }
        names
    }

    /// Collect the names an `import` / `from … import …` binds into scope.
    fn import_bound_names(node: Node<'_>, source: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut cursor = node.walk();
        for child in node.children_by_field_name("name", &mut cursor) {
            match child.kind() {
                "aliased_import" => {
                    if let Some(name) = child
                        .child_by_field_name("alias")
                        .and_then(|a| simple_identifier(a, source))
                    {
                        names.push(name);
                    }
                }
                "dotted_name" => {
                    if let Some(first) = child.named_child(0) {
                        if first.kind() == "identifier" {
                            names.push(node_text(first, source).to_owned());
                        }
                    }
                }
                "identifier" => names.push(node_text(child, source).to_owned()),
                _ => {}
            }
        }
        names
    }

    /// Collect the `as` targets of a `with` statement's `with_clause`.
    fn with_clause_targets(with_stmt: Node<'_>, source: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut cursor = with_stmt.walk();
        for child in with_stmt.children(&mut cursor) {
            if child.kind() != "with_clause" {
                continue;
            }
            let mut items = child.walk();
            for item in child.children(&mut items) {
                if item.kind() == "with_item" {
                    if let Some(name) = item
                        .child_by_field_name("value")
                        .and_then(|v| as_pattern_alias(v, source))
                    {
                        names.push(name);
                    }
                }
            }
        }
        names
    }

    /// Return the `as` target of an `except` clause, if any.
    fn except_target(except_clause: Node<'_>, source: &str) -> Vec<String> {
        except_clause
            .child_by_field_name("value")
            .and_then(|v| as_pattern_alias(v, source))
            .map(|n| vec![n])
            .unwrap_or_default()
    }

    /// Extract the bound identifier from an `as_pattern` (`… as name`).
    fn as_pattern_alias(node: Node<'_>, source: &str) -> Option<String> {
        if node.kind() != "as_pattern" {
            return None;
        }
        let alias = node.child_by_field_name("alias")?;
        let mut cursor = alias.walk();
        for child in alias.children(&mut cursor) {
            if child.kind() == "identifier" {
                return Some(node_text(child, source).to_owned());
            }
        }
        simple_identifier(alias, source)
    }

    // ── U2b: single-cell DataFrame dataflow resolver (Fork E Option b) ─────────

    /// Join a single notebook cell's [`SparkLineageEvent`] stream into directional
    /// [`LineageEdgeCandidate`]s (Fork E Option b).
    ///
    /// Consumes U2c's event stream — read *Bind*, write call, and non-Spark
    /// rebind/invalidation events — performing **no** second AST walk (U2c is the
    /// single event-emission source of truth). It tracks `df_var → read dataset`
    /// bindings established by atomic top-level read *Binds* (a read Bind does not
    /// self-invalidate — AR-02) and, when a later top-level write call on the same
    /// variable appears, emits
    /// `LineageEdgeCandidate { target: write_dataset, sources: [read_dataset] }`
    /// (the directional carrier U4 flattens to `lineage_derives_from` edges).
    ///
    /// Fail closed (no edge) on: any rebind/invalidation of the tracked variable
    /// before the write (`df = other`, `spark = other` — AR-29 — an unresolvable
    /// RHS, or any augmented/walrus/del/import/for/with/comprehension rebind, F2);
    /// a second resolved read into the same variable before any intervening write
    /// (ambiguous reread, W1);
    /// a bind or write whose event scope is not the top-level cell/module body
    /// (branch, loop, comprehension, `with`/`except`, `def`/`class` — AR-07); an
    /// unresolved receiver (`receiver = None`); or a read/write endpoint U2 left
    /// unresolved. A later resolved read *after* one or more writes rebinds the
    /// variable to the new source while preserving the earlier write fan-out.
    /// Cross-cell `df` propagation is out of v1 — callers invoke this per cell, so
    /// a cell's events never mix with another's.
    pub(crate) fn resolve_cell_candidates(
        events: &[SparkLineageEvent],
    ) -> Vec<crate::models::lineage::LineageEdgeCandidate> {
        use std::collections::{HashMap, HashSet};

        use crate::models::lineage::LineageEdgeCandidate;

        let mut binding: HashMap<String, LineageEndpoint> = HashMap::new();
        // Session names invalidated by a prior rebind (`spark = other`, AR-29).
        // A read whose chain-root session is invalidated is untrusted.
        let mut invalidated_sessions: HashSet<String> = HashSet::new();
        // Variables poisoned by an ambiguous second read before any write (W1).
        let mut reread_invalidated: HashSet<String> = HashSet::new();
        // Variables whose current read binding has already fanned out to at least
        // one write, so a later resolved read can safely rebind to a new source.
        let mut written_since_bind: HashSet<String> = HashSet::new();
        let mut candidates = Vec::new();
        for event in events {
            match event {
                // A resolved top-level read establishes the binding (AR-02) —
                // unless its session receiver was invalidated (AR-29).
                SparkLineageEvent::ReadBind {
                    variable,
                    session,
                    endpoint: Some(endpoint),
                    scope: EventScope::TopLevel,
                    ..
                } if !session
                    .as_deref()
                    .is_some_and(|s| invalidated_sessions.contains(s)) =>
                {
                    // U2b/W1 (097.004-T): a second resolved top-level read into
                    // an active binding is ambiguous until at least one write
                    // consumes that binding. Fail closed by poisoning the name.
                    // After one or more writes, the binding has already fanned
                    // out, so a later read may safely rebind to a new source.
                    if reread_invalidated.contains(variable.as_str()) {
                        binding.remove(variable);
                        written_since_bind.remove(variable);
                    } else if binding.contains_key(variable) {
                        if written_since_bind.contains(variable.as_str()) {
                            binding.insert(variable.clone(), endpoint.clone());
                            written_since_bind.remove(variable);
                        } else {
                            binding.remove(variable);
                            written_since_bind.remove(variable);
                            reread_invalidated.insert(variable.clone());
                        }
                    } else {
                        binding.insert(variable.clone(), endpoint.clone());
                        written_since_bind.remove(variable);
                    }
                }
                // A nested-scope, unresolved, or untrusted-session read rebinds
                // the variable to something ineligible — invalidate the prior
                // binding (AR-07 / AR-29).
                SparkLineageEvent::ReadBind { variable, .. } => {
                    binding.remove(variable);
                    written_since_bind.remove(variable);
                }
                // A resolved top-level write on a bound variable emits an edge.
                SparkLineageEvent::WriteCall {
                    receiver: Some(receiver),
                    endpoint: Some(target),
                    scope: EventScope::TopLevel,
                    ..
                } => {
                    if let Some(source) = binding.get(receiver) {
                        candidates.push(LineageEdgeCandidate {
                            target: target.clone(),
                            sources: vec![source.clone()],
                        });
                        written_since_bind.insert(receiver.clone());
                    }
                }
                // Unresolved receiver / endpoint / non-top-level write: no edge.
                SparkLineageEvent::WriteCall { .. } => {}
                // Any invalidation drops the tracked binding and marks the name
                // as an untrusted session receiver for later reads (F2 / AR-29).
                SparkLineageEvent::Invalidate { variable, .. } => {
                    binding.remove(variable);
                    written_since_bind.remove(variable);
                    invalidated_sessions.insert(variable.clone());
                }
            }
        }
        candidates
    }

    #[cfg(test)]
    mod tests {
        use std::collections::BTreeMap;

        use super::*;
        use crate::models::lineage::DatasetKind;

        #[test]
        fn atomic_bind_then_write_emits_one_readbind_one_write_no_self_invalidation() {
            let ctx = trusted_ctx();
            let source = concat!(
                "df = spark.read.parquet(\"s3://bucket/in\")\n",
                "df.write.saveAsTable(\"cat.sch.out\")\n",
            );

            let events = extract_python_lineage(source, &ctx).expect("extract");

            let read_binds: Vec<_> = events
                .iter()
                .filter(|e| matches!(e, SparkLineageEvent::ReadBind { .. }))
                .collect();
            assert_eq!(read_binds.len(), 1, "exactly one read Bind");
            match read_binds[0] {
                SparkLineageEvent::ReadBind {
                    variable,
                    endpoint,
                    scope,
                    ..
                } => {
                    assert_eq!(variable, "df", "read bound to the assignment target");
                    assert_eq!(endpoint.as_ref().expect("resolved").name, "s3://bucket/in");
                    assert_eq!(*scope, EventScope::TopLevel);
                }
                _ => unreachable!(),
            }

            let writes: Vec<_> = events
                .iter()
                .filter(|e| matches!(e, SparkLineageEvent::WriteCall { .. }))
                .collect();
            assert_eq!(writes.len(), 1, "exactly one write event");

            // AR-02: the read bind must NOT also emit a self-invalidation of df.
            assert!(
                !events.iter().any(|e| matches!(
                    e,
                    SparkLineageEvent::Invalidate { variable, .. } if variable == "df"
                )),
                "atomic read bind does not self-invalidate (AR-02)"
            );
        }

        #[test]
        fn mode_chain_write_resolves_chain_root_receiver() {
            let ctx = trusted_ctx();
            let source = "df.write.mode(\"overwrite\").saveAsTable(\"cat.sch.out\")\n";

            let events = extract_python_lineage(source, &ctx).expect("extract");
            let write = events
                .iter()
                .find_map(|e| match e {
                    SparkLineageEvent::WriteCall {
                        receiver, endpoint, ..
                    } => Some((receiver, endpoint)),
                    _ => None,
                })
                .expect("write event");
            // AR-13: the chain-root base simple-name is `df`.
            assert_eq!(write.0.as_deref(), Some("df"));
            assert_eq!(write.1.as_ref().expect("resolved").name, "cat.sch.out");
        }

        #[test]
        fn per_form_rebinds_each_emit_invalidation_events() {
            let ctx = trusted_ctx();
            // Each snippet rebinds the tracked name `df` (or `spark`) via a
            // distinct binding form; every one must emit an invalidation.
            let cases: &[(&str, &str)] = &[
                ("plain non-Spark", "df = other\n"),
                ("branch", "if c:\n    df = other\n"),
                ("loop", "for x in items:\n    df = other\n"),
                ("comprehension", "z = [x for df in items]\n"),
                ("with target", "with open(f) as df:\n    pass\n"),
                (
                    "except target",
                    "try:\n    pass\nexcept E as df:\n    pass\n",
                ),
                ("augmented assign", "df += 1\n"),
                ("walrus", "y = (df := compute())\n"),
                ("del", "del df\n"),
                ("import alias", "import pandas as df\n"),
                ("from-import", "from mod import df\n"),
                ("def rebind", "def df():\n    pass\n"),
                ("class rebind", "class df:\n    pass\n"),
                ("session rebind (AR-29)", "spark = other\n"),
            ];
            for (label, src) in cases {
                let events = extract_python_lineage(src, &ctx).expect("extract");
                let target = if label.contains("AR-29") {
                    "spark"
                } else {
                    "df"
                };
                assert!(
                    events.iter().any(|e| matches!(
                        e,
                        SparkLineageEvent::Invalidate { variable, .. } if variable == target
                    )),
                    "form `{label}` must emit an invalidation for `{target}`"
                );
                // Fail-closed: a non-Spark rebind is never misread as a bind.
                assert!(
                    !events.iter().any(|e| matches!(
                        e,
                        SparkLineageEvent::ReadBind { variable, .. } if variable == target
                    )),
                    "form `{label}` must not emit a read bind for `{target}`"
                );
            }
        }

        #[test]
        fn spark_sql_and_temp_view_emit_no_write_events() {
            let ctx = trusted_ctx();
            let source = concat!(
                "spark.sql(\"CREATE TABLE cat.sch.x AS SELECT 1\")\n",
                "df.createOrReplaceTempView(\"v\")\n",
            );
            let events = extract_python_lineage(source, &ctx).expect("extract");
            assert!(
                !events
                    .iter()
                    .any(|e| matches!(e, SparkLineageEvent::WriteCall { .. })),
                "spark.sql (deferred) and createOrReplaceTempView (content-only) emit no write"
            );
        }

        // ── U2b: single-cell dataflow resolver (6 scenarios) ─────────────────

        fn candidates_for(source: &str) -> Vec<crate::models::lineage::LineageEdgeCandidate> {
            let events = extract_python_lineage(source, &trusted_ctx()).expect("extract");
            resolve_cell_candidates(&events)
        }

        #[test]
        fn u2b_happy_path_single_cell_read_write_emits_candidate() {
            let candidates = candidates_for(concat!(
                "df = spark.read.parquet(\"s3://bucket/in\")\n",
                "df.write.saveAsTable(\"cat.sch.out\")\n",
            ));
            assert_eq!(candidates.len(), 1, "one read→write candidate");
            let candidate = &candidates[0];
            assert_eq!(candidate.target.name, "cat.sch.out");
            assert_eq!(candidate.sources.len(), 1);
            assert_eq!(candidate.sources[0].name, "s3://bucket/in");
        }

        #[test]
        fn u2b_reassignment_invalidates_no_edge() {
            // df = other before the write drops the binding (F2).
            assert!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/in\")\n",
                    "df = other\n",
                    "df.write.saveAsTable(\"cat.sch.out\")\n",
                ))
                .is_empty(),
                "non-Spark rebind invalidates the binding"
            );
            // spark = other invalidates the session (AR-29): a later read via
            // the untrusted spark never establishes a resolved binding.
            assert!(
                candidates_for(concat!(
                    "spark = other\n",
                    "df = spark.read.parquet(\"s3://bucket/in\")\n",
                    "df.write.saveAsTable(\"cat.sch.out\")\n",
                ))
                .is_empty(),
                "session rebind invalidates later reads via spark (AR-29)"
            );
        }

        #[test]
        fn u2b_second_read_into_bound_variable_invalidates_no_edge() {
            // W1 (097.004-T): a second resolved top-level read into an
            // already-bound variable makes the read->write dataflow chain
            // ambiguous. Per the U2b fail-closed doctrine it must invalidate
            // (emit no edge), not rebind to the later read.
            assert!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/a\")\n",
                    "df = spark.read.parquet(\"s3://bucket/b\")\n",
                    "df.write.saveAsTable(\"cat.sch.out\")\n",
                ))
                .is_empty(),
                "a second read into an already-bound variable invalidates the chain"
            );
            // A third read must not revive the chain (fail-closed persists).
            assert!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/a\")\n",
                    "df = spark.read.parquet(\"s3://bucket/b\")\n",
                    "df = spark.read.parquet(\"s3://bucket/c\")\n",
                    "df.write.saveAsTable(\"cat.sch.out\")\n",
                ))
                .is_empty(),
                "a third read does not revive a poisoned binding"
            );
        }

        #[test]
        fn u2b_regression_115_002_read_write_read_write_reuse_emits_both_exact_edges() {
            let candidates = candidates_for(concat!(
                "df = spark.read.parquet(\"s3://bucket/a\")\n",
                "df.write.saveAsTable(\"cat.sch.out_a\")\n",
                "df = spark.read.parquet(\"s3://bucket/b\")\n",
                "df.write.saveAsTable(\"cat.sch.out_b\")\n",
            ));
            assert_eq!(candidates.len(), 2, "both read→write chains must emit");
            assert_eq!(candidates[0].sources.len(), 1);
            assert_eq!(candidates[0].sources[0].name, "s3://bucket/a");
            assert_eq!(candidates[0].target.name, "cat.sch.out_a");
            assert_eq!(candidates[1].sources.len(), 1);
            assert_eq!(candidates[1].sources[0].name, "s3://bucket/b");
            assert_eq!(candidates[1].target.name, "cat.sch.out_b");
        }

        #[test]
        fn u2b_regression_115_002_non_spark_invalidation_then_valid_read_emits_edge() {
            let candidates = candidates_for(concat!(
                "df = other\n",
                "df = spark.read.parquet(\"s3://bucket/in\")\n",
                "df.write.saveAsTable(\"cat.sch.out\")\n",
            ));
            assert_eq!(
                candidates.len(),
                1,
                "a non-Spark invalidation must not poison a later valid read"
            );
            assert_eq!(candidates[0].sources.len(), 1);
            assert_eq!(candidates[0].sources[0].name, "s3://bucket/in");
            assert_eq!(candidates[0].target.name, "cat.sch.out");
        }

        #[test]
        fn u2b_regression_115_002_one_read_multiple_writes_preserves_fan_out() {
            let candidates = candidates_for(concat!(
                "df = spark.read.parquet(\"s3://bucket/in\")\n",
                "df.write.saveAsTable(\"cat.sch.out_a\")\n",
                "df.write.saveAsTable(\"cat.sch.out_b\")\n",
            ));
            assert_eq!(
                candidates.len(),
                2,
                "one read binding must fan out to both writes"
            );
            assert_eq!(candidates[0].sources.len(), 1);
            assert_eq!(candidates[0].sources[0].name, "s3://bucket/in");
            assert_eq!(candidates[0].target.name, "cat.sch.out_a");
            assert_eq!(candidates[1].sources.len(), 1);
            assert_eq!(candidates[1].sources[0].name, "s3://bucket/in");
            assert_eq!(candidates[1].target.name, "cat.sch.out_b");
        }

        #[test]
        fn u2b_non_top_level_rebind_invalidates_no_edge() {
            let forms = [
                "if c:\n    df = other\n",
                "for x in items:\n    df = other\n",
                "z = [x for df in items]\n",
                "with open(f) as df:\n    pass\n",
                "try:\n    pass\nexcept E as df:\n    pass\n",
                "def df():\n    pass\n",
                "class df:\n    pass\n",
            ];
            for form in forms {
                let source = format!(
                    "df = spark.read.parquet(\"s3://bucket/in\")\n{form}df.write.saveAsTable(\"cat.sch.out\")\n"
                );
                assert!(
                    candidates_for(&source).is_empty(),
                    "nested rebind form `{form}` must invalidate (AR-07)"
                );
            }
        }

        #[test]
        fn u2b_unresolved_receiver_no_edge() {
            // Write on a df never bound to a resolved read.
            assert!(
                candidates_for("df.write.saveAsTable(\"cat.sch.out\")\n").is_empty(),
                "write on an unbound receiver yields no edge"
            );
            // Non-simple-name chain root (receiver = None).
            assert!(
                candidates_for("get_session().write.save(\"s3://bucket/out\")\n").is_empty(),
                "non-simple-name receiver yields no edge"
            );
        }

        #[test]
        fn u2b_cross_cell_read_write_rejected() {
            // Cell A only reads; Cell B only writes. Resolving each cell's event
            // stream independently (as U4 does) yields no cross-cell edge.
            let cell_a = extract_python_lineage(
                "df = spark.read.parquet(\"s3://bucket/in\")\n",
                &trusted_ctx(),
            )
            .expect("extract A");
            let cell_b =
                extract_python_lineage("df.write.saveAsTable(\"cat.sch.out\")\n", &trusted_ctx())
                    .expect("extract B");
            assert!(
                resolve_cell_candidates(&cell_a).is_empty(),
                "cell A: no write"
            );
            assert!(
                resolve_cell_candidates(&cell_b).is_empty(),
                "cell B: df not bound in this cell"
            );
        }

        #[test]
        fn u2b_contract_u2c_emits_all_three_event_kinds() {
            // The U2c↔U2b contract: U2c emits atomic read Bind, write call, AND
            // non-Spark rebind/invalidation events that U2b consumes.
            let events = extract_python_lineage(
                concat!(
                    "df = spark.read.parquet(\"s3://bucket/in\")\n", // ReadBind
                    "other = compute()\n",                           // Invalidate(other)
                    "df.write.saveAsTable(\"cat.sch.out\")\n",       // WriteCall
                ),
                &trusted_ctx(),
            )
            .expect("extract");
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, SparkLineageEvent::ReadBind { .. })),
                "emits a read Bind"
            );
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, SparkLineageEvent::WriteCall { .. })),
                "emits a write call"
            );
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, SparkLineageEvent::Invalidate { .. })),
                "emits an invalidation"
            );
            // And the stream still joins the happy-path read→write edge.
            assert_eq!(resolve_cell_candidates(&events).len(), 1);
        }

        #[test]
        fn u2b_non_spark_session_read_root_emits_no_edge_c1() {
            // C1 (zero-false-edge): a read-shaped call on a NON-Spark object must
            // not establish a lineage binding, so a downstream write on the same
            // variable cannot mint a false edge. Any object exposing a `.table(…)`
            // method would otherwise be treated as a trusted Spark session.
            assert!(
                candidates_for(concat!(
                    "df = client.table(\"cat.sch.orders\")\n",
                    "df.write.saveAsTable(\"cat.sch.summary\")\n",
                ))
                .is_empty(),
                "a non-Spark `.table(...)` root must not mint a lineage read (C1)"
            );
            // Control: the identical shape rooted at the trusted `spark` session
            // still emits its edge.
            assert_eq!(
                candidates_for(concat!(
                    "df = spark.table(\"cat.sch.orders\")\n",
                    "df.write.saveAsTable(\"cat.sch.summary\")\n",
                ))
                .len(),
                1,
                "a genuine spark-session read still emits its edge (C1 control)"
            );
        }

        #[test]
        fn u2b_lambda_body_write_is_not_top_level_c2() {
            // C2: a write inside a lambda body must not be attributed TopLevel, so
            // it cannot join a top-level read into a (false) edge — a lambda body
            // is not a top-level statement, exactly like a plain function body.
            assert!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/in\")\n",
                    "f = lambda: df.write.saveAsTable(\"cat.sch.out\")\n",
                ))
                .is_empty(),
                "a lambda-body write is not TopLevel and must emit no edge (C2)"
            );
        }

        #[test]
        fn u2b_match_case_body_write_is_not_top_level_n2() {
            // N2 (same class as the C2 lambda fix): a write inside a `match`/`case`
            // suite must not be attributed TopLevel — a case body is a conditional
            // branch, so a top-level read joined to a case-body write would emit a
            // false conditional edge, violating the direct-child/fail-closed rule.
            assert!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/in\")\n",
                    "match kind:\n",
                    "    case 1:\n",
                    "        df.write.saveAsTable(\"cat.sch.out\")\n",
                ))
                .is_empty(),
                "a match/case-body write is a branch scope and must emit no edge (N2)"
            );
        }

        #[test]
        fn u2b_malformed_python_cell_emits_no_edge_t1() {
            // T1 (fail-closed): a cell whose parse tree has any error must emit no
            // lineage. tree-sitter recovers a malformed write call (here trailing
            // `,,` in the args) that would otherwise join the earlier read into a
            // false edge — mirror the SQL extractor's `has_error()` guard (013-D).
            assert!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/in\")\n",
                    "df.write.saveAsTable(\"cat.sch.out\",,)\n",
                ))
                .is_empty(),
                "a malformed (recoverable) write call must emit no edge (T1)"
            );
            // A syntax error ANYWHERE in the cell also drops the well-formed parts:
            // we cannot trust the cell's dataflow once it fails to parse cleanly.
            assert!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/in\")\n",
                    "df.write.saveAsTable(\"cat.sch.out\")\n",
                    "def broken(\n",
                ))
                .is_empty(),
                "a parse error elsewhere in the cell drops all lineage (T1)"
            );
            // Control: the well-formed equivalent still emits its edge.
            assert_eq!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/in\")\n",
                    "df.write.saveAsTable(\"cat.sch.out\")\n",
                ))
                .len(),
                1,
                "the well-formed control still emits its edge (T1 control)"
            );
        }

        #[test]
        fn u2b_bytes_string_literal_emits_no_edge_t2() {
            // T2 (fail-closed): a bytes literal (`b"…"`) cannot name a `str` table
            // or path at runtime, so it must not resolve to a dataset identity.
            assert!(
                candidates_for(concat!(
                    "df = spark.table(b\"cat.sch.orders\")\n",
                    "df.write.saveAsTable(\"cat.sch.out\")\n",
                ))
                .is_empty(),
                "a bytes-literal table arg must emit no edge (T2)"
            );
            // Control: the plain str form still resolves.
            assert_eq!(
                candidates_for(concat!(
                    "df = spark.table(\"cat.sch.orders\")\n",
                    "df.write.saveAsTable(\"cat.sch.out\")\n",
                ))
                .len(),
                1,
                "a plain str table arg still resolves (T2 control)"
            );
        }

        #[test]
        fn u2b_escaped_string_literal_emits_no_edge_t3() {
            // T3 (fail-closed): the extractor reads the source spelling, not the
            // runtime string value, so an escape sequence would persist a WRONG
            // dataset identity (`…da\u0074a` is `…data` at runtime). Fail closed on
            // any backslash/escape rather than mint a mislabeled edge.
            assert!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/da\\u0074a\")\n",
                    "df.write.saveAsTable(\"cat.sch.out\")\n",
                ))
                .is_empty(),
                "an escaped read literal must emit no edge (T3)"
            );
            // Control: the decoded literal spelled plainly still resolves.
            assert_eq!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/data\")\n",
                    "df.write.saveAsTable(\"cat.sch.out\")\n",
                ))
                .len(),
                1,
                "a plain read literal still resolves (T3 control)"
            );
        }

        #[test]
        fn u2b_nested_destructuring_rebind_breaks_chain_t4() {
            // T4 (fail-closed): a nested destructuring rebind of `df` invalidates
            // the earlier read binding, so a later `df.write` must NOT join the
            // stale read. Missing the nested target would mint false lineage.
            assert!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/in\")\n",
                    "(x, (df, y)) = values\n",
                    "df.write.saveAsTable(\"cat.sch.out\")\n",
                ))
                .is_empty(),
                "a nested rebind of df must break the read→write chain (T4)"
            );
            // Control: an unbroken read→write chain still emits its edge.
            assert_eq!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/in\")\n",
                    "df.write.saveAsTable(\"cat.sch.out\")\n",
                ))
                .len(),
                1,
                "an unbroken chain still emits its edge (T4 control)"
            );
        }

        #[test]
        fn u2c_boolean_guarded_write_emits_no_edge_v3() {
            // V3 (fail-closed): a write guarded by a short-circuit boolean may never
            // execute, so it is not a direct cell-body statement and must not join
            // the top-level read into an edge (same class as C1/N2).
            assert!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/in\")\n",
                    "enabled and df.write.saveAsTable(\"cat.sch.t\")\n",
                ))
                .is_empty(),
                "an `and`-guarded write must emit no edge (V3)"
            );
            assert!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/in\")\n",
                    "enabled or df.write.saveAsTable(\"cat.sch.t\")\n",
                ))
                .is_empty(),
                "an `or`-guarded write must emit no edge (V3)"
            );
            // Control: a direct cell-body write still emits its edge.
            assert_eq!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/in\")\n",
                    "df.write.saveAsTable(\"cat.sch.t\")\n",
                ))
                .len(),
                1,
                "a direct cell-body write still emits its edge (V3 control)"
            );
        }

        #[test]
        fn u2c_ternary_write_emits_no_edge_v3() {
            // V3 (fail-closed): a write inside a ternary (`conditional_expression`)
            // may never execute, so it must not be attributed TopLevel.
            assert!(
                candidates_for(concat!(
                    "df = spark.read.parquet(\"s3://bucket/in\")\n",
                    "(df.write.saveAsTable(\"cat.sch.t\") if enabled else None)\n",
                ))
                .is_empty(),
                "a ternary-guarded write must emit no edge (V3)"
            );
        }

        /// A trusted authority context: catalog `cat` → `prod-metastore`, and a
        /// trusted `s3://bucket` storage authority.
        fn trusted_ctx() -> LineageAuthorityContext {
            let mut catalogs = BTreeMap::new();
            catalogs.insert("cat".to_owned(), "prod-metastore".to_owned());
            LineageAuthorityContext::new(catalogs, vec!["s3://bucket".to_owned()])
        }

        fn resolved_endpoints(calls: &[ResolvedSparkCall]) -> usize {
            calls.iter().filter(|c| c.endpoint.is_some()).count()
        }

        #[test]
        fn resolvable_forms_resolve_role_endpoint_and_chain_root_receiver() {
            let ctx = trusted_ctx();
            let source = concat!(
                "a = spark.table(\"cat.sch.t\")\n",
                "b = spark.read.parquet(\"s3://bucket/in\")\n",
                "b.write.saveAsTable(\"cat.sch.out\")\n",
                "b.write.save(\"s3://bucket/out\")\n",
                "b.write.mode(\"overwrite\").saveAsTable(\"cat.sch.out2\")\n",
            );

            let calls = resolve_spark_calls(source, &ctx).expect("resolve");
            assert_eq!(resolved_endpoints(&calls), 5, "all five forms resolve");

            let read_table = calls
                .iter()
                .find(|c| {
                    c.role == SparkCallRole::Read
                        && c.endpoint
                            .as_ref()
                            .is_some_and(|e| e.kind == DatasetKind::Table)
                })
                .expect("read table");
            assert_eq!(read_table.receiver.as_deref(), Some("spark"));
            assert_eq!(
                read_table.endpoint.as_ref().expect("endpoint").id,
                "table::prod-metastore::cat.sch.t"
            );

            let read_path = calls
                .iter()
                .find(|c| {
                    c.role == SparkCallRole::Read
                        && c.endpoint
                            .as_ref()
                            .is_some_and(|e| e.kind == DatasetKind::Path)
                })
                .expect("read path");
            assert_eq!(read_path.receiver.as_deref(), Some("spark"));
            assert_eq!(
                read_path.endpoint.as_ref().expect("endpoint").name,
                "s3://bucket/in"
            );

            let write_table = calls
                .iter()
                .find(|c| {
                    c.role == SparkCallRole::Write
                        && c.endpoint.as_ref().is_some_and(|e| e.name == "cat.sch.out")
                })
                .expect("write table");
            assert_eq!(write_table.receiver.as_deref(), Some("b"));
            assert_eq!(
                write_table.endpoint.as_ref().expect("endpoint").kind,
                DatasetKind::Table
            );

            let write_path = calls
                .iter()
                .find(|c| {
                    c.role == SparkCallRole::Write
                        && c.endpoint
                            .as_ref()
                            .is_some_and(|e| e.kind == DatasetKind::Path)
                })
                .expect("write path");
            assert_eq!(write_path.receiver.as_deref(), Some("b"));
            assert_eq!(
                write_path.endpoint.as_ref().expect("endpoint").name,
                "s3://bucket/out"
            );

            // AR-13: mode(...) chain still resolves the base simple-name `b`.
            let mode_chain = calls
                .iter()
                .find(|c| {
                    c.endpoint
                        .as_ref()
                        .is_some_and(|e| e.name == "cat.sch.out2")
                })
                .expect("mode-chain write");
            assert_eq!(mode_chain.role, SparkCallRole::Write);
            assert_eq!(mode_chain.receiver.as_deref(), Some("b"));

            // Source order is monotonic and every top-level call is TopLevel.
            let orders: Vec<usize> = calls.iter().map(|c| c.order).collect();
            let mut sorted = orders.clone();
            sorted.sort_unstable();
            assert_eq!(orders, sorted, "calls are returned in source order");
            assert!(
                calls.iter().all(|c| c.scope == LineageScope::TopLevel),
                "all statements are top-level"
            );
        }

        #[test]
        fn nested_call_is_flagged_nested_scope() {
            let ctx = trusted_ctx();
            let source = concat!("if cond:\n", "    df = spark.table(\"cat.sch.t\")\n",);

            let calls = resolve_spark_calls(source, &ctx).expect("resolve");
            let nested = calls
                .iter()
                .find(|c| c.endpoint.is_some())
                .expect("resolved read");
            assert_eq!(nested.scope, LineageScope::Nested);
        }

        #[test]
        fn three_part_literal_with_no_authority_resolves_zero_endpoints() {
            let ctx = LineageAuthorityContext::empty();
            let source = concat!(
                "df = spark.table(\"cat.sch.t\")\n",
                "df.write.saveAsTable(\"cat.sch.out\")\n",
            );

            let calls = resolve_spark_calls(source, &ctx).expect("resolve");
            assert_eq!(
                resolved_endpoints(&calls),
                0,
                "no trusted authority ⇒ fail-closed (AR-01)"
            );
        }

        #[test]
        fn non_resolvable_arg_forms_resolve_zero_endpoints() {
            let ctx = trusted_ctx();
            let source = concat!(
                "spark.table(f\"cat.sch.{x}\")\n",                     // f-string
                "spark.read.parquet(\"s3://\" + \"bucket\")\n",        // concatenation
                "spark.read.parquet(\"data/in\")\n",                   // relative path
                "spark.table(\"t\")\n",                                // 1-part name
                "spark.table(\"sch.t\")\n",                            // 2-part name
                "spark.table(dbutils.widgets.get(\"k\"))\n",           // config/widget
                "spark.sql(\"CREATE TABLE cat.sch.x AS SELECT 1\")\n", // deferred
                "df.createOrReplaceTempView(\"v\")\n",                 // content-only
            );

            let calls = resolve_spark_calls(source, &ctx).expect("resolve");
            assert_eq!(
                resolved_endpoints(&calls),
                0,
                "every non-resolvable arg form fails closed (013-D)"
            );
        }
    }
}
