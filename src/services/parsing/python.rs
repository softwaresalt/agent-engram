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

/// DFS over a top-level function's subtree emitting `Calls` edges, stopping at
/// nested `function_definition`, `lambda`, and `class_definition` boundaries so
/// calls are attributed only to their owning top-level function.
///
/// The walk is seeded with the owning function's direct children so the owning
/// `function_definition` node itself is not treated as a nested-scope boundary.
fn extract_calls_from_body(
    node: Node<'_>,
    source: &str,
    caller_name: &str,
    edges: &mut Vec<ExtractedEdge>,
) {
    let mut stack: Vec<Node<'_>> = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
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
