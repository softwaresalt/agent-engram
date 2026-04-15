//! Tree-sitter C# grammar parser.
//!
//! Extracts top-level methods, classes, interfaces, and using-directive edges
//! from C# source files.

use tree_sitter::{Node, Parser};

use super::{
    ExtractedClass, ExtractedEdge, ExtractedFunction, ExtractedInterface, ExtractedSymbol,
    ParseResult,
};

/// Parse a C# source file and extract symbols and edges.
///
/// # Errors
///
/// Returns [`crate::errors::EngramError`] if the grammar cannot be loaded or
/// tree-sitter fails to produce a valid parse tree.
pub(super) fn parse_csharp_source(source: &str) -> Result<ParseResult, crate::errors::EngramError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
        .map_err(|e| {
            crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
                reason: format!("Failed to set C# grammar: {e}"),
            })
        })?;

    let tree = parser.parse(source, None).ok_or_else(|| {
        crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
            reason: "tree-sitter returned no parse tree for C# source".to_owned(),
        })
    })?;

    let root = tree.root_node();
    let mut symbols = Vec::new();
    let mut edges = Vec::new();

    extract_compilation_unit(root, source, &mut symbols, &mut edges);

    Ok(ParseResult { symbols, edges })
}

/// Walk a compilation_unit (or namespace_declaration body), extracting top-level items.
fn extract_compilation_unit(
    root: Node<'_>,
    source: &str,
    symbols: &mut Vec<ExtractedSymbol>,
    edges: &mut Vec<ExtractedEdge>,
) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "class_declaration" => {
                if let Some(class) = extract_class(child, source) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: class.name.clone(),
                    });
                    // Extract methods from class body.
                    extract_class_members(child, source, &class.name, symbols, edges);
                    symbols.push(ExtractedSymbol::Class(class));
                }
            }
            "interface_declaration" => {
                if let Some(iface) = extract_interface(child, source) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: iface.name.clone(),
                    });
                    symbols.push(ExtractedSymbol::Interface(iface));
                }
            }
            "namespace_declaration" | "file_scoped_namespace_declaration" => {
                // Recurse into namespace body.
                if let Some(body) = child.child_by_field_name("body") {
                    extract_compilation_unit(body, source, symbols, edges);
                }
            }
            "using_directive" => {
                let text = super::node_text(child, source);
                edges.push(ExtractedEdge::Imports { import_path: text });
            }
            _ => {}
        }
    }
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

/// Extract method declarations from a class body.
fn extract_class_members(
    class_node: Node<'_>,
    source: &str,
    class_name: &str,
    symbols: &mut Vec<ExtractedSymbol>,
    edges: &mut Vec<ExtractedEdge>,
) {
    let Some(body_node) = class_node.child_by_field_name("body") else {
        return;
    };

    let mut cursor = body_node.walk();
    for child in body_node.children(&mut cursor) {
        if child.kind() == "method_declaration" {
            if let Some(mut func) = extract_method(child, source) {
                func.name = format!("{class_name}::{}", func.name);
                edges.push(ExtractedEdge::Defines {
                    symbol_name: func.name.clone(),
                });
                symbols.push(ExtractedSymbol::Function(func));
            }
        }
    }
}

fn extract_method(node: Node<'_>, source: &str) -> Option<ExtractedFunction> {
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
    #[allow(clippy::cast_possible_truncation)]
    let token_count = (body.len() / 4) as u32;

    Some(ExtractedFunction {
        name,
        line_start,
        line_end,
        signature,
        docstring: None,
        body,
        body_hash,
        token_count,
    })
}

fn extract_signature(node: Node<'_>, source: &str) -> String {
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
