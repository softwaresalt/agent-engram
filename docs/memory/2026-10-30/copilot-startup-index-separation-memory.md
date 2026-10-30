---
title: Copilot startup index separation
date: 2026-10-30
feature: 143-F
task: 143.001-T
---

## Outcome

Both launchers now start Copilot without invoking Engram preflight or index
maintenance. Actual PowerShell and Bash `--version` invocations exited with
code 0 and printed GitHub Copilot CLI 1.0.105. The generation-directory count
remained 7 after PowerShell startup.

## Decisions and scope

* The operator rejected full index rebuilding during session startup.
  Remove the startup gate rather than extend its deadline.
* Preserve Copilot configuration, workspace anchoring, executable resolution,
  arguments, and exit status. Preserve existing backlog synchronization.
* Leave standalone preflight and daemon/background incremental updates
  unchanged. Use explicit `sync --direct` maintenance without a running daemon.
* Reuse the existing hash fast path keyed by complete normalized
  workspace-relative path. Do not introduce a second cache.
* Required parser/package-topology invalidation and explicit forced indexing
  remain valid exceptions to unchanged-content skipping.

## Changed surfaces

* Both root launchers and their three contract-test files
* Code-graph integration coverage for path-keyed hash reuse, identical
  basenames in different folders, a one-file modification, and a move
* Troubleshooting, operator glossary, and the
  [requirements](../../research/2026-10-30-session-start-index-separation-requirements.md)
* Generated probe/cache ignore rules needed for Cargo file discovery
* Backlog feature 143-F, task 143.001-T, and continuity records

No production indexer changes, dependencies, or final Cargo profile changes
were needed.

## Verification

* `cargo fmt --all -- --check`: passed
* Strict all-target Clippy using `target\validation`: passed
* Five launcher contracts and all 43 code-graph integrations: passed
* `cargo build --target-dir target\validation`: passed
* Actual PowerShell and Bash launchers: passed
* `cargo audit`: eight existing advisories; tracked separately as stash
  25A1BE9E. Dependencies were not changed.
* The full repository test suite was not run; verification covered the affected
  launcher contracts and complete code-graph integration target.

The test-first launcher contracts failed against the preflight launchers.
The corrected Windows workspace assertion canonicalizes paths to handle
short TEMP aliases. A new test-fixture Clippy warning was fixed before the
successful final checks.

## Historical failure evidence

The earlier forced-generation timeout workaround failed three actual startup
attempts. The
[circuit-breaker record](../2026-10-29/circuit-break-copilot-preflight.md)
remains authoritative for that failed operation. It was not retried.
The [compacted diagnostics](../compacted/2026-10-30-143-f-preflight-diagnostics-compacted.md)
describe the superseded timeout approach and link its archived original.

Do not inspect candidate SQLite databases to poll progress while a builder is
writing; even read-only probes can contend with the writer.

## Remaining work

Dependency advisory remediation is separate work. No startup or hash-reuse
implementation remains open. Standalone full-generation preflight reliability
is not claimed resolved by the launcher change.

## Commit provenance

Implementation commit: `984e7df9062f6903baa5120c25a94859e181c168`.
The official completion transitions archived both work items automatically.
Task 143.001-T records this commit through the supported backlog update
operation. No PR or merge was requested.
