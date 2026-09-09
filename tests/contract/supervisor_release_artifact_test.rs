//! Contract tests for supervisor release-artifact publication.

use std::fs;
use std::path::Path;

fn repository_file(relative_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

#[test]
fn release_workflow_builds_the_supervisor_binary() {
    let workflow = repository_file(".github/workflows/release.yml");
    let builds_supervisor = workflow.contains("--bin engram-indexer")
        || workflow.contains("-p engram-indexer")
        || workflow.contains(
            "cargo build --locked --release --target ${{ matrix.target }} -p engram-indexer",
        )
        || workflow.contains(
            "cross build --locked --release --target ${{ matrix.target }} -p engram-indexer",
        );

    assert!(
        builds_supervisor,
        "release workflow must build engram-indexer as a distinct release binary"
    );
}

#[test]
fn release_workflow_uploads_a_distinct_supervisor_artifact() {
    let workflow = repository_file(".github/workflows/release.yml");
    let stages_supervisor_archive = workflow.contains("engram-indexer-${TAG}-${{ matrix.target }}")
        || workflow.contains("engram-indexer-$tag-${{ matrix.target }}")
        || workflow.contains("engram-indexer-${{ matrix.target }}");
    let uploads_supervisor_artifact = workflow
        .contains("name: engram-indexer-${{ matrix.target }}")
        || workflow.contains("name: engram-indexer-");

    assert!(
        stages_supervisor_archive && uploads_supervisor_artifact,
        "release workflow must stage and upload a supervisor artifact separately from engram"
    );
}

#[test]
fn agent_release_archive_remains_agent_only() {
    let workflow = repository_file(".github/workflows/release.yml");

    assert!(
        !workflow.contains("engram-indexer${{ matrix.binary_suffix }}"),
        "the agent archive step must not bundle the supervisor binary"
    );
}
