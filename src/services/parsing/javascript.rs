//! Tree-sitter JavaScript grammar parser.
//!
//! Extracts top-level functions, classes, and import edges from JavaScript
//! source files. Arrow function assignments at module scope are also captured.

use tree_sitter::{Node, Parser};

use super::{
    ExtractedClass, ExtractedEdge, ExtractedFunction, ExtractedInterface, ExtractedSymbol,
    ParseResult,
};

/// Parse a JavaScript source file and extract symbols and edges.
///
/// # Errors
///
/// Returns [`crate::errors::EngramError`] if the grammar cannot be loaded or
/// tree-sitter fails to produce a valid parse tree.
pub(super) fn parse_javascript_source(
    source: &str,
) -> Result<ParseResult, crate::errors::EngramError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_javascript::LANGUAGE.into())
        .map_err(|e| {
            crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
                reason: format!("Failed to set JavaScript grammar: {e}"),
            })
        })?;

    let tree = parser.parse(source, None).ok_or_else(|| {
        crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
            reason: "tree-sitter returned no parse tree for JavaScript source".to_owned(),
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
            "function_declaration" | "generator_function_declaration" => {
                if let Some(func) = extract_function(child, source) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: func.name.clone(),
                    });
                    symbols.push(ExtractedSymbol::Function(func));
                }
            }
            "class_declaration" => {
                if let Some(class) = extract_class(child, source) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: class.name.clone(),
                    });
                    symbols.push(ExtractedSymbol::Class(class));
                }
            }
            "import_statement" => {
                edges.push(ExtractedEdge::Imports {
                    import_path: extract_import(child, source),
                });
            }
            "lexical_declaration" | "variable_declaration" => {
                extract_arrow_fns(child, source, symbols, edges);
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

// Suppress dead_code: ExtractedInterface imported for type coherence with other parsers.
#[allow(dead_code)]
fn _use_interface(_: ExtractedInterface) {}

fn extract_import(node: Node<'_>, source: &str) -> String {
    super::node_text(node, source)
}

fn extract_arrow_fns(
    node: Node<'_>,
    source: &str,
    symbols: &mut Vec<ExtractedSymbol>,
    edges: &mut Vec<ExtractedEdge>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            let name = child
                .child_by_field_name("name")
                .map(|n| super::node_text(n, source));
            let value = child.child_by_field_name("value");
            if let (Some(name), Some(val)) = (name, value) {
                if val.kind() == "arrow_function" {
                    let body = super::node_text(child, source);
                    let body_hash = super::sha256_hex(&body);
                    #[allow(clippy::cast_possible_truncation)]
                    let line_start = (child.start_position().row + 1) as u32;
                    #[allow(clippy::cast_possible_truncation)]
                    let line_end = (child.end_position().row + 1) as u32;
                    #[allow(clippy::cast_possible_truncation)]
                    let token_count = (body.len() / 4) as u32;
                    let func = ExtractedFunction {
                        name: name.clone(),
                        line_start,
                        line_end,
                        signature: name.clone(),
                        docstring: None,
                        body,
                        body_hash,
                        token_count,
                    };
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: func.name.clone(),
                    });
                    symbols.push(ExtractedSymbol::Function(func));
                }
            }
        }
    }
}

fn extract_signature(node: Node<'_>, source: &str) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "statement_block" {
            let sig_end = child.start_byte();
            let sig_start = node.start_byte();
            return source[sig_start..sig_end].trim().to_owned();
        }
    }
    super::node_text(node, source)
}
