//! Unit tests for tree-sitter node extraction (T030).
//!
//! Tests function, struct, trait, impl, call site, and use declaration extraction
//! from the parsing service.

#![allow(clippy::needless_raw_string_hashes)]

use engram::services::parsing::{ExtractedEdge, ExtractedSymbol, parse_rust_source};

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
    assert!(func_names.contains(&"add"));
    assert!(func_names.contains(&"multiply"));
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

    // 1 struct (Config) + 1 trait (Configurable) + 1 impl method (configure) + 1 fn (default_name)
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
        ExtractedEdge::Calls { caller, callee } if caller == "configure" && callee == "default_name"
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
