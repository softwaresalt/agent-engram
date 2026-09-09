//! Contract tests for excluding the supervisor from the agent installer.

use std::fs;
use std::path::Path;

use engram::installer::INSTALLED_BINARIES;

fn repository_file(relative_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

#[test]
fn installer_binary_manifest_excludes_the_supervisor() {
    assert_eq!(
        INSTALLED_BINARIES,
        &[engram::APP_NAME],
        "the installer contract must expose only the agent binary"
    );
    assert!(
        !INSTALLED_BINARIES.contains(&"engram-indexer"),
        "engram-indexer must remain excluded from the installer surface"
    );
}

#[test]
fn installer_module_source_does_not_reference_the_supervisor_binary() {
    let source = repository_file("src/installer/mod.rs");
    assert!(
        !source.contains("engram-indexer"),
        "installer implementation must not reference or copy the supervisor binary"
    );
}
