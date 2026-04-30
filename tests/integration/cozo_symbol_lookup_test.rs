//! Integration tests for symbol lookup parity with the `SurrealDB` backend
//! (Task U3.6).
//!
//! Covers:
//!  - `find_symbols_by_name`
//!  - `resolve_symbol`
//!  - `list_symbols` with filters and pagination
//!  - `list_code_files`
//!  - `all_functions`, `all_classes`, `all_interfaces`
//!  - `get_class_by_name_ci`
//!
//! Requires the `cozo-backend` feature:
//!   `cargo test --no-default-features --features cozo-backend --test integration_cozo_symbol_lookup`

use tempfile::TempDir;

use engram::models::{Class, CodeFile, Function, Interface};

async fn make_db() -> (TempDir, engram::db::Db) {
    let tmp = TempDir::new().expect("tempdir");
    let db = engram::db::connect_db(tmp.path(), "test-sym")
        .await
        .expect("connect_db");
    (tmp, db)
}

fn make_fn(id: &str, name: &str, file_path: &str) -> Function {
    Function {
        id: id.to_owned(),
        name: name.to_owned(),
        file_path: file_path.to_owned(),
        line_start: 1,
        line_end: 5,
        signature: format!("fn {name}()"),
        docstring: None,
        body: String::new(),
        body_hash: format!("hash_{id}"),
        token_count: 0,
        embed_type: "explicit_code".to_owned(),
        summary: String::new(),
        embedding: vec![0.1_f32; 384],
    }
}

fn make_class(id: &str, name: &str, file_path: &str) -> Class {
    Class {
        id: id.to_owned(),
        name: name.to_owned(),
        file_path: file_path.to_owned(),
        line_start: 1,
        line_end: 10,
        docstring: None,
        body: String::new(),
        body_hash: format!("hash_{id}"),
        token_count: 0,
        embed_type: "explicit_code".to_owned(),
        summary: String::new(),
        embedding: vec![0.2_f32; 384],
    }
}

fn make_interface(id: &str, name: &str, file_path: &str) -> Interface {
    Interface {
        id: id.to_owned(),
        name: name.to_owned(),
        file_path: file_path.to_owned(),
        line_start: 1,
        line_end: 8,
        docstring: None,
        body: String::new(),
        body_hash: format!("hash_{id}"),
        token_count: 0,
        embed_type: "explicit_code".to_owned(),
        summary: String::new(),
        embedding: vec![0.3_f32; 384],
    }
}

fn make_file(id: &str, path: &str) -> CodeFile {
    CodeFile {
        id: id.to_owned(),
        path: path.to_owned(),
        language: "rust".to_owned(),
        size_bytes: 1024,
        content_hash: "abc".to_owned(),
        last_indexed_at: "2026-01-01T00:00:00Z".to_owned(),
    }
}

// ── find_symbols_by_name ──────────────────────────────────────────────────────

#[tokio::test]
async fn find_symbols_by_name_returns_matching_function() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_fn("function:fn_find_me", "find_me_fn", "src/a.rs"))
        .await
        .expect("upsert");

    let results = q
        .find_symbols_by_name("find_me_fn")
        .await
        .expect("find_symbols_by_name");

    assert!(
        !results.is_empty(),
        "should find the function with the given name"
    );
    assert!(
        results.iter().any(|s| s.name == "find_me_fn"),
        "result must include fn with name 'find_me_fn'"
    );
}

#[tokio::test]
async fn find_symbols_by_name_empty_when_no_match() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);

    let results = q
        .find_symbols_by_name("nonexistent_symbol_xyz")
        .await
        .expect("find_symbols_by_name");

    assert!(
        results.is_empty(),
        "should return empty vec when no symbol with that name exists"
    );
}

#[tokio::test]
async fn find_symbols_by_name_returns_class() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_class(&make_class("class:cls_named", "MyClass", "src/b.rs"))
        .await
        .expect("upsert class");

    let results = q
        .find_symbols_by_name("MyClass")
        .await
        .expect("find_symbols_by_name");

    assert!(
        results.iter().any(|s| s.name == "MyClass"),
        "should find class by name"
    );
}

// ── resolve_symbol ────────────────────────────────────────────────────────────

#[tokio::test]
async fn resolve_symbol_returns_function_by_id() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_fn("function:fn_resolve", "fn_resolve", "src/r.rs"))
        .await
        .expect("upsert");

    let resolved = q
        .resolve_symbol("function:fn_resolve")
        .await
        .expect("resolve_symbol");

    assert!(resolved.is_some(), "should resolve function by ID");
    assert_eq!(resolved.unwrap().name, "fn_resolve");
}

#[tokio::test]
async fn resolve_symbol_returns_none_for_missing_id() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);

    let resolved = q
        .resolve_symbol("function:nonexistent_9999")
        .await
        .expect("resolve_symbol");

    assert!(resolved.is_none(), "unknown ID must resolve to None");
}

