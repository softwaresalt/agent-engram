//! Tree-sitter C++ grammar parser.
//!
//! Extracts free functions and out-of-line member functions, class and struct
//! declarations, and `#include` edges from C++ source files.  Inline methods
//! defined inside a class body are not extracted at this level.
//!
//! # Node kinds used (tree-sitter-cpp 0.23.4)
//!
//! - `function_definition` (free + out-of-line member) — name via `declarator` chain
//!   → [`super::ExtractedSymbol::Function`]
//! - `class_specifier` at top level OR `declaration` whose `type` field is a
//!   `class_specifier` or `struct_specifier` with a body
//!   → [`super::ExtractedSymbol::Class`]
//! - `struct_specifier` at top level OR `declaration` whose `type` field is a
//!   `struct_specifier` with a body
//!   → [`super::ExtractedSymbol::Class`]
//! - `namespace_definition` — recursed into for nested declarations
//! - `preproc_include` → [`super::ExtractedEdge::Imports`]
//!
//! # Out of scope
//!
//! Template instantiation and overload-set ranking are excluded by design.

use tree_sitter::{Node, Parser};

use super::{ExtractedClass, ExtractedEdge, ExtractedFunction, ExtractedSymbol, ParseResult};

/// Parse a C++ source file and extract symbols and edges.
///
/// # Errors
///
/// Returns [`crate::errors::EngramError`] if the grammar cannot be loaded or
/// tree-sitter fails to produce a valid parse tree.
pub(super) fn parse_cpp_source(source: &str) -> Result<ParseResult, crate::errors::EngramError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_cpp::LANGUAGE.into())
        .map_err(|e| {
            crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
                reason: format!("Failed to set C++ grammar: {e}"),
            })
        })?;

    let tree = parser.parse(source, None).ok_or_else(|| {
        crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
            reason: "tree-sitter returned no parse tree for C++ source".to_owned(),
        })
    })?;

    let root = tree.root_node();
    let mut symbols = Vec::new();
    let mut edges = Vec::new();

    extract_cpp_declarations(root, source, &mut symbols, &mut edges);

    Ok(ParseResult { symbols, edges })
}

fn extract_cpp_declarations(
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
                if let Some(func) = extract_cpp_function(child, source) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: func.name.clone(),
                    });
                    symbols.push(ExtractedSymbol::Function(func));
                }
            }
            "declaration" => {
                // A class/struct with a declarator or typedef appears as a
                // declaration whose type is a class_specifier or struct_specifier.
                if let Some(type_node) = child.child_by_field_name("type") {
                    match type_node.kind() {
                        "class_specifier" | "struct_specifier"
                            if type_node.child_by_field_name("body").is_some() =>
                        {
                            if let Some(cls) = extract_cpp_class(type_node, source) {
                                edges.push(ExtractedEdge::Defines {
                                    symbol_name: cls.name.clone(),
                                });
                                symbols.push(ExtractedSymbol::Class(cls));
                            }
                        }
                        _ => {}
                    }
                }
            }
            // `class Greeter { ... };` or `struct Point { ... };` with no
            // declarator is a bare type_specifier child of translation_unit.
            "class_specifier" | "struct_specifier"
                if child.child_by_field_name("body").is_some() =>
            {
                if let Some(cls) = extract_cpp_class(child, source) {
                    edges.push(ExtractedEdge::Defines {
                        symbol_name: cls.name.clone(),
                    });
                    symbols.push(ExtractedSymbol::Class(cls));
                }
            }
            "namespace_definition" => {
                // Recurse into namespace body.
                if let Some(body) = child.child_by_field_name("body") {
                    extract_cpp_declarations(body, source, symbols, edges);
                }
            }
            _ => {}
        }
    }
}

fn extract_cpp_function(node: Node<'_>, source: &str) -> Option<ExtractedFunction> {
    let declarator = node.child_by_field_name("declarator")?;
    let name = cpp_name_from_declarator(declarator, source)?;

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
        signature: extract_cpp_signature(node, source),
        docstring: None,
        body,
        body_hash,
        token_count,
    })
}

fn extract_cpp_signature(node: Node<'_>, source: &str) -> String {
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

fn extract_cpp_class(node: Node<'_>, source: &str) -> Option<ExtractedClass> {
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

/// Recursively descend the declarator chain to extract the function name.
///
/// Handles: `identifier`, `qualified_identifier` (e.g. `Greeter::greet`),
/// `function_declarator`, `pointer_declarator`, `reference_declarator`.
fn cpp_name_from_declarator(node: Node<'_>, source: &str) -> Option<String> {
    match node.kind() {
        "identifier"
        | "qualified_identifier"
        | "type_identifier"
        | "destructor_name"
        | "operator_name" => Some(super::node_text(node, source)),
        "function_declarator"
        | "pointer_declarator"
        | "reference_declarator"
        | "parenthesized_declarator"
        | "abstract_pointer_declarator"
        | "abstract_reference_declarator" => {
            let inner = node.child_by_field_name("declarator")?;
            cpp_name_from_declarator(inner, source)
        }
        _ => None,
    }
}
