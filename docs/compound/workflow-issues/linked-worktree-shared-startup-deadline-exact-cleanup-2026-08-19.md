---
title: "Linked worktrees need one shared startup deadline and exact-process cleanup"
doc_type: learning
source: "118-S linked-worktree / bounded-startup shipment"
description: "Native linked worktrees must be admitted through validated Git metadata, and direct/fallback Engram pre-warm attempts must share one wall-clock budget while launcher cleanup terminates only the exact child it started."
problem_type: "startup_admission + bounded_timeout + exact_child_cleanup"
category: workflow-issues
component: "linked-worktree startup admission, Engram pre-warm budget, and launcher cleanup"
root_cause: "The startup path treated a valid linked worktree as if it needed ambient repository-root validation, and the pre-warm path relied on IPC-scoped timeout flags instead of one outer wall-clock deadline; cleanup also needed to target only the child process the launcher started."
resolution_type: "validated linked-worktree admission with one shared wall-clock budget and exact-process cleanup"
severity: high
message: "Linked worktree startup was rejected as not a Git repository root."
file_path: "src/db/workspace.rs"
citations:
  - "shipment: 118-S"
  - "pr: 344"
  - "merge_commit: 08676d341d94fd97b9d7ea3ea30562e63c5c9bba"
  - "https://github.com/softwaresalt/agent-engram/pull/344"
  - "src/db/workspace.rs"
  - "tests/integration/cli_direct_test.rs"
  - "tests/contract/shim_lifecycle_test.rs"
  - "tests/contract/start_launcher_test.rs"
  - "docs/closure/2026-08-19-118-s-worktree-mcp-startup-closure.md"
  - "docs/closure/2026-08-19-118-s-worktree-mcp-startup-runtime-verification.md"
  - "docs/closure/2026-05-07-027-S-startup-preloading-closure.md"
  - "docs/closure/2026-05-07-027-S-startup-preloading-runtime-verification.md"
tags:
  - "linked-worktree"
  - "startup"
  - "launcher"
  - "bounded-timeout"
  - "cleanup"
  - "fail-open"
  - "worktree"
  - "ship-agent"
  - "118-S"
  - "344"
date: 2026-08-19
confidence: high
evidence:
  - shipment: 118-S
  - pr: 344
  - merge_commit: 08676d341d94fd97b9d7ea3ea30562e63c5c9bba
  - https://github.com/softwaresalt/agent-engram/pull/344
  - src/db/workspace.rs
  - tests/integration/cli_direct_test.rs
  - tests/contract/shim_lifecycle_test.rs
  - tests/contract/start_launcher_test.rs
  - docs/closure/2026-08-19-118-s-worktree-mcp-startup-closure.md
  - docs/closure/2026-08-19-118-s-worktree-mcp-startup-runtime-verification.md
  - docs/closure/2026-05-07-027-S-startup-preloading-closure.md
  - docs/closure/2026-05-07-027-S-startup-preloading-runtime-verification.md
---

## Finding

The old "non-fatal exit code after a returned child process" approach is not a
startup safety net. A valid native linked worktree must be accepted on its own
Git metadata, and pre-warm attempts must be bounded by one outer wall-clock
deadline rather than assuming IPC-scoped timeout flags can bound the whole
indexing path.

## Evidence

| Artifact | What it proves |
|---|---|
| `src/db/workspace.rs` | Linked-worktree `.git` validation fails closed instead of falling back to ambient paths. |
| `tests/integration/cli_direct_test.rs` | A real linked worktree is admitted and keeps its active branch identity. |
| `tests/contract/shim_lifecycle_test.rs` | The shim keeps daemon lifecycle bounded, reusable, and cleanup-safe. |
| `tests/contract/start_launcher_test.rs` | Launcher pre-warm shares one wall-clock budget and cleans up only the exact process it started. |
| `docs/closure/2026-08-19-118-s-worktree-mcp-startup-closure.md` | Current operational closure for PR #344 and shipment 118-S. |
| `docs/closure/2026-08-19-118-s-worktree-mcp-startup-runtime-verification.md` | Runtime proof for linked-worktree admission and bounded startup. |
| `docs/closure/2026-05-07-027-S-startup-preloading-closure.md` | Historical counterexample showing why non-fatal exit-code checks alone were insufficient. |

## Guardrails

1. Validate native linked-worktree metadata before any expensive startup path.
2. Share one wall-clock budget across direct and fallback Engram pre-warm.
3. Terminate only the exact child process the launcher started.
4. Treat fail-open to Copilot as a requirement, not a best-effort fallback.
5. Keep startup-order checks and linked-worktree identity covered by tests.

## Result

Shipment 118-S and PR #344 replace the older "blocking startup is impossible"
assumption with an admitted, bounded, and cleanup-safe startup contract.