// ── list_symbols ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_symbols_returns_all_types() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_fn("function:ls_fn", "ls_fn", "src/ls.rs"))
        .await
        .expect("upsert fn");
    q.upsert_class(&make_class("class:ls_cls", "LsCls", "src/ls.rs"))
        .await
        .expect("upsert class");
    q.upsert_interface(&make_interface(
        "interface:ls_iface",
        "LsIface",
        "src/ls.rs",
    ))
    .await
    .expect("upsert interface");

    let filter = engram::db::queries::SymbolFilter {
        file_path: None,
        node_type: None,
        name_prefix: None,
        limit: 100,
        offset: 0,
    };
    let result = q.list_symbols(&filter).await.expect("list_symbols");

    assert!(
        result.total_count >= 3,
        "total_count must count all inserted symbols; got {}",
        result.total_count
    );
    let names: std::collections::HashSet<&str> =
        result.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains("ls_fn"), "must include function");
    assert!(names.contains("LsCls"), "must include class");
    assert!(names.contains("LsIface"), "must include interface");
}

#[tokio::test]
async fn list_symbols_filters_by_node_type() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_fn("function:nt_fn", "nt_fn", "src/nt.rs"))
        .await
        .expect("upsert fn");
    q.upsert_class(&make_class("class:nt_cls", "NtCls", "src/nt.rs"))
        .await
        .expect("upsert cls");

    let filter = engram::db::queries::SymbolFilter {
        file_path: None,
        node_type: Some("function".to_owned()),
        name_prefix: None,
        limit: 100,
        offset: 0,
    };
    let result = q
        .list_symbols(&filter)
        .await
        .expect("list_symbols with type filter");

    assert!(
        result.symbols.iter().all(|s| s.node_type == "function"),
        "all results must be of type 'function' when filtered"
    );
}

#[tokio::test]
async fn list_symbols_filters_by_file_path() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_fn(
        "function:fp_fn_target",
        "fp_fn_target",
        "src/target.rs",
    ))
    .await
    .expect("upsert in target");
    q.upsert_function(&make_fn(
        "function:fp_fn_other",
        "fp_fn_other",
        "src/other.rs",
    ))
    .await
    .expect("upsert in other");

    let filter = engram::db::queries::SymbolFilter {
        file_path: Some("src/target.rs".to_owned()),
        node_type: None,
        name_prefix: None,
        limit: 100,
        offset: 0,
    };
    let result = q
        .list_symbols(&filter)
        .await
        .expect("list_symbols file filter");

    let names: Vec<&str> = result.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"fp_fn_target"),
        "target file symbol must appear"
    );
    assert!(
        !names.contains(&"fp_fn_other"),
        "other file symbol must NOT appear"
    );
}

// ── list_code_files ───────────────────────────────────────────────────────────

#[tokio::test]
async fn list_code_files_returns_inserted_file() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_code_file(&make_file("code_file:cf1", "src/cf1.rs"))
        .await
        .expect("upsert");

    let files = q.list_code_files().await.expect("list_code_files");
    assert!(
        files.iter().any(|f| f.path == "src/cf1.rs"),
        "inserted file must appear in list_code_files"
    );
}

// ── all_functions / all_classes / all_interfaces ──────────────────────────────

#[tokio::test]
async fn all_functions_returns_upserted_function() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_function(&make_fn("function:af_fn", "af_fn", "src/af.rs"))
        .await
        .expect("upsert");

    let fns = q.all_functions().await.expect("all_functions");
    assert!(
        fns.iter().any(|f| f.name == "af_fn"),
        "all_functions must return af_fn"
    );
}

#[tokio::test]
async fn all_classes_returns_upserted_class() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_class(&make_class("class:ac_cls", "AcClass", "src/ac.rs"))
        .await
        .expect("upsert");

    let clss = q.all_classes().await.expect("all_classes");
    assert!(
        clss.iter().any(|c| c.name == "AcClass"),
        "all_classes must return AcClass"
    );
}

#[tokio::test]
async fn all_interfaces_returns_upserted_interface() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_interface(&make_interface(
        "interface:ai_iface",
        "AiTrait",
        "src/ai.rs",
    ))
    .await
    .expect("upsert");

    let ifaces = q.all_interfaces().await.expect("all_interfaces");
    assert!(
        ifaces.iter().any(|i| i.name == "AiTrait"),
        "all_interfaces must return AiTrait"
    );
}

// ── get_class_by_name_ci ──────────────────────────────────────────────────────

#[tokio::test]
async fn get_class_by_name_ci_finds_class_case_insensitively() {
    let (_tmp, db) = make_db().await;
    let q = engram::db::queries::CodeGraphQueries::new(db);
    q.upsert_class(&make_class(
        "class:ci_cls",
        "CaseSensitiveClass",
        "src/ci.rs",
    ))
    .await
    .expect("upsert");

    let found_exact = q
        .get_class_by_name_ci("CaseSensitiveClass")
        .await
        .expect("ci exact");
    assert!(found_exact.is_some(), "exact case should match");

    let found_lower = q
        .get_class_by_name_ci("casesensitiveclass")
        .await
        .expect("ci lowercase");
    assert!(
        found_lower.is_some(),
        "lowercase should match case-insensitively"
    );
}
