//! Contract tests for packaged-archive smoke verification in the Release workflow.

use std::fs;
use std::path::Path;

fn repository_file(relative_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

#[test]
fn release_workflow_retains_the_cross_platform_packaging_contract() {
    let workflow = repository_file(".github/workflows/release.yml");

    assert!(
        workflow.contains("fail-fast: true")
            && workflow.contains("fail_on_unmatched_files: true")
            && workflow.contains("x86_64-unknown-linux-gnu")
            && workflow.contains("x86_64-pc-windows-msvc")
            && workflow.contains("aarch64-apple-darwin"),
        "the Release workflow must retain fail-closed packaging for all supported targets"
    );
}

#[test]
fn release_workflow_publishes_a_version_generic_changelog_section() {
    let workflow = repository_file(".github/workflows/release.yml");
    let changelog_job = workflow
        .find("  changelog:")
        .expect("Release workflow must retain the changelog job");
    let build_job = workflow
        .find("  build:")
        .expect("Release workflow must retain the build job");
    let changelog = &workflow[changelog_job..build_job];

    assert!(
        changelog.contains("scripts/extract-changelog-section.py")
            && changelog.contains("--tag \"${GITHUB_REF_NAME}\""),
        "the changelog job must select the curated section by generic tag"
    );
    assert!(
        !changelog.contains("git-cliff --latest"),
        "the published body must not come from the unconfigured raw git-cliff output"
    );
    assert!(
        !changelog.contains("0.3.0-rc.1"),
        "the changelog job must remain safe for later stable releases"
    );
}

#[test]
fn archive_verifier_is_version_generic_and_never_uses_cargo_run() {
    let verifier = repository_file("scripts/verify-release-archive.py");

    for required in [
        "--archive",
        "--tag",
        "--target",
        "--work-dir",
        "--mcp",
        "README.md",
        "LICENSE",
        "protocolVersion",
        "tools/list",
    ] {
        assert!(
            verifier.contains(required),
            "archive verifier is missing required contract marker: {required}"
        );
    }
    assert!(
        verifier.contains("tarfile") && verifier.contains("zipfile"),
        "archive verifier must support both release archive formats"
    );
    assert!(
        !verifier.contains("0.3.0-rc.1"),
        "archive verifier must remain version-generic for stable releases"
    );
    assert!(
        !verifier.contains("cargo run"),
        "version evidence must come from the unpacked archive binary"
    );
}

#[test]
fn changelog_extractor_is_version_generic_and_fail_closed() {
    let extractor = repository_file("scripts/extract-changelog-section.py");

    for required in [
        "--changelog",
        "--tag",
        "section not found",
        "section is empty",
        "must begin with v",
    ] {
        assert!(
            extractor.contains(required),
            "changelog extractor is missing contract marker: {required}"
        );
    }
    assert!(
        !extractor.contains("0.3.0-rc.1"),
        "changelog extraction must remain generic for stable releases"
    );
}

#[test]
fn verification_record_separates_g1_evidence_from_g3_artifact_proof() {
    let record = repository_file("docs/closure/2026-08-29-v0.3.0-rc.1-verification.md");

    for required in [
        "## G1 pre-merge verification",
        "## G3 post-publish verification",
        "Pre-tag static evidence",
        "Post-publish required",
        "scripts/verify-release-archive.py",
        "engram-v0.3.0-rc.1-x86_64-unknown-linux-gnu.tar.gz",
        "engram-v0.3.0-rc.1-x86_64-pc-windows-msvc.zip",
        "engram-v0.3.0-rc.1-aarch64-apple-darwin.tar.gz",
    ] {
        assert!(
            record.contains(required),
            "verification record is missing required boundary marker: {required}"
        );
    }
}
