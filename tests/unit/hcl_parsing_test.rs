//! Dependency-agnostic RED parser harness for HCL declarations and traversals (121.002-T).

use engram::services::parsing::{ExtractedEdge, ExtractedSymbol, Language, parse_source};

const DECLARATION_SOURCE: &str = r#"
terraform {
  required_version = ">= 1.6"
}

resource "aws_instance" "web" {
  ami = "ami-123"
}

data "aws_ami" "ubuntu" {
  most_recent = true
}

module "vpc" {
  source = "./vpc"
}

region = "us-west-2"
"#;

const TRAVERSAL_SOURCE: &str = r#"
resource "aws_instance" "web" {
  region           = var.region
  duplicate_region = var.region
  subnet_id        = module.vpc.id
  image_id         = data.aws_ami.ubuntu.id
  indexed          = local.items[count.index]
  splatted         = aws_instance.web[*].id
  rendered         = "${local.name}"
}
"#;

const MALFORMED_SOURCE: &str = r#"
resource "aws_instance" "broken" {
  ami = var.
"#;

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
    let parsed = parse_source(DECLARATION_SOURCE, language).expect("parse valid HCL declarations");

    let expected = [
        "hcl.block.terraform",
        "hcl.block.resource.aws_instance.web",
        "hcl.block.data.aws_ami.ubuntu",
        "hcl.block.module.vpc",
        "hcl.attribute.region",
    ];
    assert_eq!(structural_names(&parsed), expected);

    let defines: Vec<&str> = parsed
        .edges
        .iter()
        .filter_map(|edge| match edge {
            ExtractedEdge::Defines { symbol_name } => Some(symbol_name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(defines, expected);
}

#[test]
fn plain_traversals_are_stable_deduplicated_and_dynamic_forms_fail_closed() {
    let language = hcl_language("HCL_TRAVERSAL_PARSER_MISSING");
    let parsed = parse_source(TRAVERSAL_SOURCE, language).expect("parse valid HCL traversals");

    assert_eq!(
        reference_targets(&parsed),
        ["var.region", "module.vpc.id", "data.aws_ami.ubuntu.id"],
        "plain traversals must retain first-encounter order while duplicate, index, splat, and template forms are skipped"
    );

    let malformed = parse_source(MALFORMED_SOURCE, language).expect("malformed HCL stays bounded");
    assert!(
        malformed.symbols.is_empty() && malformed.edges.is_empty(),
        "malformed HCL must not fabricate declarations or traversal targets: {malformed:?}"
    );
}
