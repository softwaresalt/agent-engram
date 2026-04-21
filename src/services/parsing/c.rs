//! Tree-sitter C grammar parser.
//!
//! Extracts top-level functions, named struct definitions, and `#include`
//! edges from C source files.
//!
//! # Node kinds used (tree-sitter-c 0.23.4)
//!
//! - `function_definition` — name via `declarator` chain recursion
//!   → [`super::ExtractedSymbol::Function`]
//! - `struct_specifier` at top level (bare `struct Point { ... };`) OR
//!   `declaration` whose `type` field is a `struct_specifier` with a body
//!   → [`super::ExtractedSymbol::Class`]
//! - `preproc_include` → [`super::ExtractedEdge::Imports`]
//!
//! # Out of scope
//!
//! Function-pointer calls and indirect dispatch are excluded by design.

use tree_sitter::{Node, Parser};

use super::{ExtractedClass, ExtractedEdge, ExtractedFunction, ExtractedSymbol, ParseResult};

/// Parse a C source file and extract symbols and edges.
///
/// # Errors
///
/// Returns [`crate::errors::EngramError`] if the grammar cannot be loaded or
/// tree-sitter fails to produce a valid parse tree.
pub(super) fn parse_c_source(source: &str) -> Result<ParseResult, crate::errors::EngramError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c::LANGUAGE.into())
        .map_err(|e| {
            crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
                reason: format!("Failed to set C grammar: {e}"),
            })
        })?;

    let tree = parser.parse(source, None).ok_or_else(|| {
        crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
            reason: "tree-sitter returned no parse tree for C source".to_owned(),
        })
    })?;

    let root = tree.root_node();
    let mut symbols = Vec::new();
    let mut edges = Vec::new();

    extract_c_top_level(root, source, &mut symbols, &mut edges);

    Ok(ParseResult { symbols, edges })
}

fn extract_c_top_level(
    root: Node<'_>,
    source: &str,
    symbols: &mut Vec<ExtractedSymbol>,
    edges: &mut Vec<ExtractedEdge>,
) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "preproc_include" => {
                edges.push(ExtractedEdge::Imports {
                    import_path: super::node_text(child, source),
                });
            }
            "function_definition" => {
                if let Some(func) = extract_c_function(child, source) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: func.name.clone(),
                    });
                    symbols.push(ExtractedSymbol::Function(func));
                }
            }
            "declaration" => {
                // `typedef struct { ... } Point;` or `struct Point { ... } var;`
                // — struct_specifier appears as the `type` field.
                if let Some(type_node) = child.child_by_field_name("type") {
                    if type_node.kind() == "struct_specifier"
                        && type_node.child_by_field_name("body").is_some()
                    {
                        if let Some(cls) = extract_c_struct(type_node, source) {
                            edges.push(ExtractedEdge::Defines {
                                symbol_name: cls.name.clone(),
                            });
                            symbols.push(ExtractedSymbol::Class(cls));
                        }
                    }
                }
            }
            "struct_specifier" => {
                // `struct Point { ... };` with no declarator is a bare
                // type_specifier child of translation_unit in tree-sitter-c.
                if child.child_by_field_name("body").is_some() {
                    if let Some(cls) = extract_c_struct(child, source) {
                        edges.push(ExtractedEdge::Defines {
                            symbol_name: cls.name.clone(),
                        });
                        symbols.push(ExtractedSymbol::Class(cls));
                    }
                }
            }
            _ => {}
        }
    }
}

fn extract_c_function(node: Node<'_>, source: &str) -> Option<ExtractedFunction> {
    let declarator = node.child_by_field_name("declarator")?;
    let name = fn_name_from_declarator(declarator, source)?;

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
        signature: extract_c_signature(node, source),
        docstring: None,
        body,
        body_hash,
        token_count,
    })
}

fn extract_c_signature(node: Node<'_>, source: &str) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "compound_statement" {
            let sig_end = child.start_byte();
            let sig_start = node.start_byte();
            return source[sig_start..sig_end].trim().to_owned();
        }
    }
    super::node_text(node, source)
}

fn extract_c_struct(node: Node<'_>, source: &str) -> Option<ExtractedClass> {
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

/// Recursively descend the declarator chain to find the function identifier.
///
/// C declarators form a chain: `function_declarator → identifier` or
/// `pointer_declarator → function_declarator → identifier`.
fn fn_name_from_declarator(node: Node<'_>, source: &str) -> Option<String> {
    match node.kind() {
        "identifier" => Some(super::node_text(node, source)),
        "function_declarator"
        | "pointer_declarator"
        | "parenthesized_declarator"
        | "abstract_pointer_declarator" => {
            let inner = node.child_by_field_name("declarator")?;
            fn_name_from_declarator(inner, source)
        }
        _ => None,
    }
}
