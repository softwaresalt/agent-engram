//! Contract tests for packaged-archive smoke verification in the Release workflow.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn repository_file(relative_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn run_changelog_extractor(changelog: &Path, tag: &str) -> Output {
    Command::new("python")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/extract-changelog-section.py"))
        .args(["--changelog"])
        .arg(changelog)
        .args(["--tag", tag])
        .output()
        .unwrap_or_else(|error| panic!("failed to run changelog extractor: {error}"))
}

fn assert_python_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_marker_count(content: &str, marker: &str, expected: usize) {
    assert_eq!(
        content.matches(marker).count(),
        expected,
        "unexpected count for marker: {marker}"
    );
}

fn dedent(source: &str) -> String {
    let source = source.trim_matches('\n');
    let indentation = source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start_matches(' ').len())
        .min()
        .unwrap_or(0);
    source
        .lines()
        .map(|line| line.get(indentation..).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

fn native_release_target() -> Option<(&'static str, &'static str, &'static str)> {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some(("x86_64-pc-windows-msvc", "engram.exe", ".zip"))
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some(("x86_64-unknown-linux-gnu", "engram", ".tar.gz"))
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some(("aarch64-apple-darwin", "engram", ".tar.gz"))
    } else {
        None
    }
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
fn changelog_extractor_selects_the_exact_dated_heading() {
    let temporary = TempDir::new().expect("create temporary changelog directory");
    let changelog = temporary.path().join("CHANGELOG.md");
    fs::write(
        &changelog,
        dedent(
            r"
    # Changelog

    ## [1.2.3-rc.1] - 2026-08-29

    Prerelease body.

    ## [1.2.3] - 2026-08-30

    Stable body.

    ### Fixed

    * Exact dated heading selected

    ## [1.2.30] - 2026-09-01

    Wrong prefix-like body.
    ",
        ),
    )
    .expect("write generated changelog");

    let output = run_changelog_extractor(&changelog, "v1.2.3");

    assert_python_success(&output, "exact changelog extraction");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
        "Stable body.\n\n### Fixed\n\n* Exact dated heading selected\n"
    );
}

#[test]
fn changelog_extractor_rejects_wrong_and_missing_sections() {
    let temporary = TempDir::new().expect("create temporary changelog directory");
    let wrong = temporary.path().join("wrong-version.md");
    fs::write(&wrong, "## [1.2.30] - 2026-08-30\n\nA different release.\n")
        .expect("write wrong-version changelog");
    let missing = temporary.path().join("missing-section.md");
    fs::write(&missing, "# Changelog\n\nNo release sections yet.\n")
        .expect("write section-free changelog");

    for changelog in [&wrong, &missing] {
        let output = run_changelog_extractor(changelog, "v1.2.3");
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert_eq!(output.status.code(), Some(2), "unexpected stderr: {stderr}");
        assert!(
            stderr.contains("section not found for tag: v1.2.3"),
            "missing section did not fail closed: {stderr}"
        );
    }
}

#[test]
fn changelog_extractor_rejects_an_empty_section() {
    let temporary = TempDir::new().expect("create temporary changelog directory");
    let changelog = temporary.path().join("CHANGELOG.md");
    fs::write(
        &changelog,
        dedent(
            r"
    ## [1.2.3] - 2026-08-30

    ## [1.2.2] - 2026-08-29

    Previous release.
    ",
        ),
    )
    .expect("write generated changelog");

    let output = run_changelog_extractor(&changelog, "v1.2.3");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2), "unexpected stderr: {stderr}");
    assert!(
        stderr.contains("section is empty for tag: v1.2.3"),
        "empty section did not fail closed: {stderr}"
    );
}

