//! Tree-sitter based AST parsing service.
//!
//! Provides node extraction for `function_item`, `struct_item`, `trait_item`,
//! `impl_item` and edge discovery for `call_expression`, `use_declaration`.

use sha2::{Digest, Sha256};
use tree_sitter::{Node, Parser};

/// A raw symbol extracted from a single source file by tree-sitter.
///
/// This is an intermediate representation before the symbol is inserted
/// into the database as a `Function`, `Class`, or `Interface` entity.
#[derive(Debug, Clone, PartialEq)]
pub enum ExtractedSymbol {
    /// A function or method definition (`function_item`).
    Function(ExtractedFunction),
    /// A struct definition (`struct_item`).
    Class(ExtractedClass),
    /// A trait definition (`trait_item`).
    Interface(ExtractedInterface),
}

/// A function or method extracted from a `function_item` node.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedFunction {
    /// Function name.
    pub name: String,
    /// 1-based start line.
    pub line_start: u32,
    /// 1-based end line (inclusive).
    pub line_end: u32,
    /// Full function signature (everything before the body block).
    pub signature: String,
    /// Doc comment text if present.
    pub docstring: Option<String>,
    /// Full source body of the function.
    pub body: String,
    /// SHA-256 hex digest of the source body.
    pub body_hash: String,
    /// Estimated token count (body length / 4).
    pub token_count: u32,
}

/// A struct definition extracted from a `struct_item` node.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedClass {
    /// Struct name.
    pub name: String,
    /// 1-based start line.
    pub line_start: u32,
    /// 1-based end line (inclusive).
    pub line_end: u32,
    /// Doc comment text if present.
    pub docstring: Option<String>,
    /// Full source body.
    pub body: String,
    /// SHA-256 hex digest of the source body.
    pub body_hash: String,
    /// Estimated token count (body length / 4).
    pub token_count: u32,
}

/// A trait definition extracted from a `trait_item` node.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedInterface {
    /// Trait name.
    pub name: String,
    /// 1-based start line.
    pub line_start: u32,
    /// 1-based end line (inclusive).
    pub line_end: u32,
    /// Doc comment text if present.
    pub docstring: Option<String>,
    /// Full source body.
    pub body: String,
    /// SHA-256 hex digest of the source body.
    pub body_hash: String,
    /// Estimated token count (body length / 4).
    pub token_count: u32,
}

/// A raw edge discovered during AST walking.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExtractedEdge {
    /// A call from one function name to another.
    Calls {
        /// Name of the calling function.
        caller: String,
        /// Name of the called function.
        callee: String,
    },
    /// A `use` declaration importing a path.
    Imports {
        /// Full path string (e.g., `crate::billing::process_payment`).
        import_path: String,
    },
    /// A struct implements a trait (from `impl Trait for Struct`).
    InheritsFrom {
        /// The implementing struct name.
        struct_name: String,
        /// The implemented trait name.
        trait_name: String,
    },
    /// A file defines a top-level symbol.
    Defines {
        /// Name of the defined symbol.
        symbol_name: String,
    },
}

/// Result of parsing a single source file.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// All symbols extracted from the file.
    pub symbols: Vec<ExtractedSymbol>,
    /// All edges discovered from the file.
    pub edges: Vec<ExtractedEdge>,
}

