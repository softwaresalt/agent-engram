#!/usr/bin/env python3
"""Verify a packaged Engram release archive without invoking Cargo."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import zipfile


SUPPORTED_TARGETS = {
    "x86_64-unknown-linux-gnu": "engram",
    "x86_64-pc-windows-msvc": "engram.exe",
    "aarch64-apple-darwin": "engram",
}
ARCHIVE_SUFFIXES = {
    "x86_64-unknown-linux-gnu": ".tar.gz",
    "x86_64-pc-windows-msvc": ".zip",
    "aarch64-apple-darwin": ".tar.gz",
}
EXECUTABLE_TARGETS = {
    "x86_64-unknown-linux-gnu",
    "aarch64-apple-darwin",
}
SEMVER_IDENTIFIER = r"(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
SEMVER_PATTERN = re.compile(
    rf"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    rf"(?:-({SEMVER_IDENTIFIER}(?:\.{SEMVER_IDENTIFIER})*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)


class SmokeFailure(RuntimeError):
    """A release archive failed a required smoke assertion."""


def parse_semver(value: str, source: str) -> tuple[int, int, int, str | None, str | None]:
    """Parse a complete SemVer value or fail with its source."""
    match = SEMVER_PATTERN.fullmatch(value)
    if match is None:
        raise SmokeFailure(f"{source} is not valid SemVer: {value!r}")
    major, minor, patch, prerelease, build = match.groups()
    return int(major), int(minor), int(patch), prerelease, build


def release_identity(
    version: tuple[int, int, int, str | None, str | None],
) -> tuple[int, int, int, str | None]:
    """Return fields that must identify the tagged release exactly."""
    # Build metadata is valid but does not change release identity.
    return version[:4]


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(
        description="Unpack and smoke an Engram release archive."
    )
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--target", required=True, choices=SUPPORTED_TARGETS)
    parser.add_argument("--work-dir", required=True, type=Path)
    parser.add_argument(
        "--mcp",
        action="store_true",
        help="also verify initialize, tools/list, and stdin-close behavior",
    )
    return parser.parse_args()


def checked_destination(root: Path, member_name: str) -> Path:
    """Resolve an archive member below ``root`` or reject it."""
    member = PurePosixPath(member_name.replace("\\", "/"))
    if member.is_absolute() or ".." in member.parts:
        raise SmokeFailure(f"unsafe archive member: {member_name}")

    relative_parts = [part for part in member.parts if part not in ("", ".")]
    destination = root.joinpath(*relative_parts).resolve()
    resolved_root = root.resolve()
    if destination != resolved_root and resolved_root not in destination.parents:
        raise SmokeFailure(f"archive member escapes work directory: {member_name}")
    return destination


def extract_tar(archive: Path, root: Path) -> None:
    """Extract regular files and directories from a tar archive."""
    with tarfile.open(archive, mode="r:gz") as source:
        for member in source.getmembers():
            destination = checked_destination(root, member.name)
            if member.isdir():
                destination.mkdir(parents=True, exist_ok=True)
                continue
            if not member.isfile():
                raise SmokeFailure(f"unsupported tar member type: {member.name}")

            destination.parent.mkdir(parents=True, exist_ok=True)
            extracted = source.extractfile(member)
            if extracted is None:
                raise SmokeFailure(f"cannot read tar member: {member.name}")
            with extracted, destination.open("wb") as output:
                shutil.copyfileobj(extracted, output)
            destination.chmod(member.mode & 0o777)


def extract_zip(archive: Path, root: Path) -> None:
    """Extract regular files and directories from a zip archive."""
    with zipfile.ZipFile(archive) as source:
        for member in source.infolist():
            destination = checked_destination(root, member.filename)
            unix_mode = member.external_attr >> 16
            member_type = stat.S_IFMT(unix_mode)
            if member.is_dir():
                if member_type not in (0, stat.S_IFDIR):
                    raise SmokeFailure(
                        f"unsupported zip member type: {member.filename}"
                    )
                destination.mkdir(parents=True, exist_ok=True)
                continue
            if member_type == stat.S_IFLNK:
                raise SmokeFailure(f"symbolic links are not allowed: {member.filename}")
            if member_type not in (0, stat.S_IFREG):
                raise SmokeFailure(f"unsupported zip member type: {member.filename}")

            destination.parent.mkdir(parents=True, exist_ok=True)
            with source.open(member) as extracted, destination.open("wb") as output:
                shutil.copyfileobj(extracted, output)


def extract_archive(archive: Path, root: Path) -> None:
    """Extract the supported release archive format."""
    if not archive.is_file():
        raise SmokeFailure(f"archive does not exist: {archive}")
    if root.exists():
        raise SmokeFailure(f"work directory already exists: {root}")
    root.mkdir(parents=True)

    if archive.name.endswith(".tar.gz"):
        extract_tar(archive, root)
    elif archive.suffix == ".zip":
        extract_zip(archive, root)
    else:
        raise SmokeFailure(f"unsupported archive format: {archive.name}")


def run_cli(binary: Path, argument: str) -> str:
    """Run one bounded CLI assertion against the unpacked binary."""
    result = subprocess.run(
        [str(binary), argument],
        cwd=binary.parent,
        check=False,
        capture_output=True,
        text=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise SmokeFailure(
            f"{binary.name} {argument} exited {result.returncode}: "
            f"{result.stderr.strip()}"
        )
    return result.stdout.strip()


def response_with_id(responses: list[dict[str, object]], request_id: int) -> dict:
    """Return the response matching a JSON-RPC request ID."""
    for response in responses:
        if response.get("id") == request_id:
            return response
    raise SmokeFailure(f"missing JSON-RPC response id {request_id}")


def verify_mcp_stdio(binary: Path, work_dir: Path) -> None:
    """Verify serve-first MCP behavior without a readiness-dependent gate."""
    missing_workspace = work_dir / "intentionally-missing-workspace"
    requests = [
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {
                    "name": "engram-release-archive-smoke",
                    "version": "1.0",
                },
            },
        },
        {
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {},
        },
        {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}},
    ]
    payload = "".join(f"{json.dumps(request)}\n" for request in requests)
    environment = os.environ.copy()
    environment.pop("ENGRAM_DATA_DIR", None)

    process = subprocess.Popen(
        [str(binary), "shim", "--workspace", str(missing_workspace)],
        cwd=binary.parent,
        env=environment,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        stdout, stderr = process.communicate(payload, timeout=45)
    except subprocess.TimeoutExpired as error:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
        raise SmokeFailure("MCP stdio process hung after stdin closed") from error

    if process.returncode is None or process.returncode < 0:
        raise SmokeFailure(f"MCP stdio process terminated abnormally: {process.returncode}")
    if process.returncode != 10:
        raise SmokeFailure(
            "MCP stdio smoke expected classified admission exit 10 for the "
            f"intentionally missing workspace, got {process.returncode}"
        )
    if "panicked at" in stderr or "stack backtrace:" in stderr:
        raise SmokeFailure(f"MCP stdio process panicked: {stderr.strip()}")

    responses: list[dict[str, object]] = []
    for line in stdout.splitlines():
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as error:
            raise SmokeFailure(f"non-JSON stdout from MCP stdio: {line}") from error
        if isinstance(value, dict):
            responses.append(value)

    initialize = response_with_id(responses, 1)
    initialize_result = initialize.get("result")
    if not isinstance(initialize_result, dict):
        raise SmokeFailure(f"initialize did not return a result: {initialize}")
    server_info = initialize_result.get("serverInfo")
    protocol_version = initialize_result.get("protocolVersion")
    if not isinstance(server_info, dict) or not isinstance(protocol_version, str):
        raise SmokeFailure("initialize result lacks serverInfo or protocolVersion")

    tools_response = response_with_id(responses, 2)
    tools_result = tools_response.get("result")
    tools = tools_result.get("tools") if isinstance(tools_result, dict) else None
    if not isinstance(tools, list) or not tools:
        raise SmokeFailure("tools/list did not return a non-empty tools array")

    print(f"MCP_PROTOCOL_VERSION={protocol_version}")
    print(f"MCP_TOOL_COUNT={len(tools)}")
    print(f"MCP_STDIN_CLOSE_EXIT={process.returncode}")


def main() -> int:
    """Run archive structure, CLI, and optional MCP stdio checks."""
    args = parse_args()
    work_dir = args.work_dir.resolve()
    tag = args.tag
    if not tag.startswith("v") or len(tag) == 1:
        raise SmokeFailure(f"tag must begin with v: {tag}")
    expected_version = parse_semver(tag[1:], "release tag")
    archive_suffix = ARCHIVE_SUFFIXES[args.target]
    expected_archive_name = f"engram-{tag}-{args.target}{archive_suffix}"
    if args.archive.name != expected_archive_name:
        raise SmokeFailure(
            f"archive filename must be {expected_archive_name!r}, "
            f"got {args.archive.name!r}"
        )

    extract_archive(args.archive.resolve(), work_dir)
    binary_name = SUPPORTED_TARGETS[args.target]
    required_paths = [
        work_dir / binary_name,
        work_dir / "README.md",
        work_dir / "LICENSE",
    ]
    missing = [str(path) for path in required_paths if not path.is_file()]
    if missing:
        raise SmokeFailure(f"archive is missing required files: {', '.join(missing)}")

    binary = required_paths[0]
    if args.target in EXECUTABLE_TARGETS and binary.stat().st_mode & (
        stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH
    ) == 0:
        raise SmokeFailure(f"packaged binary is not executable: {binary.name}")

    version_output = run_cli(binary, "--version")
    version_parts = version_output.split()
    if len(version_parts) != 2 or version_parts[0] != "engram":
        raise SmokeFailure(
            f"archive binary version output has unexpected format: {version_output!r}"
        )
    reported_version = parse_semver(version_parts[1], "archive binary version")
    if release_identity(reported_version) != release_identity(expected_version):
        raise SmokeFailure(
            f"archive binary version {version_parts[1]!r} does not match tag {tag!r}"
        )
    run_cli(binary, "--help")

    print(f"ARCHIVE_TARGET={args.target}")
    print(f"ARCHIVE_VERSION_OUTPUT={version_output}")
    if args.mcp:
        verify_mcp_stdio(binary, work_dir)
    print("ARCHIVE_SMOKE=PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, SmokeFailure, subprocess.SubprocessError) as error:
        print(f"ARCHIVE_SMOKE=FAIL: {error}", file=sys.stderr)
        raise SystemExit(2) from error
