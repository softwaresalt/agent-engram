//! Tree-sitter Rust grammar parser.
//!
//! Extracts functions, structs, traits, impl methods, use declarations, and
//! call-graph edges from Rust source files.

use tree_sitter::{Node, Parser};

use super::canonical::Qualifier;
use super::{
    ExtractedClass, ExtractedEdge, ExtractedFunction, ExtractedInterface, ExtractedSymbol,
    ParseResult,
};

/// Parse a Rust source file and extract symbols and edges.
///
/// # Errors
///
/// Returns an error string if the grammar cannot be set or tree-sitter
/// fails to produce a valid parse tree.
pub(super) fn parse_rust_source(source: &str) -> Result<ParseResult, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|e| format!("Failed to set Rust grammar: {e}"))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "tree-sitter returned no parse tree".to_owned())?;

    let root = tree.root_node();

    let mut symbols = Vec::new();
    let mut edges = Vec::new();

    extract_top_level(root, source, &mut symbols, &mut edges);

    Ok(ParseResult { symbols, edges })
}

/// Walk top-level children of the root node, extracting symbols and edges.
fn extract_top_level(
    root: Node<'_>,
    source: &str,
    symbols: &mut Vec<ExtractedSymbol>,
    edges: &mut Vec<ExtractedEdge>,
) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_item" => {
                if let Some(func) = extract_function(child, source) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: func.name.clone(),
                    });
                    extract_calls_from_body(child, source, &func.name, edges);
                    symbols.push(ExtractedSymbol::Function(func));
                }
            }
            "struct_item" => {
                if let Some(class) = extract_class(child, source) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: class.name.clone(),
                    });
                    symbols.push(ExtractedSymbol::Class(class));
                }
            }
            "trait_item" => {
                if let Some(iface) = extract_interface(child, source) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: iface.name.clone(),
                    });
                    symbols.push(ExtractedSymbol::Interface(iface));
                }
            }
            "impl_item" => {
                extract_impl(child, source, symbols, edges);
            }
            "use_declaration" => {
                if let Some(import_path) = extract_use_path(child, source) {
                    edges.push(ExtractedEdge::Imports { import_path });
                }
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

fn extract_interface(node: Node<'_>, source: &str) -> Option<ExtractedInterface> {
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

    Some(ExtractedInterface {
        name,
        line_start,
        line_end,
        docstring,
        body,
        body_hash,
        token_count,
    })
}

fn extract_impl(
    node: Node<'_>,
    source: &str,
    symbols: &mut Vec<ExtractedSymbol>,
    edges: &mut Vec<ExtractedEdge>,
) {
    let trait_name = node
        .child_by_field_name("trait")
        .map(|n| super::node_text(n, source));
    let type_name = node
        .child_by_field_name("type")
        .map(|n| super::node_text(n, source));

    if let (Some(t), Some(s)) = (&trait_name, &type_name) {
        edges.push(ExtractedEdge::InheritsFrom {
            struct_name: s.clone(),
            trait_name: t.clone(),
        });
    }

    if let Some(body_node) = node.child_by_field_name("body") {
        let mut cursor = body_node.walk();
        for child in body_node.children(&mut cursor) {
            if child.kind() == "function_item" {
                if let Some(mut func) = extract_function(child, source) {
                    if let Some(ref ty) = type_name {
                        func.name = format!("{ty}::{}", func.name);
                    }
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: func.name.clone(),
                    });
                    extract_calls_from_body(child, source, &func.name, edges);
                    symbols.push(ExtractedSymbol::Function(func));
                }
            }
        }
    }
}

fn extract_use_path(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "scoped_identifier" | "identifier" | "scoped_use_list" | "use_wildcard"
            | "use_as_clause" | "use_list" => {
                return Some(super::node_text(child, source));
            }
            _ => {}
        }
    }
    None
}

fn extract_calls_from_body(
    node: Node<'_>,
    source: &str,
    caller_name: &str,
    edges: &mut Vec<ExtractedEdge>,
) {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if current.kind() == "call_expression" {
            if let Some((callee, is_method, is_qualified)) = resolve_call_name(current, source) {
                edges.push(ExtractedEdge::Calls {
                    caller: caller_name.to_owned(),
                    callee,
                    is_method,
                    is_qualified,
                });
            }
        }
        let mut cursor = current.walk();
        for child in current.children(&mut cursor) {
            stack.push(child);
        }
    }
}

const CALL_BLOCKLIST: &[&str] = &[
    "new", "default", "into", "clone", "from", "unwrap", "expect", "ok", "err",
];

