//! RED harness for the HCL grammar ABI and Engram registration boundary (121.007-T).

use engram::services::parsing::Language;
use tree_sitter::Parser;

struct AbiCase {
    extension: &'static str,
    source: &'static str,
}

const ABI_CASES: [AbiCase; 3] = [
    AbiCase {
        extension: ".hcl",
        source: "service \"api\" { port = 8080 }\n",
    },
    AbiCase {
        extension: ".tf",
        source: "resource \"aws_instance\" \"web\" { ami = \"ami-123\" }\n",
    },
    AbiCase {
        extension: ".tfvars",
        source: "region = \"us-west-2\"\n",
    },
];

fn assert_compact_hcl_parses(parser: &mut Parser, case: &AbiCase) -> Result<(), String> {
    let tree = parser.parse(case.source, None).ok_or_else(|| {
        format!(
            "ABI_MISMATCH:HCL_PARSE_NONE {} input produced no tree",
            case.extension
        )
    })?;
    let root = tree.root_node();

    if root.is_error() || root.is_missing() || root.has_error() {
        return Err(format!(
            "ABI_MISMATCH:HCL_PARSE_ERROR {} root={} sexp={}",
            case.extension,
            root.kind(),
            root.to_sexp()
        ));
    }
    if root.end_byte() != case.source.len() {
        return Err(format!(
            "ABI_MISMATCH:HCL_PARSE_INCOMPLETE {} consumed {} of {} bytes",
            case.extension,
            root.end_byte(),
            case.source.len()
        ));
    }

    eprintln!("ABI_PASS:{} root={}", case.extension, root.kind());
    Ok(())
}

#[test]
fn hcl_grammar_abi_loads_before_engram_registration() -> Result<(), String> {
    let grammar = tree_sitter_hcl::LANGUAGE.into();
    let mut parser = Parser::new();
    parser.set_language(&grammar).map_err(|error| {
        format!("ABI_MISMATCH:HCL_LANGUAGE_LOAD tree-sitter rejected the grammar: {error}")
    })?;
    eprintln!("ABI_PASS:LANGUAGE_LOAD");

    for case in &ABI_CASES {
        assert_compact_hcl_parses(&mut parser, case)?;
    }

    let registration = Language::try_from("hcl");
    assert!(
        matches!(&registration, Ok(language) if language.as_str() == "hcl"),
        "RED:HCL_ENGRAM_REGISTRATION_MISSING grammar ABI passed for .hcl, .tf, and .tfvars, \
         but Language::try_from(\"hcl\") is not canonically registered: {registration:?}"
    );

    Ok(())
}
