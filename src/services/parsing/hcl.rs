//! Tree-sitter HCL grammar service.
//!
//! Top-level blocks and attributes are represented as namespaced structural
//! class symbols. Plain dotted traversals are represented as conservative
//! reference target hints without interpreting Terraform semantics.

use std::collections::HashSet;

use tree_sitter::{Node, Parser};

use super::{ExtractedClass, ExtractedEdge, ExtractedSymbol, ParseResult};
use crate::errors::{CodeGraphError, EngramError};

// Direct parser calls bypass the workspace file-size guard, so bound AST depth too.
const MAX_HCL_AST_DEPTH: usize = 128;

/// Parse HCL source through the shared grammar.
///
/// # Errors
///
/// Returns an [`EngramError`] if the grammar cannot be loaded or tree-sitter
/// fails to produce a parse tree.
pub(super) fn parse_hcl_source(source: &str) -> Result<ParseResult, EngramError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_hcl::LANGUAGE.into())
        .map_err(|error| {
            EngramError::CodeGraph(CodeGraphError::ParseFailed {
                reason: format!("failed to set HCL grammar: {error}"),
            })
        })?;

    let tree = parser.parse(source, None).ok_or_else(|| {
        EngramError::CodeGraph(CodeGraphError::ParseFailed {
            reason: "tree-sitter returned no parse tree for HCL source".to_owned(),
        })
    })?;

    Ok(extract_hcl_items(tree.root_node(), source))
}

fn extract_hcl_items(root: Node<'_>, source: &str) -> ParseResult {
    if root.has_error() {
        return ParseResult {
            symbols: Vec::new(),
            edges: Vec::new(),
        };
    }

    let mut symbols = Vec::new();
    let mut edges = Vec::new();
    let mut cursor = root.walk();
    let top_level = root
        .named_children(&mut cursor)
        .find(|node| node.kind() == "body")
        .unwrap_or(root);
    let mut cursor = top_level.walk();

    for node in top_level.named_children(&mut cursor) {
        let name = match node.kind() {
            "block" => extract_block_name(node, source),
            "attribute" => extract_attribute_name(node, source),
            _ => None,
        };

        if let Some(name) = name {
            edges.push(ExtractedEdge::Defines {
                symbol_name: name.clone(),
            });
            symbols.push(ExtractedSymbol::Class(extract_class(name, node, source)));
        }
    }

    let mut seen_targets = HashSet::new();
    if !extract_references(root, source, &mut seen_targets, &mut edges) {
        return ParseResult {
            symbols: Vec::new(),
            edges: Vec::new(),
        };
    }

    ParseResult { symbols, edges }
}

fn extract_references(
    node: Node<'_>,
    source: &str,
    seen_targets: &mut HashSet<String>,
    edges: &mut Vec<ExtractedEdge>,
) -> bool {
    let mut nodes = vec![(node, 0_usize)];

    while let Some((node, depth)) = nodes.pop() {
        if depth > MAX_HCL_AST_DEPTH {
            return false;
        }

        if node.kind() == "expression" {
            if let Some(target) = extract_plain_traversal(node, source)
                && seen_targets.insert(target.clone())
            {
                edges.push(ExtractedEdge::References {
                    source: super::node_text(node, source),
                    target,
                });
            }
            continue;
        }

        let child_depth = depth.saturating_add(1);
        let child_start = nodes.len();
        let mut cursor = node.walk();
        nodes.extend(
            node.named_children(&mut cursor)
                .map(|child| (child, child_depth)),
        );
        // LIFO traversal needs reversed children to retain first-encounter source order.
        nodes[child_start..].reverse();
    }

    true
}

fn extract_plain_traversal(node: Node<'_>, source: &str) -> Option<String> {
    if node.has_error() {
        return None;
    }

    let mut cursor = node.walk();
    let mut children = node.named_children(&mut cursor);
    let root = children.next()?;
    if root.kind() != "variable_expr" {
        return None;
    }

    let mut segments = vec![extract_plain_identifier(root, source)?];
    for child in children {
        if child.kind() != "get_attr" {
            return None;
        }
        segments.push(extract_plain_identifier(child, source)?);
    }

    (segments.len() > 1).then(|| segments.join("."))
}

fn extract_plain_identifier(node: Node<'_>, source: &str) -> Option<String> {
    if node.has_error() || node.named_child_count() != 1 {
        return None;
    }

    let identifier = node.named_child(0)?;
    (identifier.kind() == "identifier").then(|| super::node_text(identifier, source))
}

fn extract_block_name(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut children = node.named_children(&mut cursor);
    let block_type = children.next()?;
    if block_type.kind() != "identifier" {
        return None;
    }

    let mut segments = vec![super::node_text(block_type, source)];
    let mut header_complete = false;
    for child in children {
        match child.kind() {
            "block_start" => {
                header_complete = true;
                break;
            }
            "comment" => {}
            "identifier" => segments.push(super::node_text(child, source)),
            "string_lit" => segments.push(extract_plain_label(child, source)?),
            _ => return None,
        }
    }

    header_complete.then(|| format!("hcl.block.{}", segments.join(".")))
}

fn extract_attribute_name(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let key = node.named_children(&mut cursor).next()?;
    (key.kind() == "identifier").then(|| format!("hcl.attribute.{}", super::node_text(key, source)))
}

fn extract_plain_label(node: Node<'_>, source: &str) -> Option<String> {
    let label = super::node_text(node, source);
    let label = label.strip_prefix('"')?.strip_suffix('"')?;
    (!label.is_empty() && !label.contains("${") && !label.contains("%{")).then(|| label.to_owned())
}

fn extract_class(name: String, node: Node<'_>, source: &str) -> ExtractedClass {
    let body = super::node_text(node, source);
    let body_hash = super::sha256_hex(&body);
    let line_start = u32::try_from(node.start_position().row.saturating_add(1)).unwrap_or(u32::MAX);
    let line_end = u32::try_from(node.end_position().row.saturating_add(1)).unwrap_or(u32::MAX);
    let token_count = u32::try_from(body.len() / 4).unwrap_or(u32::MAX);

    ExtractedClass {
        name,
        line_start,
        line_end,
        docstring: None,
        body,
        body_hash,
        token_count,
    }
}