fn resolve_call_name(node: Node<'_>, source: &str) -> Option<(String, bool, bool)> {
    let function_node = node.child_by_field_name("function")?;
    // (name, is_method, is_qualified) — the flags are mutually exclusive.
    let (name, is_method, is_qualified) = match function_node.kind() {
        "identifier" => (Some(super::node_text(function_node, source)), false, false),
        // Path-qualified calls: `a::b()`. Reduced to the final segment `b`.
        // Marked `is_qualified` so the consumer does not promote them: a module
        // path (`crate::util::helper`) resolves by bare final segment, but a
        // type-associated call (`Type::parse`) is indexed under its qualified
        // name, and the two are indistinguishable here. Promoting the bare name
        // would risk a false singleton edge to an unrelated free function.
        "scoped_identifier" => {
            let mut cursor = function_node.walk();
            let n = function_node
                .children(&mut cursor)
                .filter(|c| c.kind() == "identifier")
                .last()
                .map(|n| super::node_text(n, source));
            (n, false, true)
        }
        // Method / receiver calls: `x.foo()`, `self.bar()`. The `function`
        // child is a `field_expression` whose `field` names the called method.
        // Extracted (and marked `is_method`) for completeness and future
        // method-aware resolution, but the consumer must not promote them to a
        // `calls_edge`: impl methods are indexed as `Type::method`, so name-only
        // resolution cannot match them and would risk a false singleton edge.
        // The blocklist below still drops idiomatic no-ops (`x.clone()`).
        "field_expression" => (
            function_node
                .child_by_field_name("field")
                .map(|n| super::node_text(n, source)),
            true,
            false,
        ),
        _ => (None, false, false),
    };
    name.filter(|n| !CALL_BLOCKLIST.contains(&n.as_str()))
        .map(|n| (n, is_method, is_qualified))
}

/// Collect the left-to-right segments of a (possibly nested) `scoped_identifier`
/// (`a::b::c` → `["a", "b", "c"]`).
fn scoped_path_segments(node: Node<'_>, source: &str) -> Vec<String> {
    let mut segments = Vec::new();
    collect_scoped_segments(node, source, &mut segments);
    segments
}

fn collect_scoped_segments(node: Node<'_>, source: &str, out: &mut Vec<String>) {
    if node.kind() == "scoped_identifier" {
        if let Some(path) = node.child_by_field_name("path") {
            collect_scoped_segments(path, source, out);
        }
        if let Some(name) = node.child_by_field_name("name") {
            out.push(super::node_text(name, source));
        }
    } else {
        out.push(super::node_text(node, source));
    }
}

/// Classify a call's `function` node into a canonical [`Qualifier`] plus the
/// trailing path segments (the parts after the qualifier root, ending with the
/// callee), or `None` for non-path calls (bare identifier / method-receiver).
///
/// The `Self` type yields the unforgeable [`Qualifier::SelfType`] marker, set
/// **only** from the reserved `Self` keyword at the path root — `Self` cannot be
/// a user identifier, so no other source text can produce the marker
/// (091.009-T / Option C A7). Consumed by Unit B staging; precision-neutral in
/// Unit A (qualified calls remain dropped).
#[must_use]
pub fn classify_call_qualifier(
    function_node: Node<'_>,
    source: &str,
) -> Option<(Qualifier, Vec<String>)> {
    if function_node.kind() != "scoped_identifier" {
        return None;
    }
    let mut segments = scoped_path_segments(function_node, source);
    if segments.len() < 2 {
        return None;
    }
    let root = segments.remove(0);
    let qualifier = if root == "Self" {
        Qualifier::SelfType
    } else {
        Qualifier::Path(root)
    };
    Some((qualifier, segments))
}

fn extract_signature(node: Node<'_>, source: &str) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" || child.kind() == "declaration_list" {
            let sig_end = child.start_byte();
            let sig_start = node.start_byte();
            return source[sig_start..sig_end].trim().to_owned();
        }
    }
    super::node_text(node, source)
}

fn extract_docstring(node: Node<'_>, source: &str) -> Option<String> {
    let mut doc_lines = Vec::new();
    let mut sibling = node.prev_sibling();
    while let Some(s) = sibling {
        let text = super::node_text(s, source);
        if s.kind() == "line_comment" && text.starts_with("///") {
            doc_lines.push(text.trim_start_matches("///").trim().to_owned());
            sibling = s.prev_sibling();
        } else if s.kind() == "block_comment" && text.starts_with("/**") {
            let cleaned = text
                .trim_start_matches("/**")
                .trim_end_matches("*/")
                .trim()
                .to_owned();
            doc_lines.push(cleaned);
            sibling = None;
        } else if s.kind() == "attribute_item" || s.kind() == "attribute" {
            sibling = s.prev_sibling();
        } else {
            break;
        }
    }
    if doc_lines.is_empty() {
        None
    } else {
        doc_lines.reverse();
        Some(doc_lines.join("\n"))
    }
}
