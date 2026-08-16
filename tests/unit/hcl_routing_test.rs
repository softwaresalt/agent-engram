//! RED harness for canonical HCL-family routing parity (121.003-T).

use std::path::{Path, PathBuf};

use chrono::Utc;
use engram::daemon::debounce::{ServiceAction, adapt_event};
use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::models::{WatchEventKind, WatcherEvent};
use engram::services::code_graph;
use engram::services::parsing::Language;
use engram::services::retrieval_eval::language_of;

const HCL_ALIASES: [&str; 3] = ["infra/main.hcl", "infra/main.tf", "infra/main.tfvars"];

fn write_source(workspace: &Path, relative: &str, source: &str) {
    let path = workspace.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create HCL fixture directory");
    }
    std::fs::write(path, source).expect("write HCL routing fixture");
}

fn event(path: &str, kind: WatchEventKind) -> WatcherEvent {
    WatcherEvent {
        path: PathBuf::from(path),
        old_path: None,
        kind,
        timestamp: Utc::now(),
    }
}

fn assert_canonical_hcl_token(path: &str, marker: &str) {
    let token = language_of(path);
    assert_eq!(token, "hcl", "RED:{marker} {path} must route as hcl");

    let parsed = Language::try_from(token.as_str());
    assert!(
        parsed.is_ok(),
        "RED:HCL_ROUTED_IDENTITY_UNSUPPORTED routed token {token:?} is not parseable: {parsed:?}"
    );
    assert_eq!(parsed.expect("asserted routed language").as_str(), token);
}

#[test]
fn three_case_sensitive_aliases_share_only_the_hcl_language_identity() {
    for path in HCL_ALIASES {
        assert_canonical_hcl_token(path, "HCL_ALIAS_ROUTING_MISSING");
    }

    let language = Language::try_from("hcl");
    assert!(
        language.is_ok(),
        "RED:HCL_IDENTITY_MISSING canonical hcl language is unsupported: {language:?}"
    );
    assert_eq!(language.expect("asserted HCL language").as_str(), "hcl");
    assert!(Language::try_from("terraform").is_err());

    assert_eq!(language_of("infra/main.TF"), "TF");
    assert_eq!(language_of("infra/main.HCL"), "HCL");
    assert_eq!(language_of("infra/main.TFVARS"), "TFVARS");
}

#[tokio::test]
async fn default_startup_and_explicit_sync_persist_one_hcl_identity() {
    let config = CodeGraphConfig::default();
    assert!(
        config
            .supported_languages
            .iter()
            .any(|language| language == "hcl"),
        "RED:HCL_DEFAULT_ROUTING_MISSING default startup languages omit hcl: {:?}",
        config.supported_languages
    );

    let workspace = tempfile::tempdir().expect("create isolated routing workspace");
    write_source(workspace.path(), HCL_ALIASES[0], "service \"api\" {}\n");
    write_source(
        workspace.path(),
        HCL_ALIASES[1],
        "resource \"aws_instance\" \"web\" {}\n",
    );
    write_source(workspace.path(), HCL_ALIASES[2], "region = \"west\"\n");

    let data_dir = workspace.path().join(".engram");
    let branch = "hcl-routing-red";
    let indexed = code_graph::index_workspace(workspace.path(), &data_dir, branch, &config, true)
        .await
        .expect("startup-style index must return a result");
    assert_eq!(
        indexed.files_parsed, 3,
        "RED:HCL_STARTUP_ROUTING_MISSING errors={:?}",
        indexed.errors
    );

    let startup_db = connect_db(&data_dir, branch)
        .await
        .expect("connect startup graph DB");
    let startup_files = CodeGraphQueries::new(startup_db)
        .list_code_files()
        .await
        .expect("list startup HCL files");
    assert_eq!(startup_files.len(), 3);
    assert!(
        startup_files.iter().all(|file| file.language == "hcl"),
        "startup must persist only hcl identities: {startup_files:?}"
    );
    let mut startup_paths: Vec<&str> = startup_files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    startup_paths.sort_unstable();

    write_source(
        workspace.path(),
        HCL_ALIASES[1],
        "resource \"aws_instance\" \"web_v2\" {}\n",
    );
    let synced = code_graph::sync_workspace(workspace.path(), &data_dir, branch, &config)
        .await
        .expect("explicit HCL sync must return a result");
    assert_eq!(synced.files_modified, 1);
    assert!(
        synced.errors.is_empty(),
        "HCL sync errors: {:?}",
        synced.errors
    );

    let db = connect_db(&data_dir, branch)
        .await
        .expect("connect graph DB");
    let files = CodeGraphQueries::new(db)
        .list_code_files()
        .await
        .expect("list indexed HCL files");
    assert_eq!(files.len(), 3);
    let mut synced_paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    synced_paths.sort_unstable();
    assert_eq!(
        synced_paths, startup_paths,
        "explicit sync must retain startup file identity"
    );
    assert!(
        files.iter().all(|file| file.language == "hcl"),
        "startup and sync must persist only hcl identities: {files:?}"
    );
}

#[test]
fn live_created_and_modified_aliases_match_retrieval_routing() {
    for path in HCL_ALIASES {
        for kind in [WatchEventKind::Created, WatchEventKind::Modified] {
            assert_eq!(
                adapt_event(&event(path, kind)),
                ServiceAction::ReindexFile {
                    path: PathBuf::from(path)
                },
                "RED:HCL_LIVE_ROUTING_MISSING {path} must route to reindex"
            );
        }
        assert_canonical_hcl_token(path, "HCL_RETRIEVAL_ROUTING_MISSING");
        assert_eq!(
            adapt_event(&event(path, WatchEventKind::Deleted)),
            ServiceAction::Skip
        );
        assert_eq!(
            adapt_event(&event(path, WatchEventKind::Renamed)),
            ServiceAction::Skip
        );
    }

    assert_eq!(
        adapt_event(&event("infra/main.TF", WatchEventKind::Modified)),
        ServiceAction::Skip,
        "existing case-sensitive extension behavior must remain unchanged"
    );
}