#[test]
fn archive_verifier_extracts_valid_tar_and_zip_layouts() {
    let temporary = TempDir::new().expect("create temporary archive directory");
    let verifier = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/verify-release-archive.py");
    let assertion = r#"
    import io
    from pathlib import Path
    import runpy
    import sys
    import tarfile
    import zipfile

    module = runpy.run_path(sys.argv[1])
    extract_archive = module["extract_archive"]
    base = Path(sys.argv[2])
    tar_path = base / "valid.tar.gz"
    zip_path = base / "valid.zip"
    required = {
        "engram": b"native binary",
        "README.md": b"readme",
        "LICENSE": b"license",
    }

    with tarfile.open(tar_path, "w:gz") as archive:
        for name, content in required.items():
            member = tarfile.TarInfo(name)
            member.size = len(content)
            member.mode = 0o755 if name == "engram" else 0o644
            archive.addfile(member, io.BytesIO(content))

    with zipfile.ZipFile(zip_path, "w") as archive:
        archive.writestr("engram.exe", b"native binary")
        archive.writestr("README.md", b"readme")
        archive.writestr("LICENSE", b"license")

    tar_root = base / "tar-root"
    zip_root = base / "zip-root"
    extract_archive(tar_path, tar_root)
    extract_archive(zip_path, zip_root)
    assert (tar_root / "engram").read_bytes() == b"native binary"
    assert (tar_root / "README.md").read_bytes() == b"readme"
    assert (tar_root / "LICENSE").read_bytes() == b"license"
    assert (zip_root / "engram.exe").read_bytes() == b"native binary"
    assert (zip_root / "README.md").read_bytes() == b"readme"
    assert (zip_root / "LICENSE").read_bytes() == b"license"
    "#;
    let output = Command::new("python")
        .arg("-c")
        .arg(dedent(assertion))
        .arg(verifier)
        .arg(temporary.path())
        .output()
        .unwrap_or_else(|error| panic!("failed to exercise valid archive extraction: {error}"));

    assert_python_success(&output, "valid tar.gz and zip extraction");
}

#[test]
fn archive_verifier_rejects_traversal_links_and_unsupported_members() {
    let temporary = TempDir::new().expect("create temporary archive directory");
    let verifier = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/verify-release-archive.py");
    let assertion = r#"
    import io
    from pathlib import Path
    import runpy
    import stat
    import sys
    import tarfile
    import zipfile

    module = runpy.run_path(sys.argv[1])
    extract_archive = module["extract_archive"]
    SmokeFailure = module["SmokeFailure"]
    base = Path(sys.argv[2])

    def reject(archive, root, message):
        try:
            extract_archive(archive, root)
        except SmokeFailure as error:
            assert message in str(error), str(error)
        else:
            raise AssertionError(f"unsafe archive was accepted: {archive}")

    tar_traversal = base / "tar-traversal.tar.gz"
    with tarfile.open(tar_traversal, "w:gz") as archive:
        content = b"escape"
        member = tarfile.TarInfo("../escaped-from-tar")
        member.size = len(content)
        archive.addfile(member, io.BytesIO(content))
    reject(tar_traversal, base / "tar-traversal-root", "unsafe archive member")
    assert not (base / "escaped-from-tar").exists()

    zip_traversal = base / "zip-traversal.zip"
    with zipfile.ZipFile(zip_traversal, "w") as archive:
        archive.writestr("../escaped-from-zip", b"escape")
    reject(zip_traversal, base / "zip-traversal-root", "unsafe archive member")
    assert not (base / "escaped-from-zip").exists()

    tar_link = base / "tar-link.tar.gz"
    with tarfile.open(tar_link, "w:gz") as archive:
        member = tarfile.TarInfo("engram-link")
        member.type = tarfile.SYMTYPE
        member.linkname = "engram"
        archive.addfile(member)
    reject(tar_link, base / "tar-link-root", "unsupported tar member type")

    zip_link = base / "zip-link.zip"
    with zipfile.ZipFile(zip_link, "w") as archive:
        member = zipfile.ZipInfo("engram-link")
        member.create_system = 3
        member.external_attr = (stat.S_IFLNK | 0o777) << 16
        archive.writestr(member, "engram")
    reject(zip_link, base / "zip-link-root", "symbolic links are not allowed")

    zip_fifo = base / "zip-fifo.zip"
    with zipfile.ZipFile(zip_fifo, "w") as archive:
        member = zipfile.ZipInfo("named-pipe")
        member.create_system = 3
        member.external_attr = (stat.S_IFIFO | 0o644) << 16
        archive.writestr(member, b"")
    reject(zip_fifo, base / "zip-fifo-root", "unsupported zip member type")
    "#;
    let output = Command::new("python")
        .arg("-c")
        .arg(dedent(assertion))
        .arg(verifier)
        .arg(temporary.path())
        .output()
        .unwrap_or_else(|error| panic!("failed to exercise unsafe archive rejection: {error}"));

    assert_python_success(&output, "unsafe archive rejection");
}

