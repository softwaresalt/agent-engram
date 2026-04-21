//! Tree-sitter Swift grammar parser.
//!
//! Extracts top-level and member functions, class/struct/actor declarations
//! (via the unified `class_declaration` node with `declaration_kind` field),
//! protocol declarations, and `import_declaration` edges from Swift source files.
//!
//! # Node kinds used (tree-sitter-swift 0.7.1)
//!
//! - `function_declaration` → [`super::ExtractedSymbol::Function`] (name via `name` field)
//! - `class_declaration` with `declaration_kind` in {class, struct, actor}
//!   → [`super::ExtractedSymbol::Class`]
//! - `protocol_declaration` → [`super::ExtractedSymbol::Interface`]
//! - `import_declaration` → [`super::ExtractedEdge::Imports`]
//!
//! Extensions and enums are skipped at this scope level.

use tree_sitter::{Node, Parser};

use super::{
    ExtractedClass, ExtractedEdge, ExtractedFunction, ExtractedInterface, ExtractedSymbol,
    ParseResult,
};

/// Parse a Swift source file and extract symbols and edges.
///
/// # Errors
///
/// Returns [`crate::errors::EngramError`] if the grammar cannot be loaded or
/// tree-sitter fails to produce a valid parse tree.
pub(super) fn parse_swift_source(source: &str) -> Result<ParseResult, crate::errors::EngramError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_swift::LANGUAGE.into())
        .map_err(|e| {
            crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
                reason: format!("Failed to set Swift grammar: {e}"),
            })
        })?;

    let tree = parser.parse(source, None).ok_or_else(|| {
        crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
            reason: "tree-sitter returned no parse tree for Swift source".to_owned(),
        })
    })?;

    let root = tree.root_node();
    let mut symbols = Vec::new();
    let mut edges = Vec::new();

    extract_swift_top_level(root, source, &mut symbols, &mut edges);

    Ok(ParseResult { symbols, edges })
}

fn extract_swift_top_level(
    root: Node<'_>,
    source: &str,
    symbols: &mut Vec<ExtractedSymbol>,
    edges: &mut Vec<ExtractedEdge>,
) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "import_declaration" => {
                edges.push(ExtractedEdge::Imports {
                    import_path: super::node_text(child, source),
                });
            }
            "function_declaration" => {
                if let Some(func) = extract_swift_function(child, source, None) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: func.name.clone(),
                    });
                    symbols.push(ExtractedSymbol::Function(func));
                }
            }
            "class_declaration" => {
                // class_declaration covers class, struct, actor, enum, extension.
                // Emit Class for class/struct/actor; skip enum/extension at this scope.
                let decl_kind = child
                    .child_by_field_name("declaration_kind")
                    .map(|n| n.kind())
                    .unwrap_or("");
                if matches!(decl_kind, "class" | "struct" | "actor") {
                    if let Some(cls) = extract_swift_class(child, source) {
                        edges.push(ExtractedEdge::Defines {
                            symbol_name: cls.name.clone(),
                        });
                        extract_swift_members(child, source, &cls.name.clone(), symbols, edges);
                        symbols.push(ExtractedSymbol::Class(cls));
                    }
                }
            }
            "protocol_declaration" => {
                if let Some(iface) = extract_swift_interface(child, source) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: iface.name.clone(),
                    });
                    symbols.push(ExtractedSymbol::Interface(iface));
                }
            }
            _ => {}
        }
    }
}

fn extract_swift_members(
    class_node: Node<'_>,
    source: &str,
    class_name: &str,
    symbols: &mut Vec<ExtractedSymbol>,
    edges: &mut Vec<ExtractedEdge>,
) {
    let Some(body) = class_node.child_by_field_name("body") else {
        return;
    };
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "function_declaration" {
            if let Some(func) = extract_swift_function(child, source, Some(class_name)) {
                edges.push(ExtractedEdge::Defines {
                    symbol_name: func.name.clone(),
                });
                symbols.push(ExtractedSymbol::Function(func));
            }
        }
    }
}

fn extract_swift_function(
    node: Node<'_>,
    source: &str,
    class_name: Option<&str>,
) -> Option<ExtractedFunction> {
    let name_node = node.child_by_field_name("name")?;
    let bare_name = super::node_text(name_node, source);
    let name = match class_name {
        Some(cls) => format!("{cls}::{bare_name}"),
        None => bare_name,
    };

    let body = super::node_text(node, source);
    let body_hash = super::sha256_hex(&body);
    #[allow(clippy::cast_possible_truncation)]
    let line_start = (node.start_position().row + 1) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let line_end = (node.end_position().row + 1) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let token_count = (body.len() / 4) as u32;

    Some(ExtractedFunction {
        name,
        line_start,
        line_end,
        signature: extract_swift_signature(node, source),
        docstring: None,
        body,
        body_hash,
        token_count,
    })
}

fn extract_swift_signature(node: Node<'_>, source: &str) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "function_body" {
            let sig_end = child.start_byte();
            let sig_start = node.start_byte();
            return source[sig_start..sig_end].trim().to_owned();
        }
    }
    super::node_text(node, source)
}

fn extract_swift_class(node: Node<'_>, source: &str) -> Option<ExtractedClass> {
    let name = node
        .child_by_field_name("name")
        .map(|n| super::node_text(n, source))?;

    let body = super::node_text(node, source);
    let body_hash = super::sha256_hex(&body);
    #[allow(clippy::cast_possible_truncation)]
    let line_start = (node.start_position().row + 1) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let line_end = (node.end_position().row + 1) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let token_count = (body.len() / 4) as u32;

    Some(ExtractedClass {
        name,
        line_start,
        line_end,
        docstring: None,
        body,
        body_hash,
        token_count,
    })
}

fn extract_swift_interface(node: Node<'_>, source: &str) -> Option<ExtractedInterface> {
    let name = node
        .child_by_field_name("name")
        .map(|n| super::node_text(n, source))?;

    let body = super::node_text(node, source);
    let body_hash = super::sha256_hex(&body);
    #[allow(clippy::cast_possible_truncation)]
    let line_start = (node.start_position().row + 1) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let line_end = (node.end_position().row + 1) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let token_count = (body.len() / 4) as u32;

    Some(ExtractedInterface {
        name,
        line_start,
        line_end,
        docstring: None,
        body,
        body_hash,
        token_count,
    })
}
