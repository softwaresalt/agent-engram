//! RED security and resource harness for untrusted HCL input (121.004-T).

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;
use engram::services::parsing::{ExtractedEdge, Language, parse_source};

#[allow(dead_code)]
#[path = "../fixtures/hcl_parser_cases.rs"]
mod hcl_parser_cases;

use hcl_parser_cases::{MALFORMED, TRAVERSALS};

fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create fixture parent");
    }
    std::fs::write(path, contents).expect("write security fixture");
}

fn hcl_language(marker: &str) -> Language {
    let language = Language::try_from("hcl");
    assert!(
        language.is_ok(),
        "RED:{marker} secure HCL parser boundary is unavailable: {language:?}"
    );
    language.expect("asserted HCL support")
}

fn references(result: &engram::services::parsing::ParseResult) -> Vec<&str> {
    result
        .edges
        .iter()
        .filter_map(|edge| match edge {
            ExtractedEdge::References { target, .. } => Some(target.as_str()),
            _ => None,
        })
        .collect()
}

fn directory_entries(path: &Path) -> Vec<String> {
    let mut entries: Vec<String> = std::fs::read_dir(path)
        .expect("read side-effect directory")
        .map(|entry| {
            entry
                .expect("read side-effect entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    entries.sort();
    entries
}

#[tokio::test]
async fn oversized_hcl_reuses_the_existing_pre_parse_file_guard() {
    let workspace = tempfile::tempdir().expect("create isolated workspace");
    write_file(
        workspace.path(),
        "oversized.hcl",
        &format!("payload = \"{}\"\n", "x".repeat(256)),
    );
    let config = CodeGraphConfig {
        max_file_size_bytes: 32,
        supported_languages: vec!["hcl".to_owned()],
        ..CodeGraphConfig::default()
    };

    let result = code_graph::index_workspace(
        workspace.path(),
        &workspace.path().join(".engram"),
        "hcl-oversize-red",
        &config,
        true,
    )
    .await
    .expect("oversize policy must return a bounded index result");

    assert_eq!(result.files_parsed, 0);
    assert_eq!(result.oversized_files_skipped, 1);
    assert!(result.errors.is_empty());
    let _ = hcl_language("HCL_OVERSIZE_GUARD_PARSER_MISSING");
}

#[test]
fn deeply_nested_malformed_and_dynamic_inputs_are_bounded_and_conservative() {
    let language = hcl_language("HCL_BOUNDED_PARSE_MISSING");
    let nested = format!(
        "locals {{\n  nested = {}0{}\n}}\n",
        "[".repeat(256),
        "]".repeat(256)
    );

    let started = Instant::now();
    let nested_result = parse_source(&nested, language).expect("deep HCL input stays parseable");
    let traversal_result =
        parse_source(TRAVERSALS.source, language).expect("dynamic HCL input stays parseable");
    let malformed_result =
        parse_source(MALFORMED.source, language).expect("malformed HCL input stays bounded");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "bounded HCL fixtures exceeded the two-second unit budget"
    );

    assert!(references(&nested_result).is_empty());
    assert_eq!(references(&traversal_result), TRAVERSALS.references);
    assert!(
        malformed_result.symbols.is_empty() && malformed_result.edges.is_empty(),
        "malformed HCL must not fabricate graph output: {malformed_result:?}"
    );
}

#[tokio::test]
async fn ignored_and_outside_aliases_stay_contained_and_parsing_has_no_side_effects() {
    let workspace = tempfile::tempdir().expect("create contained workspace");
    let outside = tempfile::tempdir().expect("create outside control directory");
    write_file(workspace.path(), ".gitignore", "ignored.tf\n");
    write_file(
        workspace.path(),
        "ignored.tf",
        "resource \"ignored\" \"inside\" {}\n",
    );
    write_file(workspace.path(), "visible.hcl", "service \"visible\" {}\n");
    write_file(
        outside.path(),
        "outside.tfvars",
        "escaped = \"must-not-index\"\n",
    );

    let config = CodeGraphConfig {
        supported_languages: vec!["hcl".to_owned()],
        ..CodeGraphConfig::default()
    };
    let data_dir = workspace.path().join(".engram");
    let branch = "hcl-containment-red";
    let _ = code_graph::index_workspace(workspace.path(), &data_dir, branch, &config, true)
        .await
        .expect("contained index returns a bounded result");

    let db = connect_db(&data_dir, branch)
        .await
        .expect("connect graph DB");
    let files = CodeGraphQueries::new(db)
        .list_code_files()
        .await
        .expect("list contained files");
    assert!(
        files
            .iter()
            .all(|file| file.path != "ignored.tf" && !file.path.contains("outside.tfvars")),
        "ignored or outside HCL alias escaped containment: {files:?}"
    );

    let language = hcl_language("HCL_SIDE_EFFECT_BOUNDARY_MISSING");
    let side_effects = tempfile::tempdir().expect("create side-effect control directory");
    let sentinel = side_effects.path().join("sentinel.txt");
    std::fs::write(&sentinel, "unchanged").expect("write sentinel");
    let before_entries = directory_entries(side_effects.path());
    let before_environment: BTreeMap<_, _> = std::env::vars_os().collect();
    let source = format!(
        r#"
module "remote" {{
  source = "https://example.invalid/module.zip"
}}

resource "null_resource" "probe" {{
  input = file("{}")
  environment = env.SECRET_VALUE
  provisioner "local-exec" {{
    command = "write executed.txt"
  }}
}}
"#,
        sentinel.display().to_string().replace('\\', "/")
    );

    let _ = parse_source(&source, language).expect("parse side-effect-shaped HCL as syntax only");
    assert_eq!(
        std::fs::read_to_string(&sentinel).expect("read sentinel"),
        "unchanged"
    );
    assert_eq!(directory_entries(side_effects.path()), before_entries);
    assert_eq!(
        std::env::vars_os().collect::<BTreeMap<_, _>>(),
        before_environment
    );
}
