#!/usr/bin/env bash
# Independence guard for the agent-visible MCP catalog oracle (Feature 127-F).
#
# Shell mirror of scripts/check-oracle-independence.ps1. Enforces the oracle
# independence invariant mechanically so a future refactor cannot quietly
# reconnect the oracle to the production derivation path:
#
#   1. Forbidden-import scan — the oracle test and its capture helper (the Rust
#      sources that could `use` the production module) must NOT reference the
#      production catalog module or its enumeration function (forbidden tokens
#      'tools_catalog', 'all_tools'). The human-authored JSON fixture is data,
#      not code: it may name the source contract in its policy note, and its
#      independence is enforced by the regeneration scan below plus its header.
#   2. Fixture-regeneration scan — no build script, test, CI step, or helper
#      script may write the fixture file. Any line under build.rs,
#      .github/workflows, scripts, or tests that names the fixture AND uses a
#      write verb is a violation.
#
# Exit 0 = independent; exit 1 = a violation was detected (printed to stderr).
# The two scenarios are demonstrable by pointing --root at a throwaway tree
# containing a violating copy.
#
# Usage: scripts/check-oracle-independence.sh [--root <repo-root>]

set -euo pipefail

ROOT=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --root)
            ROOT="${2:-}"
            shift 2
            ;;
        *)
            shift
            ;;
    esac
done
if [ -z "$ROOT" ]; then
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
    ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
fi

FIXTURE_NAME="mcp_tool_catalog.expected.json"
WRITE_VERBS='fs::write|write_all|File::create|to_writer|Out-File|Set-Content|Add-Content|tee|>'
SELF="$(cd "$(dirname "$0")" && pwd)/$(basename "$0")"
VIOLATIONS=0

# 1. Forbidden-import / forbidden-token scan of the two Rust oracle artifacts.
scan_files="tests/contract/mcp_catalog_oracle_test.rs tests/helpers/mcp_catalog_capture.rs"
for rel in $scan_files; do
    file="$ROOT/$rel"
    [ -f "$file" ] || continue
    for token in "tools_catalog" "all_tools"; do
        hits="$(grep -nF "$token" "$file" 2>/dev/null || true)"
        if [ -n "$hits" ]; then
            echo "FORBIDDEN-IMPORT ($token) in $file:" >&2
            echo "$hits" | sed 's/^/  /' >&2
            VIOLATIONS=1
        fi
    done
done

# 2. Fixture-regeneration scan of build scripts, CI, helper scripts, and tests.
gather_targets() {
    for rel in build.rs .github/workflows scripts tests; do
        path="$ROOT/$rel"
        if [ -d "$path" ]; then
            find "$path" -type f
        elif [ -f "$path" ]; then
            echo "$path"
        fi
    done
}

regen_hits="$(gather_targets | while IFS= read -r file; do
    [ -n "$file" ] || continue
    [ "$file" = "$SELF" ] && continue
    grep -nF "$FIXTURE_NAME" "$file" 2>/dev/null | grep -E "$WRITE_VERBS" | sed "s#^#$file:#" || true
done)"
if [ -n "$regen_hits" ]; then
    echo "FIXTURE-REGENERATION detected:" >&2
    echo "$regen_hits" | sed 's/^/  /' >&2
    VIOLATIONS=1
fi

if [ "$VIOLATIONS" -ne 0 ]; then
    echo "Oracle independence guard: FAIL" >&2
    exit 1
fi

echo "Oracle independence guard: PASS (no forbidden imports, no fixture regeneration)."
exit 0
