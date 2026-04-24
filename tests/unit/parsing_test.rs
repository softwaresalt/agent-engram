//! Unit tests for tree-sitter node extraction (T030).
//!
//! Tests function, struct, trait, impl, call site, and use declaration extraction
//! from the parsing service.

#![allow(clippy::needless_raw_string_hashes)]

use engram::services::parsing::{
    ExtractedEdge, ExtractedSymbol, Language, parse_rust_source, parse_source,
};

/// Debug helper: dump the raw tree-sitter node kinds from a C++ class body.
///
/// Run with `cargo test test_cpp_inline_tree_debug -- --nocapture` to see output.
#[test]
fn test_cpp_inline_tree_debug() {
    fn dump_cpp(source: &str, label: &str) {
        use tree_sitter::Parser;
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_cpp::LANGUAGE.into())
            .expect("C++ grammar load");
        let tree = parser.parse(source, None).expect("parse");
        let root = tree.root_node();
        eprintln!("=== {label} ===");
        eprintln!("root kind: {}", root.kind());
        for i in 0..root.child_count() {
            let Some(c) = root.child(i) else { continue };
            eprintln!("  [{}] kind={} named={}", i, c.kind(), c.is_named());
            if c.kind().contains("class")
                || c.kind().contains("struct")
                || c.kind() == "declaration"
            {
                if let Some(body) = c.child_by_field_name("body") {
                    eprintln!("    BODY field kind: {}", body.kind());
                    for j in 0..body.child_count() {
                        let Some(bc) = body.child(j) else { continue };
                        eprintln!(
                            "      BODY[{j}] kind={} text={:?}",
                            bc.kind(),
                            &source[bc.byte_range()]
                        );
                        // Descend one more level for complex nodes
                        for k in 0..bc.child_count() {
                            let Some(bcc) = bc.child(k) else { continue };
                            eprintln!(
                                "        [{k}] kind={} field={:?} text={:?}",
                                bcc.kind(),
                                bc.field_name_for_child(k as u32),
                                &source[bcc.byte_range()]
                            );
                        }
                    }
                } else {
                    eprintln!("    NO 'body' field on {}", c.kind());
                    for j in 0..c.child_count() {
                        let Some(cc) = c.child(j) else { continue };
                        eprintln!("    CHILD[{j}] kind={}", cc.kind());
                    }
                }
                if c.kind() == "declaration" {
                    if let Some(ty) = c.child_by_field_name("type") {
                        eprintln!("    type field kind: {}", ty.kind());
                        if let Some(body2) = ty.child_by_field_name("body") {
                            eprintln!("      type.body kind: {}", body2.kind());
                            for j in 0..body2.child_count() {
                                let Some(bc) = body2.child(j) else { continue };
                                eprintln!(
                                    "        type.BODY[{j}] kind={} text={:?}",
                                    bc.kind(),
                                    &source[bc.byte_range()]
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    dump_cpp(
        "class Calculator { int add(int a, int b) { return a + b; } };",
        "class with int method",
    );
    dump_cpp(
        "struct Point { int sum() { return 1; } };",
        "struct with int method",
    );
    dump_cpp(
        r#"class Foo { std::string greet() { return "hi"; } };"#,
        "class with std::string method",
    );
    dump_cpp(
        "class Ops { Ops operator+(Ops o) { return o; } };",
        "class with operator",
    );
    dump_cpp(
        "class W { W(int i) {} ~W() {} };",
        "class with ctor/dtor",
    );
}

#[test]
fn extracts_top_level_function() {
    let source = r#"
fn process_payment(amount: u64) -> bool {
    validate(amount);
    true
}
"#;
    let result = parse_rust_source(source).unwrap();
    let funcs: Vec<_> = result
        .symbols
        .iter()
        .filter_map(|s| match s {
            ExtractedSymbol::Function(f) => Some(f),
            _ => None,
        })
        .collect();
    assert_eq!(funcs.len(), 1);
    assert_eq!(funcs[0].name, "process_payment");
    assert!(funcs[0].line_start >= 1);
    assert!(funcs[0].line_end >= funcs[0].line_start);
    assert!(funcs[0].body.contains("validate(amount)"));
    assert_eq!(funcs[0].body_hash.len(), 64);
    assert!(funcs[0].token_count > 0);
}

#[test]
fn extracts_function_signature() {
    let source = "pub fn greet(name: &str) -> String { format!(\"Hello {name}\") }";
    let result = parse_rust_source(source).unwrap();
    match &result.symbols[0] {
        ExtractedSymbol::Function(f) => {
            assert_eq!(f.signature, "pub fn greet(name: &str) -> String");
        }
        _ => panic!("Expected Function"),
    }
}

#[test]
fn extracts_struct_as_class() {
    let source = r#"
/// A billing record.
pub struct Invoice {
    pub id: u64,
    pub amount: f64,
    pub paid: bool,
}
"#;
    let result = parse_rust_source(source).unwrap();
    let classes: Vec<_> = result
        .symbols
        .iter()
        .filter_map(|s| match s {
            ExtractedSymbol::Class(c) => Some(c),
            _ => None,
        })
        .collect();
    assert_eq!(classes.len(), 1);
    assert_eq!(classes[0].name, "Invoice");
    assert!(classes[0].body.contains("amount: f64"));
    assert!(classes[0].docstring.as_deref().unwrap().contains("billing"));
}

#[test]
fn extracts_trait_as_interface() {
    let source = r#"
/// A service handler trait.
pub trait Handler {
    fn handle(&self, request: Request) -> Response;
    fn name(&self) -> &str;
}
"#;
    let result = parse_rust_source(source).unwrap();
    let interfaces: Vec<_> = result
        .symbols
        .iter()
        .filter_map(|s| match s {
            ExtractedSymbol::Interface(i) => Some(i),
            _ => None,
        })
        .collect();
    assert_eq!(interfaces.len(), 1);
    assert_eq!(interfaces[0].name, "Handler");
    assert!(interfaces[0].body.contains("fn handle"));
    assert!(
        interfaces[0]
            .docstring
            .as_deref()
            .unwrap()
            .contains("handler")
    );
}

#[test]
fn extracts_impl_methods_as_functions() {
    let source = r#"
struct Calculator;

impl Calculator {
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    pub fn multiply(&self, a: i32, b: i32) -> i32 {
        a * b
    }
}
"#;
    let result = parse_rust_source(source).unwrap();
    let func_names: Vec<_> = result
        .symbols
        .iter()
        .filter_map(|s| match s {
            ExtractedSymbol::Function(f) => Some(f.name.as_str()),
            _ => None,
        })
        .collect();
    assert!(func_names.contains(&"Calculator::add"));
    assert!(func_names.contains(&"Calculator::multiply"));
    assert_eq!(func_names.len(), 2);
}

#[test]
fn extracts_trait_impl_inherits_edge() {
    let source = r#"
struct MyService;
trait ServiceHandler {}
impl ServiceHandler for MyService {}
"#;
    let result = parse_rust_source(source).unwrap();
    let inherits: Vec<_> = result
        .edges
        .iter()
        .filter(|e| matches!(e, ExtractedEdge::InheritsFrom { .. }))
        .collect();
    assert_eq!(inherits.len(), 1);
    match &inherits[0] {
        ExtractedEdge::InheritsFrom {
            struct_name,
            trait_name,
        } => {
            assert_eq!(struct_name, "MyService");
            assert_eq!(trait_name, "ServiceHandler");
        }
        _ => unreachable!(),
    }
}

#[test]
fn extracts_call_expression_edges() {
    let source = r#"
fn orchestrate() {
    step_one();
    step_two();
}

fn step_one() {}
fn step_two() {}
"#;
    let result = parse_rust_source(source).unwrap();
    let calls: Vec<_> = result
        .edges
        .iter()
        .filter_map(|e| match e {
            ExtractedEdge::Calls { caller, callee } => Some((caller.as_str(), callee.as_str())),
            _ => None,
        })
        .collect();
    assert!(calls.contains(&("orchestrate", "step_one")));
    assert!(calls.contains(&("orchestrate", "step_two")));
}

#[test]
fn extracts_scoped_call_final_segment() {
    let source = r#"
fn caller() {
    billing::process_payment();
}
"#;
    let result = parse_rust_source(source).unwrap();
    assert!(result.edges.iter().any(|e| matches!(
        e,
        ExtractedEdge::Calls { caller, callee } if caller == "caller" && callee == "process_payment"
    )));
}

#[test]
fn extracts_use_declarations() {
    let source = r#"
use std::collections::HashMap;
use crate::models::Task;
"#;
    let result = parse_rust_source(source).unwrap();
    let imports: Vec<_> = result
        .edges
        .iter()
        .filter_map(|e| match e {
            ExtractedEdge::Imports { import_path } => Some(import_path.as_str()),
            _ => None,
        })
        .collect();
    assert!(imports.contains(&"std::collections::HashMap"));
    assert!(imports.contains(&"crate::models::Task"));
}

#[test]
fn creates_defines_edges_for_top_level_symbols() {
    let source = r#"
fn top_func() {}
struct TopStruct;
trait TopTrait {}
"#;
    let result = parse_rust_source(source).unwrap();
    let defines: Vec<_> = result
        .edges
        .iter()
        .filter_map(|e| match e {
            ExtractedEdge::Defines { symbol_name } => Some(symbol_name.as_str()),
            _ => None,
        })
        .collect();
    assert!(defines.contains(&"top_func"));
    assert!(defines.contains(&"TopStruct"));
    assert!(defines.contains(&"TopTrait"));
}

#[test]
fn skips_macro_invocations_in_call_discovery() {
    let source = r#"
fn with_macros() {
    println!("hello");
    real_call();
}
"#;
    let result = parse_rust_source(source).unwrap();
    let calls: Vec<_> = result
        .edges
        .iter()
        .filter_map(|e| match e {
            ExtractedEdge::Calls { callee, .. } => Some(callee.as_str()),
            _ => None,
        })
        .collect();
    // Should have real_call but NOT println.
    assert!(calls.contains(&"real_call"));
    assert!(!calls.iter().any(|c| c.contains("println")));
}

#[test]
fn extracts_doc_comments_across_attributes() {
    let source = r#"
/// Important function.
#[inline]
fn attributed() {}
"#;
    let result = parse_rust_source(source).unwrap();
    match &result.symbols[0] {
        ExtractedSymbol::Function(f) => {
            assert_eq!(f.docstring.as_deref(), Some("Important function."));
        }
        _ => panic!("Expected Function"),
    }
}

#[test]
fn body_hash_is_deterministic() {
    let source = "fn stable() { let x = 42; }";
    let r1 = parse_rust_source(source).unwrap();
    let r2 = parse_rust_source(source).unwrap();
    match (&r1.symbols[0], &r2.symbols[0]) {
        (ExtractedSymbol::Function(f1), ExtractedSymbol::Function(f2)) => {
            assert_eq!(f1.body_hash, f2.body_hash);
        }
        _ => panic!("Expected Function"),
    }
}

#[test]
fn handles_complex_mixed_file() {
    let source = r#"
use std::fmt;

/// A config struct.
pub struct Config {
    pub name: String,
}

pub trait Configurable {
    fn configure(&mut self);
}

impl Configurable for Config {
    fn configure(&mut self) {
        self.name = default_name();
    }
}

fn default_name() -> String {
    "default".to_string()
}
"#;
    let result = parse_rust_source(source).unwrap();

    // 1 struct (Config) + 1 trait (Configurable) + 1 impl method (Config::configure) + 1 fn (default_name)
    assert_eq!(result.symbols.len(), 4);

    // Should have Imports, Defines, InheritsFrom, and Calls edges.
    assert!(
        result
            .edges
            .iter()
            .any(|e| matches!(e, ExtractedEdge::Imports { .. }))
    );
    assert!(
        result
            .edges
            .iter()
            .any(|e| matches!(e, ExtractedEdge::Defines { .. }))
    );
    assert!(
        result
            .edges
            .iter()
            .any(|e| matches!(e, ExtractedEdge::InheritsFrom { .. }))
    );
    assert!(result.edges.iter().any(|e| matches!(
        e,
        ExtractedEdge::Calls { caller, callee } if caller == "Config::configure" && callee == "default_name"
    )));
}

#[test]
fn token_count_matches_body_div_4() {
    let source = r#"
fn verbose_function() {
    let a = 1;
    let b = 2;
    let c = a + b;
    let d = c * 2;
    let e = d - 1;
}
"#;
    let result = parse_rust_source(source).unwrap();
    match &result.symbols[0] {
        ExtractedSymbol::Function(f) => {
            #[allow(clippy::cast_possible_truncation)]
            let expected = (f.body.len() / 4) as u32;
            assert_eq!(f.token_count, expected);
        }
        _ => panic!("Expected Function"),
    }
}

// ── SI-1 infrastructure tests ────────────────────────────────────────────────
// Verify shared infrastructure added in 027.001-T.
// PASS once SI-1 is in place and continue to pass through all later tasks.

#[test]
fn si1_language_enum_new_variants_exist() {
    assert_eq!(Language::C.as_str(), "c");
    assert_eq!(Language::Cpp.as_str(), "cpp");
    assert_eq!(Language::Swift.as_str(), "swift");
    assert_eq!(Language::Kotlin.as_str(), "kotlin");
}

#[test]
fn si1_language_tryfrom_str_new_variants() {
    assert!(Language::try_from("c").is_ok());
    assert!(Language::try_from("cpp").is_ok());
    assert!(Language::try_from("swift").is_ok());
    assert!(Language::try_from("kotlin").is_ok());
}

#[test]
fn si1_parse_source_returns_ok_empty_for_stubs() {
    // Empty input must return Ok(empty) for all language variants —
    // not Err — so callers can distinguish "no symbols found" from parse failure.
    // Swift has a full implementation but still returns empty on empty input.
    let swift_result = parse_source("", Language::Swift).unwrap();
    assert!(swift_result.symbols.is_empty());
    assert!(swift_result.edges.is_empty());

    // Kotlin is a no-op stub (deferred: tree-sitter-kotlin incompatible with >=0.25).
    let kotlin_result = parse_source("", Language::Kotlin).unwrap();
    assert!(kotlin_result.symbols.is_empty());
    assert!(kotlin_result.edges.is_empty());
}

// ── A-1 spike: Swift grammar ABI verification (027.002-T) ───────────────────
// Verifies tree-sitter-swift loads and parses without ABI or runtime error.
// tree-sitter-swift 0.7.1 emits ABI 15; requires tree-sitter >=0.25 runtime.

#[test]
fn a1_spike_swift_grammar_loads() {
    // Swift parser is fully implemented (swift.rs). This test confirms the
    // grammar loads and parse_source returns Ok for valid Swift input.
    let result = parse_source("func foo() { }", Language::Swift);
    assert!(
        result.is_ok(),
        "Swift grammar failed to load: {:?}",
        result.err()
    );
}

// ── A-2/A-3: Swift parser (027.003-T / 027.004-T) ───────────────────────────
// Swift parsing is fully implemented in swift.rs.

#[test]
fn test_swift_parsing() {
    let source = r#"
import Foundation

protocol Greetable {
    func greet() -> String
}

struct Person: Greetable {
    var name: String
    func greet() -> String { "Hello, \(name)" }
}

func make_person(name: String) -> Person {
    return Person(name: name)
}
"#;
    let result = parse_source(source, Language::Swift).unwrap();
    let func_count = result
        .symbols
        .iter()
        .filter(|s| matches!(s, ExtractedSymbol::Function(_)))
        .count();
    let class_count = result
        .symbols
        .iter()
        .filter(|s| matches!(s, ExtractedSymbol::Class(_)))
        .count();
    let iface_count = result
        .symbols
        .iter()
        .filter(|s| matches!(s, ExtractedSymbol::Interface(_)))
        .count();
    assert!(func_count >= 1, "expected ≥1 Function, got {func_count}");
    assert!(
        class_count >= 1,
        "expected ≥1 Class (struct/actor), got {class_count}"
    );
    assert!(
        iface_count >= 1,
        "expected ≥1 Interface (protocol), got {iface_count}"
    );
    assert!(
        result
            .edges
            .iter()
            .any(|e| matches!(e, ExtractedEdge::Imports { .. })),
        "expected at least one Imports edge"
    );
}

// ── B-1 spike: Kotlin no-op stub validation (027.005-T) ─────────────────────
// NOTE: This test validates that the no-op Kotlin stub returns Ok(empty result)
// without panicking.  It does NOT verify grammar or ABI compatibility — the
// stub never calls Parser::set_language(), so it cannot detect ABI issues.
// Real grammar compatibility must be tested once a tree-sitter 0.25-compatible
// Kotlin crate is available.

#[test]
fn b1_kotlin_stub_returns_ok() {
    let result = parse_source("fun foo() { }", Language::Kotlin);
    assert!(
        result.is_ok(),
        "Kotlin stub returned error unexpectedly: {:?}",
        result.err()
    );
    let pr = result.unwrap();
    assert!(pr.symbols.is_empty(), "stub should return no symbols");
}

// ── B-2/B-3: Kotlin parser (027.006-T / 027.007-T) ──────────────────────────
// IGNORED: tree-sitter-kotlin 0.3.x depends on tree-sitter 0.20–0.22 which
// conflicts with the project-wide tree-sitter 0.25 runtime.  Kotlin support
// is deferred until a 0.25-compatible grammar crate is available.
// Track: see stash item for "Kotlin tree-sitter 0.25 compat".

#[test]
#[ignore = "deferred: tree-sitter-kotlin incompatible with tree-sitter 0.25"]
fn test_kotlin_parsing() {
    let source = r#"
import java.lang.String

interface Greetable {
    fun greet(): String
}

data class Person(val name: String) : Greetable {
    override fun greet(): String {
        return "Hello, $name"
    }
}

fun make_person(name: String): Person {
    return Person(name)
}
"#;
    let result = parse_source(source, Language::Kotlin).unwrap();
    let func_count = result
        .symbols
        .iter()
        .filter(|s| matches!(s, ExtractedSymbol::Function(_)))
        .count();
    let class_count = result
        .symbols
        .iter()
        .filter(|s| matches!(s, ExtractedSymbol::Class(_)))
        .count();
    let iface_count = result
        .symbols
        .iter()
        .filter(|s| matches!(s, ExtractedSymbol::Interface(_)))
        .count();
    assert!(func_count >= 1, "expected ≥1 Function, got {func_count}");
    assert!(
        class_count >= 1,
        "expected ≥1 Class (data class), got {class_count}"
    );
    assert!(iface_count >= 1, "expected ≥1 Interface, got {iface_count}");
    assert!(
        result
            .edges
            .iter()
            .any(|e| matches!(e, ExtractedEdge::Imports { .. })),
        "expected at least one Imports edge"
    );
}

// ── C-1/C-2: C parser (027.008-T / 027.009-T) ───────────────────────────────
// FAIL until C-1 implements real extraction in c.rs.

#[test]
fn test_c_parsing() {
    let source = r#"
#include <stdio.h>

struct Point {
    int x;
    int y;
};

int add(int a, int b) {
    return a + b;
}

void print_point(struct Point *p) {
    add(p->x, p->y);
}
"#;
    let result = parse_source(source, Language::C).unwrap();
    let func_count = result
        .symbols
        .iter()
        .filter(|s| matches!(s, ExtractedSymbol::Function(_)))
        .count();
    let class_count = result
        .symbols
        .iter()
        .filter(|s| matches!(s, ExtractedSymbol::Class(_)))
        .count();
    assert!(func_count >= 1, "expected ≥1 Function, got {func_count}");
    assert!(
        class_count >= 1,
        "expected ≥1 Class (struct), got {class_count}"
    );
    assert!(
        result
            .edges
            .iter()
            .any(|e| matches!(e, ExtractedEdge::Imports { .. })),
        "expected Imports edge for #include"
    );
}

// ── D-1/D-2: C++ parser (027.010-T / 027.011-T) ─────────────────────────────

#[test]
fn test_cpp_parsing() {

    let source = r#"
#include <string>

class Greeter {
public:
    std::string greet(const std::string& name);
    int count() const;
};

std::string Greeter::greet(const std::string& name) {
    return "Hello, " + name;
}

int Greeter::count() const {
    return 0;
}

int free_function() {
    return 42;
}
"#;
    let result = parse_source(source, Language::Cpp).unwrap();
    let func_count = result
        .symbols
        .iter()
        .filter(|s| matches!(s, ExtractedSymbol::Function(_)))
        .count();
    let class_count = result
        .symbols
        .iter()
        .filter(|s| matches!(s, ExtractedSymbol::Class(_)))
        .count();
    // class with 2 out-of-line methods + 1 free function = ≥3 Function symbols
    assert!(
        func_count >= 3,
        "expected class 2 methods + free fn (≥3 Function), got {func_count}"
    );
    assert!(class_count >= 1, "expected ≥1 Class, got {class_count}");
    assert!(
        result
            .edges
            .iter()
            .any(|e| matches!(e, ExtractedEdge::Imports { .. })),
        "expected Imports edge for #include"
    );
}

// ── E-1 to E-5: C++ inline member extraction (030.002-C) ─────────────────────

/// Inline methods declared inside a class body must be extracted with a
/// `ClassName::method` qualified name.
#[test]
fn test_cpp_inline_method_in_class() {
    let source = r#"
#include <string>

class Greeter {
public:
    std::string name;
    std::string greet() {
        return "Hello, " + name;
    }
    int count() const {
        return 42;
    }
};
"#;
    let result = parse_source(source, Language::Cpp).unwrap();
    let func_names: Vec<&str> = result
        .symbols
        .iter()
        .filter_map(|s| match s {
            ExtractedSymbol::Function(f) => Some(f.name.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        func_names.iter().any(|n| n.contains("greet")),
        "expected 'greet' inline method, got: {func_names:?}"
    );
    assert!(
        func_names.iter().any(|n| n.contains("count")),
        "expected 'count' inline method, got: {func_names:?}"
    );
    let qualified: Vec<&&str> = func_names.iter().filter(|n| n.contains("::")).collect();
    assert!(
        !qualified.is_empty(),
        "at least one inline method must have a qualified name (ClassName::method), got: {func_names:?}"
    );
}

/// Struct inline methods must also be extracted.
#[test]
fn test_cpp_inline_struct_method() {
    let source = r#"
struct Point {
    int x;
    int y;
    int sum() {
        return x + y;
    }
};
"#;
    let result = parse_source(source, Language::Cpp).unwrap();
    let funcs: Vec<&str> = result
        .symbols
        .iter()
        .filter_map(|s| match s {
            ExtractedSymbol::Function(f) => Some(f.name.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        funcs.iter().any(|n| n.contains("sum")),
        "expected 'sum' struct inline method, got: {funcs:?}"
    );
}

/// Inline operator overloads must be extracted with their operator token name.
#[test]
fn test_cpp_inline_operator_overload() {
    let source = r#"
class MyInt {
    int val;
public:
    MyInt operator+(const MyInt& other) {
        return MyInt();
    }
    bool operator==(const MyInt& other) const {
        return val == other.val;
    }
};
"#;
    let result = parse_source(source, Language::Cpp).unwrap();
    let func_names: Vec<&str> = result
        .symbols
        .iter()
        .filter_map(|s| match s {
            ExtractedSymbol::Function(f) => Some(f.name.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        func_names.iter().any(|n| n.contains("operator")),
        "expected operator overload to be extracted, got: {func_names:?}"
    );
}

/// Inline methods must use the exact `ClassName::method` qualified name format.
#[test]
fn test_cpp_inline_method_qualified_name_format() {
    let source = r#"
class Calculator {
    int add(int a, int b) {
        return a + b;
    }
};
"#;
    let result = parse_source(source, Language::Cpp).unwrap();
    let func = result.symbols.iter().find_map(|s| match s {
        ExtractedSymbol::Function(f) if f.name.contains("add") => Some(f),
        _ => None,
    });
    assert!(func.is_some(), "expected 'add' inline method to be extracted");
    let name = &func.unwrap().name;
    assert_eq!(
        name, "Calculator::add",
        "inline method must use fully qualified name ClassName::method"
    );
}

/// Constructors and destructors defined inline in a class body must be extracted.
#[test]
fn test_cpp_inline_constructor_destructor() {
    let source = r#"
class Widget {
    int id;
public:
    Widget(int i) : id(i) {}
    ~Widget() {}
};
"#;
    let result = parse_source(source, Language::Cpp).unwrap();
    // Constructors/destructors appear as function_definition inside the class body.
    let func_count = result
        .symbols
        .iter()
        .filter(|s| matches!(s, ExtractedSymbol::Function(_)))
        .count();
    assert!(
        func_count >= 1,
        "expected constructor/destructor to be extracted, got {func_count} functions"
    );
}