#[test]
fn archive_verifier_resolves_a_relative_work_dir_once_for_all_checks() {
    let temporary = TempDir::new().expect("create temporary archive directory");
    let verifier = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/verify-release-archive.py");
    let assertion = r#"
    import os
    from pathlib import Path
    import runpy
    import sys

    module = runpy.run_path(sys.argv[1])
    main = module["main"]
    base = Path(sys.argv[2]).resolve()
    invocation_cwd = base / "invocation-cwd"
    invocation_cwd.mkdir()
    archive = base / "engram-v1.2.3-x86_64-pc-windows-msvc.zip"
    relative_work_dir = Path("relative-work") / "unpacked"
    expected_work_dir = (invocation_cwd / relative_work_dir).resolve()
    mcp_checks = []

    def extract_archive(archive_path, work_dir):
        assert archive_path.is_absolute(), archive_path
        assert work_dir == expected_work_dir, work_dir
        work_dir.mkdir(parents=True)
        (work_dir / "engram.exe").write_bytes(b"binary")
        (work_dir / "README.md").write_text("readme", encoding="utf-8")
        (work_dir / "LICENSE").write_text("license", encoding="utf-8")

    def run_cli(binary, argument):
        assert binary.is_absolute(), binary
        assert binary.parent == expected_work_dir, binary
        return "engram 1.2.3" if argument == "--version" else ""

    def verify_mcp_stdio(binary, work_dir):
        assert binary.is_absolute(), binary
        assert binary.parent == expected_work_dir, binary
        assert work_dir == expected_work_dir, work_dir
        mcp_checks.append((binary, work_dir))

    main.__globals__["extract_archive"] = extract_archive
    main.__globals__["run_cli"] = run_cli
    main.__globals__["verify_mcp_stdio"] = verify_mcp_stdio
    os.chdir(invocation_cwd)
    sys.argv = [
        sys.argv[1],
        "--archive", str(archive),
        "--tag", "v1.2.3",
        "--target", "x86_64-pc-windows-msvc",
        "--work-dir", str(relative_work_dir),
        "--mcp",
    ]
    assert main() == 0
    assert len(mcp_checks) == 1
    "#;
    let output = Command::new("python")
        .arg("-c")
        .arg(dedent(assertion))
        .arg(verifier)
        .arg(temporary.path())
        .output()
        .unwrap_or_else(|error| panic!("failed to exercise relative work directory: {error}"));

    assert_python_success(&output, "relative work directory verification");
}

