//! Unit tests for the `derive_edge_id` free function (U3.1).
//!
//! Verifies the deterministic edge-ID derivation used by the CozoDB edge
//! CRUD methods to ensure stable, canonical IDs across sessions.
//!
//! Requires the `cozo-backend` feature:
//!   `cargo test --no-default-features --features cozo-backend --test unit_cozo_edge_id`

use engram::db::queries::derive_edge_id;

#[test]
fn calls_edge_id_is_type_colon_parts_joined_by_pipe() {
    let id = derive_edge_id("calls", &["fn:abc", "fn:def"]);
    assert_eq!(id, "calls:fn:abc|fn:def");
}

#[test]
fn imports_edge_id_includes_import_path() {
    let id = derive_edge_id("imports", &["file:src", "file:lib", "std::collections"]);
    assert_eq!(id, "imports:file:src|file:lib|std::collections");
}

#[test]
fn defines_edge_id_encodes_table_and_symbol() {
    let id = derive_edge_id("defines", &["file:src", "function", "fn:my_fn"]);
    assert_eq!(id, "defines:file:src|function|fn:my_fn");
}

#[test]
fn inherits_from_edge_id_uses_child_and_parent() {
    let id = derive_edge_id("inherits_from", &["cls:child", "cls:parent"]);
    assert_eq!(id, "inherits_from:cls:child|cls:parent");
}

#[test]
fn concerns_edge_id_encodes_task_table_symbol() {
    let id = derive_edge_id("concerns", &["task:t1", "function", "fn:bar"]);
    assert_eq!(id, "concerns:task:t1|function|fn:bar");
}

#[test]
fn single_part_edge_id() {
    let id = derive_edge_id("test", &["only_part"]);
    assert_eq!(id, "test:only_part");
}

#[test]
fn derive_edge_id_is_deterministic() {
    let id1 = derive_edge_id("calls", &["fn:a", "fn:b"]);
    let id2 = derive_edge_id("calls", &["fn:a", "fn:b"]);
    assert_eq!(id1, id2, "same inputs must produce the same edge ID");
}