/// Parse a Rust source file and extract symbols and edges.
///
/// This function is synchronous and CPU-bound. Callers should run it via
/// `tokio::task::spawn_blocking` to avoid blocking the async runtime.
///
/// # Errors
///
/// Returns an error string if tree-sitter fails to parse the source.
pub fn parse_rust_source(source: &str) -> Result<ParseResult, String> {
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
                    // Discover call edges within the function body.
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

/// Extract a `function_item` node into an `ExtractedFunction`.
fn extract_function(node: Node<'_>, source: &str) -> Option<ExtractedFunction> {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(n, source))?;

    let body = node_text(node, source);
    let body_hash = sha256_hex(&body);
    #[allow(clippy::cast_possible_truncation)]
    let line_start = (node.start_position().row + 1) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let line_end = (node.end_position().row + 1) as u32;

    // Signature = everything before the body block (the `{...}` part).
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

/// Extract a `struct_item` node into an `ExtractedClass`.
fn extract_class(node: Node<'_>, source: &str) -> Option<ExtractedClass> {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(n, source))?;

    let body = node_text(node, source);
    let body_hash = sha256_hex(&body);
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

/// Extract a `trait_item` node into an `ExtractedInterface`.
fn extract_interface(node: Node<'_>, source: &str) -> Option<ExtractedInterface> {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(n, source))?;

    let body = node_text(node, source);
    let body_hash = sha256_hex(&body);
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

/// Extract methods and `inherits_from` edges from an `impl_item` node.
///
/// Methods inside `impl` blocks are extracted as `Function` symbols.
/// If the impl block has a trait reference (`impl Trait for Struct`),
/// an `InheritsFrom` edge is created.
fn extract_impl(
    node: Node<'_>,
    source: &str,
    symbols: &mut Vec<ExtractedSymbol>,
    edges: &mut Vec<ExtractedEdge>,
) {
    // Check for trait impl: `impl Trait for Type`
    let trait_name = node
        .child_by_field_name("trait")
        .map(|n| node_text(n, source));
    let type_name = node
        .child_by_field_name("type")
        .map(|n| node_text(n, source));

    if let (Some(t), Some(s)) = (&trait_name, &type_name) {
        edges.push(ExtractedEdge::InheritsFrom {
            struct_name: s.clone(),
            trait_name: t.clone(),
        });
    }

    // Walk the impl body for function_item children (methods).
    if let Some(body_node) = node.child_by_field_name("body") {
        let mut cursor = body_node.walk();
        for child in body_node.children(&mut cursor) {
            if child.kind() == "function_item" {
                if let Some(mut func) = extract_function(child, source) {
                    // Qualify method name with the impl type to avoid symbol collisions.
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

/// Extract a `use_declaration` path as a string.
///
/// Handles `scoped_identifier`, `scoped_use_list`, and `identifier` forms.
fn extract_use_path(node: Node<'_>, source: &str) -> Option<String> {
    // The use declaration's argument is usually its second child (after `use` keyword).
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "scoped_identifier" | "identifier" | "scoped_use_list" | "use_wildcard"
            | "use_as_clause" | "use_list" => {
                return Some(node_text(child, source));
            }
            _ => {}
        }
    }
    None
}

/// Walk a function body to discover `call_expression` nodes (calls edges)
/// and `method_call_expression` nodes.
fn extract_calls_from_body(
    node: Node<'_>,
    source: &str,
    caller_name: &str,
    edges: &mut Vec<ExtractedEdge>,
) {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        // Only call_expression nodes produce edges; macro invocations
        // and all other node types are skipped (not indexed as functions).
        if current.kind() == "call_expression" {
            if let Some(callee) = resolve_call_name(current, source) {
                edges.push(ExtractedEdge::Calls {
                    caller: caller_name.to_owned(),
                    callee,
                });
            }
        }
        // Push children onto the stack for DFS traversal.
        let mut cursor = current.walk();
        for child in current.children(&mut cursor) {
            stack.push(child);
        }
    }
}

/// Callee names that are too generic to form meaningful edges.
const CALL_BLOCKLIST: &[&str] = &[
    "new", "default", "into", "clone", "from", "unwrap", "expect", "ok", "err",
];

/// Resolve the callee name from a `call_expression` node.
///
/// Handles simple calls (`foo()`), qualified calls (`bar::foo()`),
/// and field access calls (`self.foo()`).
fn resolve_call_name(node: Node<'_>, source: &str) -> Option<String> {
    let function_node = node.child_by_field_name("function")?;
    let name = match function_node.kind() {
        "identifier" => Some(node_text(function_node, source)),
        "scoped_identifier" => {
            // For `path::segment::name(...)`, extract the final segment.
            let mut cursor = function_node.walk();
            function_node
                .children(&mut cursor)
                .filter(|c| c.kind() == "identifier")
                .last()
                .map(|n| node_text(n, source))
        }
        // Skip field_expression (`self.foo()`, `obj.method()`) and all other
        // forms — the receiver object type is not known at parse time, so
        // linking would produce false positives (e.g., every `.clone()` call
        // would become an edge).
        _ => None,
    };
    // Filter out generic method names that produce noise edges.
    name.filter(|n| !CALL_BLOCKLIST.contains(&n.as_str()))
}

/// Extract the function signature (everything before the body block).
fn extract_signature(node: Node<'_>, source: &str) -> String {
    // Find the declaration_list or block node that is the body.
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "block" || child.kind() == "declaration_list" {
            // Signature is from node start to just before the body.
            let sig_end = child.start_byte();
            let sig_start = node.start_byte();
            return source[sig_start..sig_end].trim().to_owned();
        }
    }
    // Fallback: use the full node text.
    node_text(node, source)
}

/// Extract the doc comment immediately preceding a node.
///
/// Looks for adjacent `line_comment` siblings starting with `///` or
/// a single `block_comment` starting with `/**`.
fn extract_docstring(node: Node<'_>, source: &str) -> Option<String> {
    let mut doc_lines = Vec::new();
    let mut sibling = node.prev_sibling();
    while let Some(s) = sibling {
        let text = node_text(s, source);
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
            // Skip attributes like #[derive(...)] that may sit between doc comments.
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

/// Get the text of a tree-sitter node from the source.
fn node_text(node: Node<'_>, source: &str) -> String {
    source[node.byte_range()].to_owned()
}

/// Compute the SHA-256 hex digest of a string.
fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_function() {
        let source = r#"fn hello() { println!("Hello"); }"#;
        let result = parse_rust_source(source).unwrap();
        assert_eq!(result.symbols.len(), 1);
        match &result.symbols[0] {
            ExtractedSymbol::Function(f) => {
                assert_eq!(f.name, "hello");
                assert_eq!(f.line_start, 1);
                assert_eq!(f.line_end, 1);
                assert!(!f.body.is_empty());
                assert!(!f.body_hash.is_empty());
                assert!(f.token_count > 0);
            }
            _ => panic!("Expected Function symbol at index 0, got a different variant"),
        }
        // Should have a Defines edge.
        assert!(result.edges.iter().any(|e| matches!(
            e,
            ExtractedEdge::Defines { symbol_name } if symbol_name == "hello"
        )));
    }

    #[test]
    fn parse_struct_item() {
        let source = "pub struct Config {\n    pub name: String,\n}\n";
        let result = parse_rust_source(source).unwrap();
        assert_eq!(result.symbols.len(), 1);
        match &result.symbols[0] {
            ExtractedSymbol::Class(c) => {
                assert_eq!(c.name, "Config");
                assert_eq!(c.line_start, 1);
                assert_eq!(c.line_end, 3);
            }
            _ => panic!("Expected Class symbol at index 0, got a different variant"),
        }
    }

    #[test]
    fn parse_trait_item() {
        let source = "pub trait Handler {\n    fn handle(&self);\n}\n";
        let result = parse_rust_source(source).unwrap();
        assert_eq!(result.symbols.len(), 1);
        match &result.symbols[0] {
            ExtractedSymbol::Interface(i) => {
                assert_eq!(i.name, "Handler");
            }
            _ => panic!("Expected Interface symbol at index 0, got a different variant"),
        }
    }

    #[test]
    fn parse_impl_block_methods() {
        let source = r#"
struct Foo;

impl Foo {
    fn bar(&self) {}
    fn baz(&self) {}
}
"#;
        let result = parse_rust_source(source).unwrap();
        // 1 struct + 2 methods
        assert_eq!(result.symbols.len(), 3);
        let func_names: Vec<&str> = result
            .symbols
            .iter()
            .filter_map(|s| match s {
                ExtractedSymbol::Function(f) => Some(f.name.as_str()),
                _ => None,
            })
            .collect();
        assert!(func_names.contains(&"Foo::bar"));
        assert!(func_names.contains(&"Foo::baz"));
    }

    #[test]
    fn parse_trait_impl_creates_inherits_edge() {
        let source = r#"
struct MyStruct;
trait MyTrait {}
impl MyTrait for MyStruct {}
"#;
        let result = parse_rust_source(source).unwrap();
        assert!(result.edges.iter().any(|e| matches!(
            e,
            ExtractedEdge::InheritsFrom {
                struct_name,
                trait_name,
            } if struct_name == "MyStruct" && trait_name == "MyTrait"
        )));
    }

    #[test]
    fn parse_use_declaration() {
        let source = "use std::collections::HashMap;\n";
        let result = parse_rust_source(source).unwrap();
        assert!(result.edges.iter().any(|e| matches!(
            e,
            ExtractedEdge::Imports { import_path } if import_path == "std::collections::HashMap"
        )));
    }

    #[test]
    fn parse_call_expression() {
        let source = r#"
fn caller() {
    callee();
}

fn callee() {}
"#;
        let result = parse_rust_source(source).unwrap();
        assert!(result.edges.iter().any(|e| matches!(
            e,
            ExtractedEdge::Calls { caller, callee } if caller == "caller" && callee == "callee"
        )));
    }

    #[test]
    fn parse_doc_comment() {
        let source = "/// This is a doc comment.\n/// Second line.\nfn documented() {}\n";
        let result = parse_rust_source(source).unwrap();
        match &result.symbols[0] {
            ExtractedSymbol::Function(f) => {
                assert_eq!(
                    f.docstring.as_deref(),
                    Some("This is a doc comment.\nSecond line.")
                );
            }
            _ => panic!("Expected Function symbol for parse_doc_comment, got a different variant"),
        }
    }

    #[test]
    fn token_count_uses_char_div_4() {
        let body = "fn example() { let x = 1 + 2; }";
        let result = parse_rust_source(body).unwrap();
        match &result.symbols[0] {
            ExtractedSymbol::Function(f) => {
                #[allow(clippy::cast_possible_truncation)]
                let expected = (f.body.len() / 4) as u32;
                assert_eq!(f.token_count, expected);
            }
            _ => panic!(
                "Expected Function symbol for token_count_uses_char_div_4, got a different variant"
            ),
        }
    }

    #[test]
    fn body_hash_is_sha256() {
        let source = "fn test_hash() {}";
        let result = parse_rust_source(source).unwrap();
        match &result.symbols[0] {
            ExtractedSymbol::Function(f) => {
                // Verify it's a 64-char hex string (SHA-256).
                assert_eq!(f.body_hash.len(), 64);
                assert!(f.body_hash.chars().all(|c| c.is_ascii_hexdigit()));
            }
            _ => {
                panic!("Expected Function symbol for body_hash_is_sha256, got a different variant")
            }
        }
    }

    #[test]
    fn empty_source_produces_empty_result() {
        let result = parse_rust_source("").unwrap();
        assert!(result.symbols.is_empty());
        assert!(result.edges.is_empty());
    }

    #[test]
    fn signature_excludes_body_block() {
        let source = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
        let result = parse_rust_source(source).unwrap();
        match &result.symbols[0] {
            ExtractedSymbol::Function(f) => {
                assert_eq!(f.signature, "pub fn add(a: i32, b: i32) -> i32");
            }
            _ => panic!(
                "Expected Function symbol for signature_excludes_body_block, got a different variant"
            ),
        }
    }
}
