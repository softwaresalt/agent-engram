#!/usr/bin/env python3
"""Print one version's curated CHANGELOG section for a release body."""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import sys


SECTION_HEADING = re.compile(r"^## \[([^\]]+)\](?:\s+-\s+.+)?$")


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(
        description="Extract a version-matched CHANGELOG section."
    )
    parser.add_argument("--changelog", required=True, type=Path)
    parser.add_argument("--tag", required=True)
    return parser.parse_args()


def extract_section(changelog: Path, tag: str) -> str:
    """Return the content below the heading that exactly matches ``tag``."""
    if not tag.startswith("v") or len(tag) == 1:
        raise ValueError(f"tag must begin with v: {tag}")
    version = tag[1:]
    lines = changelog.read_text(encoding="utf-8").splitlines()

    start = None
    for index, line in enumerate(lines):
        match = SECTION_HEADING.fullmatch(line)
        if match and match.group(1) == version:
            start = index + 1
            break
    if start is None:
        raise ValueError(f"section not found for tag: {tag}")

    end = len(lines)
    for index in range(start, len(lines)):
        if SECTION_HEADING.fullmatch(lines[index]):
            end = index
            break

    body = "\n".join(lines[start:end]).strip()
    if not body:
        raise ValueError(f"section is empty for tag: {tag}")
    return body


def main() -> int:
    """Extract the requested section to stdout."""
    args = parse_args()
    print(extract_section(args.changelog, args.tag))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError) as error:
        print(f"CHANGELOG_EXTRACTION_FAILED: {error}", file=sys.stderr)
        raise SystemExit(2) from error
