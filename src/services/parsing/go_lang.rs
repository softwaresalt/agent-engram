//! Tree-sitter Go grammar parser.
//!
//! Extracts top-level functions, type declarations (structs/interfaces), and
//! import edges from Go source files.

use tree_sitter::{Node, Parser};

use super::{
    ExtractedClass, ExtractedEdge, ExtractedFunction, ExtractedInterface, ExtractedSymbol,
    ParseResult,
};

/// Parse a Go source file and extract symbols and edges.
///
/// # Errors
///
/// Returns [`crate::errors::EngramError`] if the grammar cannot be loaded or
/// tree-sitter fails to produce a valid parse tree.
pub(super) fn parse_go_source(source: &str) -> Result<ParseResult, crate::errors::EngramError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .map_err(|e| {
            crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
                reason: format!("Failed to set Go grammar: {e}"),
            })
        })?;

    let tree = parser.parse(source, None).ok_or_else(|| {
        crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
            reason: "tree-sitter returned no parse tree for Go source".to_owned(),
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
            "function_declaration" | "method_declaration" => {
                if let Some(func) = extract_function(child, source) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: func.name.clone(),
                    });
                    symbols.push(ExtractedSymbol::Function(func));
                }
            }
            "type_declaration" => {
                extract_type_decl(child, source, symbols, edges);
            }
            "import_declaration" => {
                extract_imports(child, source, edges);
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

/// Extract struct and interface type declarations.
fn extract_type_decl(
    node: Node<'_>,
    source: &str,
    symbols: &mut Vec<ExtractedSymbol>,
    edges: &mut Vec<ExtractedEdge>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_spec" {
            let name = child
                .child_by_field_name("name")
                .map(|n| super::node_text(n, source));
            let type_val = child.child_by_field_name("type");

            if let (Some(name), Some(tv)) = (name, type_val) {
                let body = super::node_text(child, source);
                let body_hash = super::sha256_hex(&body);
                #[allow(clippy::cast_possible_truncation)]
                let line_start = (child.start_position().row + 1) as u32;
                #[allow(clippy::cast_possible_truncation)]
                let line_end = (child.end_position().row + 1) as u32;
                #[allow(clippy::cast_possible_truncation)]
                let token_count = (body.len() / 4) as u32;

                match tv.kind() {
                    "struct_type" => {
                        edges.push(ExtractedEdge::Defines {
                            symbol_name: name.clone(),
                        });
                        symbols.push(ExtractedSymbol::Class(ExtractedClass {
                            name,
                            line_start,
                            line_end,
                            docstring: None,
                            body,
                            body_hash,
                            token_count,
                        }));
                    }
                    "interface_type" => {
                        edges.push(ExtractedEdge::Defines {
                            symbol_name: name.clone(),
                        });
                        symbols.push(ExtractedSymbol::Interface(ExtractedInterface {
                            name,
                            line_start,
                            line_end,
                            docstring: None,
                            body,
                            body_hash,
                            token_count,
                        }));
                    }
                    _ => {}
                }
            }
        }
    }
}

fn extract_imports(node: Node<'_>, source: &str, edges: &mut Vec<ExtractedEdge>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "import_spec_list" || child.kind() == "import_spec" {
            let text = super::node_text(child, source);
            edges.push(ExtractedEdge::Imports { import_path: text });
        }
    }
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
