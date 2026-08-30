//! Contract tests for packaged-archive smoke verification in the Release workflow.

use std::fs;
use std::path::Path;
use std::process::Command;

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
    for required in [
        "import secrets",
        "secrets.token_hex(32)",
        "if delimiter not in body.splitlines()",
    ] {
        assert!(
            changelog.contains(required),
            "the changelog job is missing collision-safe output marker: {required}"
        );
    }
    assert!(
        !changelog.contains("content<<CHANGELOG_EOF"),
        "the changelog job must not use a fixed multiline output delimiter"
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
fn archive_verifier_requires_exact_basename_and_semver_release_identity() {
    let verifier = repository_file("scripts/verify-release-archive.py");

    for required in [
        "expected_archive_name",
        "args.archive.name != expected_archive_name",
        "SEMVER_PATTERN.fullmatch",
        "reported_version = parse_semver",
        "expected_version = parse_semver",
        "release_identity(reported_version) != release_identity(expected_version)",
        "Build metadata is valid but does not change release identity.",
    ] {
        assert!(
            verifier.contains(required),
            "archive verifier is missing strict release identity marker: {required}"
        );
    }
    assert!(
        !verifier.contains("args.target not in args.archive.name")
            && !verifier.contains("expected_version not in version_output"),
        "archive verification must not accept substring matches"
    );
}

#[test]
fn archive_verifier_rejects_a_filename_that_only_contains_the_expected_name() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let verifier = root.join("scripts/verify-release-archive.py");
    let archive = root.join("engram-v0.3.0-rc.1-x86_64-unknown-linux-gnu.tar.gz.unexpected");
    let output = Command::new("python")
        .arg(verifier)
        .args(["--archive"])
        .arg(archive)
        .args([
            "--tag",
            "v0.3.0-rc.1",
            "--target",
            "x86_64-unknown-linux-gnu",
            "--work-dir",
        ])
        .arg(root.join("archive-name-contract-work"))
        .output()
        .unwrap_or_else(|error| panic!("failed to run archive verifier: {error}"));
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2), "unexpected stderr: {stderr}");
    assert!(
        stderr.contains("archive filename must be"),
        "substring-only archive name was not rejected first: {stderr}"
    );
}

#[test]
fn archive_verifier_distinguishes_prereleases_but_allows_reported_build_metadata() {
    let verifier = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/verify-release-archive.py");
    let assertion = r#"
import runpy
import sys

module = runpy.run_path(sys.argv[1])
parse_semver = module["parse_semver"]
release_identity = module["release_identity"]
stable = release_identity(parse_semver("0.3.0", "stable"))
prerelease = release_identity(parse_semver("0.3.0-rc.1", "prerelease"))
reported = release_identity(
    parse_semver("0.3.0-rc.1+g0123456789-dirty", "reported")
)
assert stable != prerelease
assert prerelease == reported
"#;
    let output = Command::new("python")
        .args(["-c", assertion])
        .arg(verifier)
        .output()
        .unwrap_or_else(|error| panic!("failed to run SemVer contract: {error}"));

    assert!(
        output.status.success(),
        "SemVer contract failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
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
