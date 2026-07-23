//! Tree-sitter Python grammar parser.
//!
//! Extracts top-level functions, classes, and import edges from Python source
//! files. Method bodies are not yet indexed (Tier 1 implementation).

use tree_sitter::{Node, Parser};

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

    extract_top_level(root, source, &mut symbols, &mut edges);

    Ok(ParseResult { symbols, edges })
}

fn extract_top_level(
    root: Node<'_>,
    source: &str,
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
                    extract_calls_from_body(child, source, &func.name, edges);
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

/// A resolved Python call site. `is_qualified` is never set for Python (no `::`
/// path form), so no Rust-style `scoped_*` helpers are needed.
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
            if let Some(call) = resolve_call_name(current, source) {
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

/// Classify a Python `call` node's `function` child.
///
/// * `identifier` (`foo()`) → bare call, promoted (`is_method:false`).
/// * `attribute` (`obj.foo()`, `self.bar()`) → marked `is_method:true` with an
///   EMPTY `raw_qualifier`, so `should_stage_provenance_call(true, false, "")`
///   returns `false` and the consumer drops it (never promoted, never staged —
///   fails closed, closing the `self`-receiver leak). The callee is the
///   `attribute` field text (NOT Rust's `field`); the receiver `object` is
///   intentionally not copied.
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
            ResolvedCallName {
                callee,
                is_method: true,
                is_qualified: false,
                // Empty on purpose: fails closed at should_stage_provenance_call.
                raw_qualifier: String::new(),
                qualifier_kind: "method".to_owned(),
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

    /// Extract the content of a plain (non-f, non-concatenated) string literal.
    ///
    /// Fails closed for f-strings (an `f`/`F` prefix or any `interpolation`
    /// child) and for any node that is not a bare `string`.
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
                    if prefix.contains('f') || prefix.contains('F') {
                        return None;
                    }
                }
                "interpolation" => return None,
                "string_content" => content = Some(node_text(child, source).to_owned()),
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
            } else {
                enclosing
            };
            collect_events(child, source, ctx, child_scope, out);
        }
    }

    /// The scope a `block` child introduces, given its parent's kind.
    fn block_scope(parent_kind: &str, current: EventScope) -> EventScope {
        match parent_kind {
            "if_statement" | "elif_clause" | "else_clause" => EventScope::Branch,
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
    /// Handles a bare `identifier` and flat tuple/list patterns; subscript and
    /// attribute targets bind no simple name and yield nothing.
    fn collect_target_names(target: Node<'_>, source: &str) -> Vec<String> {
        match target.kind() {
            "identifier" => vec![node_text(target, source).to_owned()],
            "pattern_list" | "tuple_pattern" | "list_pattern" | "tuple" | "list" => {
                let mut names = Vec::new();
                let mut cursor = target.walk();
                for child in target.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        names.push(node_text(child, source).to_owned());
                    }
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
