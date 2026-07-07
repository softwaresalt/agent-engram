---
title: "078-F Implementation Plan — Daemon & Workspace Status Observability Fixes"
type: exec-plan
date: 2026-07-07
feature: 078-F
tasks: [078.001-T, 078.002-T]
shipment: 077-S
status: draft
---

# 078-F — Fix daemon and workspace status observability misreporting

Two observability defects surfaced while validating the freshly-rebuilt engram
workspace database. Both cause status tools to report values that mislead an
operator or agent. Neither affects indexing correctness — the underlying data is
sound — but both undermine trust in the status surface.

## Problem statements

### Bug 1 — `get_daemon_status.memory_bytes` reports system-wide RAM (078.001-T)

`src/tools/lifecycle.rs:429` computes daemon memory as:

```rust
let memory_bytes = sys.used_memory(); // sysinfo 0.30+ returns bytes
```

`sysinfo::System::used_memory()` returns **total system RAM in use**, not the
engram process's memory. On the validation host this reported **22.5 GB** while
the daemon's real working set was **1.23 GB** — an ~18× overstatement that
exactly matched machine-wide usage. This is the direct cause of an earlier
false alarm that "engram is using an awful lot of memory."

The correct per-process pattern already exists in the same crate at
`src/tools/read.rs:849-856` (`get_health_report`), which uses
`sys.refresh_process(pid)` + `process.memory()` and reports `memory_mb`
correctly (~1.25 GB).

A second instance exists in the legacy HTTP/SSE health handler
`src/server/router.rs:20`:

```rust
"memory_bytes": sys.used_memory() * 1024,
```

This is doubly wrong: system-wide **and** multiplied by 1024 on an
already-bytes value (sysinfo 0.30+ returns bytes).

### Bug 2 — `get_workspace_status.code_graph` always zero without `git-graph` (078.002-T)

`src/tools/lifecycle.rs:479-500` gates the code-graph counts behind
`#[cfg(feature = "git-graph")]`, with the `#[cfg(not(feature = "git-graph"))]`
branch returning `CodeGraphStats::default()` (all zeros). The comment asserts:

> Without git-graph, the code-graph indexer is inactive so the counts must stay zero.

This premise is **false**. The `git-graph` feature (`Cargo.toml:73 = ["git2"]`)
gates **git commit-history** tooling only (`query_changes`, `index_git_history`,
the `git_graph` service — see the cfg gates in `src/tools/mod.rs`,
`src/tools/read.rs`, `src/tools/write.rs`, `src/services/mod.rs`). The
**code-graph indexer** (`services::code_graph::sync_workspace`) is always
compiled and always runs. Live proof: a default-feature build indexed 271 code
files / 2,332 symbols / 3,670 edges, and `get_workspace_statistics`
(`src/tools/read.rs:95-103`, ungated) reports those counts correctly — while
`get_workspace_status` returns zeros for the same workspace.

Because the shipped binary uses default features (`embeddings, cozo-backend`,
no `git-graph`), **every** end user sees `code_graph: {0,0,0,0,0}` from
`get_workspace_status`, regardless of index state.

Why no test caught this:
- CI unit lane (`cargo dev-test` = `test --lib`) skips integration tests.
- CI integration lane (`ci.yml:80` = `--no-default-features --features cozo-backend,embeddings`) runs with git-graph OFF, so the gated block compiles out.
- `smoke_test.rs::s072_status_without_git_graph_feature` binds an **empty** temp
  dir and asserts `code_files == 0` — which passes trivially (no files) and so
  encodes the buggy premise without exercising a populated graph.
- `graph_vector_rehydration_test.rs` asserts non-zero `code_graph.functions`,
  but is `#[ignore]`d on Windows **and** Linux (CozoDB SQLITE_BUSY, stash
  `100EACD8`), so it never runs on CI.

## Fix design

### Bug 1

1. Add a shared helper `current_process_memory_bytes() -> Option<u64>` that
   returns the current process's resident memory in bytes, using the
   `read.rs` pattern (`get_current_pid` + `refresh_process` + `process.memory()`).
2. `get_daemon_status`: set `memory_bytes = current_process_memory_bytes().unwrap_or(0)`.
3. `get_health_report`: derive `memory_mb` from the same helper (`bytes / 1_048_576`),
   removing the duplicated inline logic.
4. `router.rs` health handler: use the helper; drop the `* 1024`.

Placement: helper lives in a small module reachable by both `tools` and
`server` without a cycle. `src/tools/lifecycle.rs` is imported by `server`
already; simplest is a `pub(crate) fn` in a new
`src/services/process_memory.rs` (services is a leaf dependency of both tools
and server). This avoids any tools↔server cycle.

### Bug 2

1. Remove the `#[cfg(feature = "git-graph")]` / `#[cfg(not(...))]` split in
   `get_workspace_status`; read the five counts unconditionally from the DB via
   `CodeGraphQueries`, mirroring `get_workspace_statistics`. Preserve the
   existing `connect_db` failure fallback to `CodeGraphStats::default()`.
