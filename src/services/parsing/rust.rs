//! Tree-sitter Rust grammar parser.
//!
//! Extracts functions, structs, traits, impl methods, use declarations, and
//! call-graph edges from Rust source files.

use tree_sitter::{Node, Parser};

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
                    extract_calls_from_body(child, source, &func.name, None, edges);
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
                    extract_calls_from_body(child, source, &func.name, type_name.as_deref(), edges);
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
    enclosing_type: Option<&str>,
    edges: &mut Vec<ExtractedEdge>,
) {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if current.kind() == "call_expression" {
            if let Some((callee, is_method, is_qualified, qualifier)) =
                resolve_call_name(current, source)
            {
                // Rewrite a `Self::` qualifier to the concrete enclosing impl
                // type so `Self::build()` inside `impl Widget` routes as
                // `Widget::build` (088.004-T). Outside an impl there is no
                // concrete `Self`, so the qualifier is left as-is and will match
                // no index name (no edge) rather than mis-resolving.
                let qualifier = match (qualifier, enclosing_type) {
                    (Some(q), Some(ty)) if q == "Self" => Some(ty.to_owned()),
                    (other, _) => other,
                };
                edges.push(ExtractedEdge::Calls {
                    caller: caller_name.to_owned(),
                    callee,
                    is_method,
                    is_qualified,
                    qualifier,
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

fn resolve_call_name(node: Node<'_>, source: &str) -> Option<(String, bool, bool, Option<String>)> {
    let function_node = node.child_by_field_name("function")?;
    // (name, is_method, is_qualified, qualifier). `is_method` and `is_qualified`
    // are mutually exclusive; `qualifier` is `Some` only for a path-qualified
    // call and carries the immediate qualifier segment for the downstream
    // qualification-aware resolver (088.003-T / 088.004-T).
    let (name, is_method, is_qualified, qualifier) = match function_node.kind() {
        "identifier" => (
            Some(super::node_text(function_node, source)),
            false,
            false,
            None,
        ),
        // Path-qualified calls: `a::b()`. `callee` stays the final segment `b`,
        // but the immediate qualifier `a` is captured so the resolver can route
        // a module path (`crate::util::helper`, matched by the bare free-function
        // index name) apart from a type-associated call (`Type::parse`, matched
        // by the `Type::parse` impl-method index name). Without the qualifier the
        // two are indistinguishable and promoting the bare name would risk a
        // false singleton edge (findings 1 & 7).
        "scoped_identifier" => {
            let mut cursor = function_node.walk();
            let n = function_node
                .children(&mut cursor)
                .filter(|c| c.kind() == "identifier")
                .last()
                .map(|n| super::node_text(n, source));
            let qualifier = scoped_call_qualifier(function_node, source);
            (n, false, true, qualifier)
        }
        // Method / receiver calls: `x.foo()`, `self.bar()`. The `function`
        // child is a `field_expression` whose `field` names the called method.
        // Extracted (and marked `is_method`) for completeness, but the consumer
        // must not promote them to a `calls_edge`: the receiver's type is unknown
        // without inference (013-D Option B, deferred), so name-only resolution
        // cannot match `Type::method` and would risk a false singleton edge. The
        // blocklist below still drops idiomatic no-ops (`x.clone()`).
        "field_expression" => (
            function_node
                .child_by_field_name("field")
                .map(|n| super::node_text(n, source)),
            true,
            false,
            None,
        ),
        _ => (None, false, false, None),
    };
    name.filter(|n| !CALL_BLOCKLIST.contains(&n.as_str()))
        .map(|n| (n, is_method, is_qualified, qualifier))
}

/// Extract the immediate qualifier segment of a `scoped_identifier` call target
/// — the segment directly preceding the final callee. Yields `Type` for
/// `Type::parse()`, `util` for `crate::util::helper()`, and the root token
/// (`crate`, `super`, `self`, `Self`) when the call is rooted there.
///
/// The Rust naming convention (UpperCamelCase type vs snake_case module) lets
/// the downstream resolver route a type-associated target apart from a
/// module-path target without a type-checker.
fn scoped_call_qualifier(scoped: Node<'_>, source: &str) -> Option<String> {
    let path = scoped.child_by_field_name("path")?;
    let segment = if path.kind() == "scoped_identifier" {
        path.child_by_field_name("name").unwrap_or(path)
    } else {
        path
    };
    Some(super::node_text(segment, source))
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
