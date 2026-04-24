//! Tree-sitter based AST parsing service (multi-language).
//!
//! Provides symbol and edge extraction for Rust, Python, TypeScript, JavaScript,
//! Go, and C# source files. Each language has a dedicated submodule; this root
//! module defines the shared types, the [`Language`] enum, and the
//! [`parse_source`] dispatcher.

mod c;
mod cpp;
mod csharp;
mod go_lang;
mod javascript;
mod kotlin;
mod markdown;
mod python;
mod rust;
mod swift;
mod typescript;

use sha2::{Digest, Sha256};

use crate::errors::{CodeGraphError, EngramError};

/// Supported source-language identifiers for Tier-1 code graph parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// Rust (`.rs`)
    Rust,
    /// Python (`.py`)
    Python,
    /// TypeScript (`.ts`)
    TypeScript,
    /// TypeScript with JSX (`.tsx`)
    Tsx,
    /// JavaScript (`.js`, `.jsx`)
    JavaScript,
    /// Go (`.go`)
    Go,
    /// C# (`.cs`)
    CSharp,
    /// C (`.c`, `.h`)
    C,
    /// C++ (`.cpp`, `.cc`, `.cxx`, `.hpp`, `.hh`, `.hxx`)
    Cpp,
    /// Swift (`.swift`)
    Swift,
    /// Kotlin (`.kt`, `.kts`)
    Kotlin,
    /// Markdown (`.md`)
    Markdown,
}

impl Language {
    /// Return the canonical language identifier string used in the code graph.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::Tsx => "tsx",
            Language::JavaScript => "javascript",
            Language::Go => "go",
            Language::CSharp => "csharp",
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::Swift => "swift",
            Language::Kotlin => "kotlin",
            Language::Markdown => "markdown",
        }
    }
}

impl TryFrom<&str> for Language {
    type Error = EngramError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "rust" => Ok(Language::Rust),
            "python" => Ok(Language::Python),
            "typescript" => Ok(Language::TypeScript),
            "tsx" => Ok(Language::Tsx),
            "javascript" => Ok(Language::JavaScript),
            "go" => Ok(Language::Go),
            "csharp" => Ok(Language::CSharp),
            "c" => Ok(Language::C),
            "cpp" => Ok(Language::Cpp),
            "swift" => Ok(Language::Swift),
            "kotlin" => Ok(Language::Kotlin),
            "markdown" => Ok(Language::Markdown),
            _ => Err(EngramError::CodeGraph(CodeGraphError::ParseFailed {
                reason: format!("unsupported language: {value}"),
            })),
        }
    }
}

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

/// Parse a source file using the tree-sitter grammar for the given language.
///
/// This function is synchronous and CPU-bound. Callers should run it via
/// `tokio::task::spawn_blocking` to avoid blocking the async runtime.
///
/// # Errors
///
/// Returns an [`EngramError`] if the grammar cannot be initialised or if
/// tree-sitter fails to produce a valid parse tree.
pub fn parse_source(source: &str, language: Language) -> Result<ParseResult, EngramError> {
    match language {
        Language::Rust => rust::parse_rust_source(source)
            .map_err(|reason| EngramError::CodeGraph(CodeGraphError::ParseFailed { reason })),
        Language::Python => python::parse_python_source(source),
        Language::TypeScript => typescript::parse_typescript_source(source),
        Language::Tsx => typescript::parse_tsx_source(source),
        Language::JavaScript => javascript::parse_javascript_source(source),
        Language::Go => go_lang::parse_go_source(source),
        Language::CSharp => csharp::parse_csharp_source(source),
        Language::C => c::parse_c_source(source),
        Language::Cpp => cpp::parse_cpp_source(source),
        Language::Swift => swift::parse_swift_source(source),
        Language::Kotlin => kotlin::parse_kotlin_source(source),
        Language::Markdown => markdown::parse_markdown_source(source),
    }
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
    rust::parse_rust_source(source)
}

/// Get the text of a tree-sitter node from the source.
fn node_text(node: tree_sitter::Node<'_>, source: &str) -> String {
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