2. Delete `s072_status_without_git_graph_feature` (false premise; asserts the bug).
   Update the sibling doc comment if needed.

## Test strategy (TDD — red first)

All new tests run under **default features** (the shipped configuration) so they
guard the real user-facing surface, and run **in-process** (no daemon spawn) to
avoid the SQLITE_BUSY daemon-restart ignore that sidelines
`graph_vector_rehydration_test.rs`.

### Bug 1 tests
- Unit (lib) test on `current_process_memory_bytes`: returns `Some(v)` with
  `v > 0`, and `v <= System::used_memory()` (a process is a subset of system
  usage). Fails to compile before the helper exists (red), passes after.
- Lib regression test calling `get_daemon_status(&state)`: assert
  `status.memory_bytes < System::new_with_memory().used_memory()`. Currently the
  two are identical (both `used_memory()`), so the assertion fails (red); after
  the fix the process memory is strictly less than system usage (green). Uses a
  generous invariant (process ⊂ system) rather than an exact value to avoid
  environment flakiness.

### Bug 2 tests
- Integration test (default features, in-process): create a temp workspace with a
  sample `.rs` file; `set_workspace`; read the bound `WorkspaceSnapshot`
  (`data_dir`, `branch` are `pub`); `code_graph::index_workspace` into that
  `data_dir`/`branch`; call `get_workspace_status`; assert
  `code_graph.functions > 0`, `classes > 0`, `edges > 0`, and that these equal
  the counts returned by `get_workspace_statistics` for the same workspace.
  Fails today (gated to zero under default features), passes after ungating.

## Constitution Check

- **I. Safety-First Rust**: No `unsafe`. Helper returns `Option`, callers use
  `unwrap_or(0)` (a total, non-panicking default — not `unwrap()`/`expect()`).
  Clippy pedantic clean.
- **II. Test-First (NON-NEGOTIABLE)**: Red tests written and observed to fail
  before implementation. New tests target the default (shipped) feature set.
- **III/IV. Workspace isolation**: No new filesystem surface; reads existing DB.
- **V. Observability**: This *is* an observability correctness fix.
- **VI. Single Responsibility**: No new dependencies (`sysinfo` already present).
- **X. Context efficiency**: Status tools stay lean; helper is O(1).
- **XI. Merge commits**: PR merged via merge commit.

Justified deviation: removing a shipped test (`s072`). Rationale: the test
encodes a factually incorrect invariant and only passes because it exercises an
empty workspace. It is replaced by a stronger positive test. Documented here and
surfaced to adversarial review.

## Adversarial review refinements (applied)

Rubber-duck review raised two blockers and refinements, all incorporated:

1. **Bug 1 regression assertion** must not compare against `used_memory()`
   (flaky; `System::new_with_memory` does not exist in sysinfo 0.30). Instead
   independently sample the current process memory and assert
   `status.memory_bytes.abs_diff(process_bytes) <= max(64 MiB, process_bytes/4)`.
   Red against system-wide RAM, green against process memory. Lives as a
   `#[cfg(test)]` lib test so it runs in every lane (`cargo dev-test` included).
2. **Bug 2 test must not race `set_workspace`'s background hydration**
   (`background_db_hydration` acquires `try_start_indexing`). Sequence: bind →
   poll `!state.is_indexing()` with a timeout → read `snapshot.data_dir/branch` →
   `code_graph::index_workspace` → `get_workspace_status`. Fixture includes a
   `struct` + `fn` + a call so `classes`, `functions`, and `edges` are all > 0.
3. **Rewrite `s072`** (don't delete): make it `uncfg` (runs under both default and
   all-features lanes) asserting non-zero code_graph after a real index. Preserves
   the no-`git-graph` scenario and reviewer trust.
4. **Remove stale `use sysinfo::System;`** imports left unused in `lifecycle.rs`,
   `read.rs`, `router.rs` after helper extraction. Validate with BOTH
   `--no-default-features --features cozo-backend,embeddings` and `--all-features`
   clippy (router.rs only compiles under `legacy-sse`, an all-features-only path).
5. **Log a warning** on the `connect_db` failure → `CodeGraphStats::default()`
   fallback in `get_workspace_status`, so a DB failure is no longer silently
   indistinguishable from an empty graph.

## Rollback

Pure reporting change, no data migration. Rollback = revert the merge commit.
No runtime state is affected; `.engram/` artifacts are unchanged.

## Risk / blast radius

- Low. Two functions + one legacy handler changed; response *shape* is unchanged
  (same JSON keys/types) — only the numeric values become correct.
- Consumers keying on `memory_bytes` now receive a smaller, correct number;
  no schema break.
- `get_workspace_status` gains a DB read on every call in the default build (it
  already did so under git-graph). Cost is five cheap COUNT queries; identical to
  `get_workspace_statistics`, which is already called freely.
