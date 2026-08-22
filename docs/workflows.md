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

## Run the test gate and prove coverage

`cargo dev-test` is the canonical local merge gate: a native, zero-setup,
cross-platform cargo command that runs every target under default features,
including the colocated `--lib` unit tests.

```bash
cargo dev-test          # every target + lib, default features (native alias)
cargo full-test         # plain cargo test, default features
cargo ci                # every target, all features (CI equivalent, exhaustive)
```

`cargo dev-test` and `cargo full-test` run under default features, so they skip
targets gated on non-default features (`git-graph`, `legacy-sse`,
`otlp-export`). `cargo ci` (all features) is the exhaustive backstop.

The coverage oracle (`scripts/test-coverage-oracle.sh` / `.ps1`) is the
measurable proof that the mandated contract and integration targets are covered.
It does not replace `cargo dev-test`; it audits coverage and offers an optional
faster local run.

```bash
bash scripts/test-coverage-oracle.sh  --mode report        # coverage audit
bash scripts/test-coverage-oracle.sh  --mode completeness  # manifest drift check
bash scripts/test-coverage-oracle.sh  --mode run           # optional fast run
pwsh scripts/test-coverage-oracle.ps1 --mode report        # Windows equivalents
```

The optional `--mode run` is change-scoped and concurrency-bounded: it runs only
the targets required by your current diff, passes each target's own
`required-features` so feature-gated targets execute, and also runs `--lib`. It
requires Bash 4+ (associative arrays); on macOS install a modern bash (the
bundled 3.2 is unsupported).
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

