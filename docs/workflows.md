---
title: Engram Workflows Guide
description: Common Engram command sequences for setup, indexing, search, graph traversal, and diagnostics.
---

## Overview

Use this page as the task cookbook after the quickstart is done. Each workflow
starts from the current workspace root unless a command shows an explicit path.

## Bring a workspace online

```bash
engram install
engram bind
engram sync
```

That sequence installs workspace artifacts, binds the workspace, and ensures the
first index exists.

## Refresh the index

| Goal | Command | Notes |
|---|---|---|
| Normal refresh | `engram sync` | Incremental path through the daemon |
| Forced rebuild | `engram sync --full` | Full rebuild without changing the command shape |
| Alias for full rebuild | `engram index` | Same intent as `sync --full` |
| Prewarm without daemon lifecycle | `engram sync --direct` | Runs in-process and exits when complete |

Use `sync` as the routine command. Use `index` or `sync --full` when you want a
clean rebuild.

## Search by concept or symbol

```bash
engram search "workspace lifecycle" --format text
engram query-memory "release workflow" --format text
engram symbols --type function --prefix run_ --format text
```

Use `search` when you know the concept. Use `symbols` when you already know the
kind or name shape you want.

## Walk graph relationships

```bash
engram map-code run --depth 2 --format text
engram impact run --depth 2 --format text
engram query-graph --operation neighborhood --root fn:abc123 --max-depth 2 --format text
```

Use:

* `map-code` to inspect local callers, callees, and usages
* `impact` to estimate blast radius before a change
* `query-graph` when you need a structured path or neighborhood traversal

## Check health and delivery signals

```bash
engram daemon-status --format text
engram workspace-status --format text
engram health --format text
engram branch-metrics --format text
engram report token-savings --format text
```

These commands are the fastest way to confirm that the daemon is healthy, the
workspace is current, and usage telemetry is being recorded.

## Refresh or remove the installation

```bash
engram update
engram reinstall
engram uninstall --keep-data
```

Use `update` when you want fresh generated artifacts. Use `reinstall` when the
runtime directories need a clean rebuild. Use `uninstall --keep-data` when you
want to remove wiring without discarding the workspace data.

## Run the change-scoped test gate

`cargo dev-test` is the canonical local merge gate. It is change-scoped: rather
than a fixed allowlist of targets, it runs the test targets a coverage oracle
derives from your current diff, under an explicit concurrency bound. The
exhaustive `cargo ci` (all targets, all features) remains the backstop.

```bash
cargo dev-test          # change-scoped, bounded run for the current diff
cargo full-test         # every target, unbounded, DEFAULT features only
cargo ci                # every target, all features (CI equivalent, exhaustive)
```

`cargo full-test` runs every target but only under default features, so it skips
targets gated on non-default features (`git-graph`, `legacy-sse`,
`otlp-export`). `cargo ci` (all features) is the exhaustive backstop. The
change-scoped runner passes each selected target's own `required-features`, so
feature-gated targets execute rather than being silently skipped by cargo.

`cargo dev-test` delegates to the `cargo-devtest` external subcommand in
`scripts/`. On Linux/macOS, add `scripts/` to `PATH` once so cargo can find it.
On Windows, cargo does not resolve script-based external subcommands, so use the
wrapper `pwsh scripts/dev-test.ps1`. The shell oracle requires Bash 4+
(associative arrays); on macOS install a modern bash (its bundled 3.2 is
unsupported). Either way you can also invoke the oracle runner directly:

```bash
bash  scripts/test-coverage-oracle.sh  --mode run     # Linux/macOS
pwsh  scripts/dev-test.ps1                             # Windows (wrapper)
pwsh  scripts/test-coverage-oracle.ps1 --mode run     # Windows (direct)
```

### Read the coverage report

Run the oracle in `report` mode to see, for a diff, the required target set, the
selected set, and any omitted targets. The gate passes only when `omitted == 0`
and no source surface is unmapped:

```bash
bash scripts/test-coverage-oracle.sh --mode report --changed src/db/workspace.rs
```

| Field | Meaning |
|---|---|
| `REQUIRED_COUNT` | Targets the manifest requires for the changed surfaces |
| `SELECTED_COUNT` | Targets the run will actually execute |
| `OMITTED_COUNT` | Required targets not selected; must be `0` to pass |
| `UNMAPPED_COUNT` | Changed `src/` files with no manifest mapping; must be `0` |
| `STATUS` | `PASS` or `FAIL` |

### Add a mapping when you add a test target

The surface-to-target mapping lives in `.cargo/test-coverage-manifest.toml`. Each
`[[surface]]` maps a source path prefix to the test-target name globs that must
run when a file under that prefix changes. Map surfaces **broadly rather than
narrowly**: a shared module can affect targets not obviously named for it, and
diff-derived selection cannot see transitive effects, so list every target that
could plausibly exercise the surface. `cargo ci` remains the transitive backstop.

When you add a `[[test]]` target to `Cargo.toml`, make sure its name is covered
by a source surface (the tier globs `contract_*`, `integration_*`, `unit_*`,
`cold_*`, `helpers_*` cover the non-HCL tiers). The completeness check fails when
a target or a top-level `src/` module is unmapped, so the manifest cannot drift
silently:

```bash
bash scripts/test-coverage-oracle.sh --mode completeness
```

### Tune the concurrency bound

The process budget is an explicit parameter, not a consequence of the target
count. Defaults live in the manifest `[settings]` block
(`max_concurrent_test_binaries`, `test_threads`) and can be overridden per-run
via the `ENGRAM_DEVTEST_MAX_BINARIES` and `ENGRAM_DEVTEST_TEST_THREADS`
environment variables declared in `.cargo/config.toml`.

