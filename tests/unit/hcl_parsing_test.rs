//! Dependency-agnostic RED parser harness for HCL declarations and traversals (121.002-T).

use engram::services::parsing::{ExtractedEdge, ExtractedSymbol, Language, parse_source};

#[path = "../fixtures/hcl_parser_cases.rs"]
mod hcl_parser_cases;

use hcl_parser_cases::{DECLARATIONS, MALFORMED, TRAVERSALS};

fn hcl_language(marker: &str) -> Language {
    let result = Language::try_from("hcl");
    assert!(
        result.is_ok(),
        "RED:{marker} Language::try_from(\"hcl\") is unsupported: {result:?}"
    );
    result.expect("asserted HCL language support")
}

fn structural_names(result: &engram::services::parsing::ParseResult) -> Vec<&str> {
    result
        .symbols
        .iter()
        .filter_map(|symbol| match symbol {
            ExtractedSymbol::Class(class) => Some(class.name.as_str()),
            ExtractedSymbol::Function(_) | ExtractedSymbol::Interface(_) => None,
        })
        .collect()
}

fn reference_targets(result: &engram::services::parsing::ParseResult) -> Vec<&str> {
    result
        .edges
        .iter()
        .filter_map(|edge| match edge {
            ExtractedEdge::References { target, .. } => Some(target.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn canonical_hcl_language_is_available_without_a_terraform_alias() {
    let language = hcl_language("HCL_LANGUAGE_UNSUPPORTED");
    assert_eq!(language.as_str(), "hcl");
    assert!(
        Language::try_from("terraform").is_err(),
        "`terraform` is an extension-family description, not a language identity"
    );
}

#[test]
fn top_level_blocks_and_attributes_are_namespaced_structural_symbols() {
    let language = hcl_language("HCL_DECLARATION_PARSER_MISSING");
    let parsed = parse_source(DECLARATIONS.source, language).expect("parse valid HCL declarations");

    assert_eq!(
        structural_names(&parsed),
        DECLARATIONS.symbols,
        "fixture: {}",
        DECLARATIONS.name
    );

    let defines: Vec<&str> = parsed
        .edges
        .iter()
        .filter_map(|edge| match edge {
            ExtractedEdge::Defines { symbol_name } => Some(symbol_name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(defines, DECLARATIONS.symbols);
}

#[test]
fn plain_traversals_are_stable_deduplicated_and_dynamic_forms_fail_closed() {
    let language = hcl_language("HCL_TRAVERSAL_PARSER_MISSING");
    let parsed = parse_source(TRAVERSALS.source, language).expect("parse valid HCL traversals");

    assert_eq!(
        reference_targets(&parsed),
        TRAVERSALS.references,
        "fixture `{}` must retain first-encounter order while duplicate, index, splat, and template forms are skipped",
        TRAVERSALS.name
    );

    let malformed = parse_source(MALFORMED.source, language).expect("malformed HCL stays bounded");
    assert!(
        structural_names(&malformed) == MALFORMED.symbols
            && reference_targets(&malformed) == MALFORMED.references,
        "fixture `{}` must not fabricate declarations or traversal targets: {malformed:?}",
        MALFORMED.name
    );
}
