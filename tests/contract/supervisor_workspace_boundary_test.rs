//! Contract tests for the supervisor workspace boundary.

use std::process::Command;

fn cargo_metadata() -> serde_json::Value {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("parse cargo metadata JSON")
}

fn package_by_name<'a>(metadata: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    metadata["packages"]
        .as_array()
        .expect("packages array")
        .iter()
        .find(|package| package["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("package {name} missing from cargo metadata"))
}

#[test]
fn engram_indexer_is_a_distinct_workspace_member() {
    let metadata = cargo_metadata();
    let workspace_members = metadata["workspace_members"]
        .as_array()
        .expect("workspace_members array");
    let engram = package_by_name(&metadata, "engram")["id"]
        .as_str()
        .expect("engram package id");
    let engram_indexer = package_by_name(&metadata, "engram-indexer")["id"]
        .as_str()
        .expect("engram-indexer package id");

    assert_ne!(
        engram, engram_indexer,
        "engram-indexer must remain a separate workspace package"
    );
    assert!(
        workspace_members
            .iter()
            .any(|member| member.as_str() == Some(engram_indexer)),
        "engram-indexer must appear as its own workspace member"
    );
}

#[test]
fn root_engram_package_does_not_declare_an_indexer_binary() {
    let metadata = cargo_metadata();
    let engram = package_by_name(&metadata, "engram");
    let exposes_indexer_bin = engram["targets"]
        .as_array()
        .expect("engram targets")
        .iter()
        .any(|target| {
            target["name"].as_str() == Some("engram-indexer")
                && target["kind"]
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("bin")))
        });

    assert!(
        !exposes_indexer_bin,
        "the root engram package must not expose an engram-indexer binary target"
    );
}

#[test]
fn engram_indexer_declares_its_own_binary_target() {
    let metadata = cargo_metadata();
    let engram_indexer = package_by_name(&metadata, "engram-indexer");
    let has_indexer_bin = engram_indexer["targets"]
        .as_array()
        .expect("engram-indexer targets")
        .iter()
        .any(|target| {
            target["name"].as_str() == Some("engram-indexer")
                && target["kind"]
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("bin")))
        });

    assert!(
        has_indexer_bin,
        "engram-indexer must own a distinct binary target named engram-indexer"
    );
}

#[test]
fn engram_indexer_exposes_a_library_target_for_boundary_harnesses() {
    let metadata = cargo_metadata();
    let engram_indexer = package_by_name(&metadata, "engram-indexer");
    let has_library_target = engram_indexer["targets"]
        .as_array()
        .expect("engram-indexer targets")
        .iter()
        .any(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("lib")))
        });

    assert!(
        has_library_target,
        "engram-indexer must expose a library boundary for supervisor harnesses"
    );
}