#[test]
fn archive_verifier_runs_the_unpacked_native_binary() {
    let Some((target, binary_name, suffix)) = native_release_target() else {
        eprintln!("native archive smoke is not supported on this build host");
        return;
    };
    let verify_mcp = matches!(
        target,
        "x86_64-unknown-linux-gnu" | "x86_64-pc-windows-msvc" | "aarch64-apple-darwin"
    );
    let temporary = TempDir::new().expect("create temporary archive directory");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let verifier = root.join("scripts/verify-release-archive.py");
    let tag = format!("v{}", env!("CARGO_PKG_VERSION"));
    let invocation_cwd = temporary.path().join("invocation-cwd");
    fs::create_dir(&invocation_cwd).expect("create verifier invocation directory");
    let archive = temporary
        .path()
        .join(format!("engram-{tag}-{target}{suffix}"));
    let fixture_builder = r#"
    from pathlib import Path
    import sys
    import tarfile
    import zipfile

    archive_path = Path(sys.argv[1])
    binary = Path(sys.argv[2])
    readme = Path(sys.argv[3])
    license_file = Path(sys.argv[4])
    binary_name = sys.argv[5]
    if archive_path.suffix == ".zip":
        with zipfile.ZipFile(archive_path, "w") as archive:
            archive.write(binary, binary_name)
            archive.write(readme, "README.md")
            archive.write(license_file, "LICENSE")
    else:
        with tarfile.open(archive_path, "w:gz") as archive:
            archive.add(binary, binary_name)
            archive.add(readme, "README.md")
            archive.add(license_file, "LICENSE")
    "#;
    let fixture_output = Command::new("python")
        .arg("-c")
        .arg(dedent(fixture_builder))
        .arg(&archive)
        .arg(env!("CARGO_BIN_EXE_engram"))
        .arg(root.join("README.md"))
        .arg(root.join("LICENSE"))
        .arg(binary_name)
        .output()
        .unwrap_or_else(|error| panic!("failed to build native archive fixture: {error}"));
    assert_python_success(&fixture_output, "native archive fixture creation");

    let mut verifier_command = Command::new("python");
    verifier_command
        .arg(verifier)
        .args(["--archive"])
        .arg(&archive)
        .args(["--tag", &tag, "--target", target, "--work-dir"])
        .arg(Path::new("relative-work").join("unpacked"))
        .current_dir(&invocation_cwd);
    if verify_mcp {
        verifier_command.arg("--mcp");
    }
    let output = verifier_command
        .output()
        .unwrap_or_else(|error| panic!("failed to run native archive verifier: {error}"));

    assert_python_success(&output, "native archive smoke");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("ARCHIVE_TARGET={target}")));
    assert!(stdout.contains(&format!(
        "ARCHIVE_VERSION_OUTPUT=engram {}",
        env!("CARGO_PKG_VERSION")
    )));
    if verify_mcp {
        assert!(stdout.contains("MCP_PROTOCOL_VERSION="));
        let tool_count = stdout
            .lines()
            .find_map(|line| line.strip_prefix("MCP_TOOL_COUNT="))
            .and_then(|count| count.parse::<usize>().ok());
        assert!(
            tool_count.is_some_and(|count| count > 0),
            "MCP tool count evidence is missing or zero: {stdout}"
        );
        assert!(stdout.contains("MCP_STDIN_CLOSE_EXIT=10"));
    } else {
        assert!(!stdout.contains("MCP_PROTOCOL_VERSION="));
        assert!(!stdout.contains("MCP_TOOL_COUNT="));
        assert!(!stdout.contains("MCP_STDIN_CLOSE_EXIT="));
    }
    assert!(stdout.contains("ARCHIVE_SMOKE=PASS"));
}

