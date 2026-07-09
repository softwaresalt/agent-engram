---
title: "079-F Implementation Plan — --force flag for genuine full rebuild"
type: exec-plan
date: 2026-07-08
feature: 079-F
tasks: [079.001-T]
status: draft
---

# 079-F — Add `--force` flag to `sync`/`index` for a genuine full rebuild

## Problem

`engram sync --full` and `engram index` are documented to "force a complete
re-index," but both call the `index_workspace` tool with `force = false`
(`IndexWorkspaceParams.force` is `#[serde(default)]` = false, and the CLI sends
`None` params). `index_workspace_impl` then hash-skips every unchanged file
(`code_graph.rs:228` `if !force { skip }`). Result: on a workspace whose files
haven't changed, `engram index` reports `files_parsed: 0, files_skipped: N` —
it never rebuilds. The direct path (`run_direct_sync`) hardcodes `false` too
(`direct.rs:156`). There is currently **no way** to force a re-parse/re-embed of
unchanged files short of deleting `.engram/code-graph`.

This matters when the on-disk graph is stale or partial (e.g., call edges lost
after incremental re-syncs) and an operator wants to rebuild from source.

## Fix (additive, no behavior change to existing invocations)

Add a `--force` flag to the `sync` and `index` subcommands. When set, the CLI
sends `{"force": true}` to `index_workspace` (IPC path) and passes `force = true`
to `index_workspace_with_progress` (direct path). The existing default
(hash-skip) is unchanged, so no current script or automation regresses.

- `engram index --force` → full re-parse + re-embed of all files.
- `engram sync --force` → same (implies the full-index path).
- `engram index` / `engram sync --full` → unchanged (full walk, hash-skip).

Also correct the misleading `--full` doc comment (it does a full *walk*, not a
forced re-parse).

### Touch points
- `src/bin/engram.rs`: add `force: bool` (`#[arg(long)]`) to `Sync` and `Index`
  commands; thread into `run_sync`/`run_index`.
- `src/cli/commands/indexing.rs`: `run_sync(full, direct, force, …)` and
  `run_index(direct, force, …)`; take the full-index path when `full || force`;
  send `Some(json!({"force": true}))` when `force`, else `None`.
- `src/cli/direct.rs`: `run_direct_sync(workspace, full, force, …)` passes
  `force` to `index_workspace_with_progress`.

## Test strategy (TDD — red first)

Two subprocess behavioral tests in `tests/integration/cli_direct_test.rs`
(existing `run_direct` harness, isolated `ENGRAM_DATA_DIR`):

1. `direct_index_force_reparses_unchanged_files`: index → `files_parsed >= 1`;
   re-index → `files_parsed == 0` (hash-skip, existing behavior); `index --force`
   → `files_parsed >= 1` (re-parses).
2. `direct_sync_force_reparses_unchanged_files`: prime index; `sync --full` →
   `files_parsed == 0` (proves `--full` alone still hash-skips); `sync --force`
   → `files_parsed >= 1` (proves the `full || force` routing sends `sync --force`
   down the forced full-index path).

Red before the fix: `--force` is an unknown clap flag → non-zero exit → the
`--force` assertions fail. Green after wiring the flag.

## Adversarial review refinements (applied)

Rubber-duck review (no blockers) — incorporated:
- Direct path branches on `full || force` (not just `full`) so `sync --force
  --direct` cannot silently fall through to incremental sync.
- Added the `sync --force` test so coverage isn't limited to `index --force`.
- Docs describe three precise modes (incremental / full-scan hash-skip / force);
  wording avoids "complete rebuild from zero" — `--force` re-parses *discovered*
  files, it does not drop unrelated stale state.

## Constitution check
- Safety-First Rust: no `unsafe`; no `unwrap`/`expect` in production (test uses
  `expect`/`panic!`, which is allowed in tests). Clippy pedantic clean.
- Test-First: red subprocess test observed to fail before implementation.
- Single Responsibility: no new dependencies; reuses existing `force` plumbing.

## Risk / rollback
Purely additive CLI flag. Existing `sync`/`index`/`--full` behavior unchanged.
Rollback = revert the merge commit.
