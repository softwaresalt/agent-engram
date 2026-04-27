//! Tree-sitter SQL grammar parser.
//!
//! Extracts schema-definition symbols and reference edges from SQL source files.
//!
//! # Node kinds used (tree-sitter-sequel 0.3)
//!
//! Top-level structure: `program` > `statement` > actual statement node.
//!
//! - `create_table` / `create_view` → [`super::ExtractedSymbol::Class`]
//! - `create_function` → [`super::ExtractedSymbol::Function`]
//! - `CREATE PROCEDURE` is currently unsupported by tree-sitter-sequel 0.3 and
//!   parses as `ERROR` rather than `create_procedure`; the matcher for
//!   `create_procedure` is retained for forward compatibility with future
//!   grammar support
//! - `from` (SELECT from-clause, sibling inside `statement`)
//!   and `insert` > `object_reference` → [`super::ExtractedEdge::References`]
//!
//! Names are extracted from the first `object_reference` > `identifier` child.

use tree_sitter::{Node, Parser};

use super::{ExtractedClass, ExtractedEdge, ExtractedFunction, ExtractedSymbol, ParseResult};

/// Parse a SQL source file and extract symbols and edges.
///
/// # Errors
///
/// Returns [`crate::errors::EngramError`] if the grammar cannot be loaded or
/// tree-sitter fails to produce a valid parse tree.
pub(super) fn parse_sql_source(source: &str) -> Result<ParseResult, crate::errors::EngramError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_sequel::LANGUAGE.into())
        .map_err(|e| {
            crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
                reason: format!("Failed to set SQL grammar: {e}"),
            })
        })?;

    let tree = parser.parse(source, None).ok_or_else(|| {
        crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
            reason: "tree-sitter returned no parse tree for SQL source".to_owned(),
        })
    })?;

    let root = tree.root_node();
    let mut symbols = Vec::new();
    let mut edges = Vec::new();

    extract_sql_top_level(root, source, &mut symbols, &mut edges);

    Ok(ParseResult { symbols, edges })
}

fn extract_sql_top_level(
    root: Node<'_>,
    source: &str,
    symbols: &mut Vec<ExtractedSymbol>,
    edges: &mut Vec<ExtractedEdge>,
) {
    // The grammar wraps every SQL statement in a `statement` container.
    let mut root_cursor = root.walk();
    for statement in root.children(&mut root_cursor) {
        if statement.kind() != "statement" {
            continue;
        }
        let mut stmt_cursor = statement.walk();
        for child in statement.children(&mut stmt_cursor) {
            match child.kind() {
                "create_table" | "create_view" => {
                    if let Some(cls) = extract_sql_class(child, source) {
                        edges.push(ExtractedEdge::Defines {
                            symbol_name: cls.name.clone(),
                        });
                        symbols.push(ExtractedSymbol::Class(cls));
                    }
                }
                "create_function" | "create_procedure" => {
                    if let Some(func) = extract_sql_function(child, source) {
                        edges.push(ExtractedEdge::Defines {
                            symbol_name: func.name.clone(),
                        });
                        symbols.push(ExtractedSymbol::Function(func));
                    }
                }
                // `from` is a sibling of `select` inside the same `statement`.
                "from" => {
                    extract_from_references(child, source, edges);
                }
                // INSERT has the target `object_reference` as a direct child.
                "insert" => {
                    extract_insert_references(child, source, edges);
                }
                _ => {}
            }
        }
    }
}

/// Extract the name from a CREATE TABLE/VIEW/FUNCTION/PROCEDURE node.
///
/// Names live in `object_reference` > `identifier` (first occurrence).
fn extract_sql_name(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "object_reference" {
            let mut inner = child.walk();
            for id_node in child.children(&mut inner) {
                if id_node.kind() == "identifier" {
                    return Some(super::node_text(id_node, source));
                }
            }
        }
    }
    None
}

fn extract_sql_class(node: Node<'_>, source: &str) -> Option<ExtractedClass> {
    let name = extract_sql_name(node, source)?;
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

fn extract_sql_function(node: Node<'_>, source: &str) -> Option<ExtractedFunction> {
    let name = extract_sql_name(node, source)?;
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
        signature: String::new(),
        docstring: None,
        body,
        body_hash,
        token_count,
    })
}

/// Extract table references from a `from` clause node.
///
/// Structure: `from` > `relation` > `object_reference` > `identifier`.
fn extract_from_references(node: Node<'_>, source: &str, edges: &mut Vec<ExtractedEdge>) {
    let mut cursor = node.walk();
    for relation in node.children(&mut cursor) {
        if relation.kind() != "relation" {
            continue;
        }
        let mut rel_cursor = relation.walk();
        for obj_ref in relation.children(&mut rel_cursor) {
            if obj_ref.kind() == "object_reference" {
                let mut id_cursor = obj_ref.walk();
                for id_node in obj_ref.children(&mut id_cursor) {
                    if id_node.kind() == "identifier" {
                        let target = super::node_text(id_node, source);
                        if !target.is_empty() {
                            edges.push(ExtractedEdge::References {
                                source: "select".to_owned(),
                                target,
                            });
                        }
                        break;
                    }
                }
            }
        }
    }
}

/// Extract the target table reference from an `insert` node.
///
/// Structure: `insert` > `object_reference` > `identifier`.
fn extract_insert_references(node: Node<'_>, source: &str, edges: &mut Vec<ExtractedEdge>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "object_reference" {
            let mut id_cursor = child.walk();
            for id_node in child.children(&mut id_cursor) {
                if id_node.kind() == "identifier" {
                    let target = super::node_text(id_node, source);
                    if !target.is_empty() {
                        edges.push(ExtractedEdge::References {
                            source: "insert".to_owned(),
                            target,
                        });
                    }
                    return;
                }
            }
        }
    }
}