#[cfg(unix)]
#[test]
fn archive_verifier_rejects_a_non_executable_packaged_binary_before_invocation() {
    let temporary = TempDir::new().expect("create temporary archive directory");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let verifier = root.join("scripts/verify-release-archive.py");
    let archive = temporary
        .path()
        .join("engram-v1.2.3-x86_64-unknown-linux-gnu.tar.gz");
    let marker = temporary.path().join("binary-was-invoked");
    let fixture_builder = r#"
    import io
    from pathlib import Path
    import sys
    import tarfile

    archive_path = Path(sys.argv[1])
    binary = b'''#!/bin/sh
    printf 'invoked\n' >> "$ENGRAM_NONEXEC_MARKER"
    if [ "$1" = "--version" ]; then
        printf 'engram 1.2.3\n'
    fi
    exit 0
    '''
    files = {
        "engram": (binary, 0o644),
        "README.md": (b"readme", 0o644),
        "LICENSE": (b"license", 0o644),
    }
    with tarfile.open(archive_path, "w:gz") as archive:
        for name, (content, mode) in files.items():
            member = tarfile.TarInfo(name)
            member.size = len(content)
            member.mode = mode
            archive.addfile(member, io.BytesIO(content))
    "#;
    let fixture_output = Command::new("python")
        .arg("-c")
        .arg(dedent(fixture_builder))
        .arg(&archive)
        .output()
        .unwrap_or_else(|error| panic!("failed to build non-executable archive fixture: {error}"));
    assert_python_success(&fixture_output, "non-executable archive fixture creation");

    let output = Command::new("python")
        .arg(verifier)
        .args(["--archive"])
        .arg(&archive)
        .args([
            "--tag",
            "v1.2.3",
            "--target",
            "x86_64-unknown-linux-gnu",
            "--work-dir",
        ])
        .arg(temporary.path().join("unpacked"))
        .env("ENGRAM_NONEXEC_MARKER", &marker)
        .output()
        .unwrap_or_else(|error| panic!("failed to run archive verifier: {error}"));
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2), "unexpected stderr: {stderr}");
    assert!(
        stderr.contains("packaged binary is not executable"),
        "non-executable packaged binary was not rejected: {stderr}"
    );
    assert!(
        !marker.exists(),
        "non-executable packaged binary was invoked before rejection"
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

#[test]
fn published_release_verification_workflow_is_native_read_only_and_fail_closed() {
    let workflow =
        repository_file(".github/workflows/verify-release-assets.yml").replace("\r\n", "\n");

    for required in [
        "workflow_dispatch:",
        "tag:",
        "expected_commit:",
        "required: true",
        "permissions:\n  contents: read",
        "concurrency:",
        "timeout-minutes:",
        "fail-fast: false",
        "outputs:",
        "release_id:",
        "tag must be a complete SemVer value beginning with v",
        "expected_commit must be exactly 40 hexadecimal characters",
        "refs/tags/$($env:TAG)^{commit}",
        "$tagCommit -cne $expectedCommit",
        "$headCommit -cne $expectedCommit",
        "/releases/tags/$encodedTag",
        "$release.draft -ne $false",
        "persist-credentials: false",
        "fetch-depth: 0",
        "actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5",
        "RUNNER_ARCH",
        "$matchingAssets.Count -ne 1",
        "$asset = Invoke-RestMethod -Uri $assetUri -Headers $headers",
        "^sha256:[0-9a-f]{64}$",
        "$actualSize -ne $expectedSize",
        "$actualDigest -cne $apiDigest.Substring(7)",
        "Get-FileHash",
        "Get-Item",
        "scripts/verify-release-archive.py",
        "--work-dir",
        "--mcp",
    ] {
        assert!(
            workflow.contains(required),
            "published-release workflow is missing required marker: {required}"
        );
    }
    assert_marker_count(&workflow, "timeout-minutes:", 2);
    assert_marker_count(
        &workflow,
        "actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5",
        2,
    );
    assert_marker_count(&workflow, "persist-credentials: false", 2);
    assert_marker_count(&workflow, "Authorization = \"Bearer $env:GH_TOKEN\"", 2);

    for native_leg in [
        "runner: ubuntu-latest\n            target: x86_64-unknown-linux-gnu\n            runner_arch: X64",
        "runner: windows-latest\n            target: x86_64-pc-windows-msvc\n            runner_arch: X64",
        "runner: macos-latest\n            target: aarch64-apple-darwin\n            runner_arch: ARM64",
    ] {
        assert!(
            workflow.contains(native_leg),
            "published-release workflow is missing native matrix leg: {native_leg}"
        );
    }

    let lowercase = workflow.to_ascii_lowercase();
    for forbidden in [
        "v0.3.0-rc.1",
        "contents: write",
        "releases: write",
        "actions/upload-artifact",
        "softprops/action-gh-release",
        "gh release create",
        "gh release upload",
        "gh release delete",
        "git push",
        "git tag ",
        "delete release",
        "upload release",
        "-method post",
        "-method put",
        "-method patch",
        "-method delete",
        "\npush:",
        "\npull_request:",
        "\nschedule:",
        "\nworkflow_run:",
    ] {
        assert!(
            !lowercase.contains(forbidden),
            "published-release workflow contains forbidden mutation marker: {forbidden}"
        );
    }
}
