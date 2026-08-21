---
title: Stage session memory — dark-factory batch 870B1AFF/568B257C/C2413934/DE460A88
date: 2026-08-21
type: session-memory
agent: stage
batch: dark-factory-20260820-870b1aff-568b257c-c2413934-de460a88
---

## Scope

Full Stage pipeline in dark-factory mode for exactly four operator-selected
stash entries, in operator-specified order. Stage work only: no shipment claim,
no production implementation, no build execution, no PR.

## Branch and Worktree

* Branch: `stage/dark-factory-20260820-batch4`
* Worktree: `C:\Source\GitHub\engram\.worktrees\stage-dark-factory-20260820-batch4`
* Created from `origin/main` at `6f50f0e435161705a581f8ee3817358c8607964c`
* The dirty root worktree at `C:\Source\GitHub\engram` was never touched.

## Tool Status

| Tool | Status |
|---|---|
| backlogit CLI v1.10.0 | TOOL_OK (registry-backed, scoped to this worktree) |
| backlogit index sync | INDEX_SYNC_OK at session start; TOOL_DEGRADED afterwards |
| Engram CLI | TOOL_DEGRADED — cannot bind worktrees whose `.git` is a file |
| Engram MCP | Not used, per session requirement |

Engram CLI failed twice as predicted: `engram daemon-status` in the clean read
worktree returned `daemon unavailable: Daemon failed to reach Ready state within
30000ms`; `engram workspace-status` in the Stage worktree returned
`cannot compute IPC endpoint: Path '...' is not a Git repository root`. Retried
once per protocol, then fell back to targeted file reads and git history. No
broad scans, no MCP attempt. Both failures became primary evidence for 870B1AFF.

`backlogit sync` fails deterministically on 19 pre-existing malformed artifacts
inherited from `origin/main` (`.backlogit/archive/029.004*`–`029.009-T` and
`.backlogit/queue/030.005-C.md`, artifact type `chore` with `-C` suffix). None
were modified by this session. Consequence: the disposable SQLite cache is stale,
so `backlogit query` must not be trusted for `custom_fields`. The Markdown files
remain the source of truth and `backlogit get` reads them authoritatively.

## Release Units Produced

| Order | Stash | Feature | Tasks | Shipment | Status |
|---|---|---|---|---|---|
| 1 | 870B1AFF | 124-F | 124.001-T … 124.007-T | 120-S | queued |
| 2 | 568B257C | 125-F | 125.001-T … 125.008-T | 121-S | queued |
| 3 | C2413934 | 126-F | 126.001-T … 126.008-T | 122-S | queued |
| 4 | DE460A88 | 127-F | 127.001-T … 127.008-T | 123-S | queued |

Four separate release units, four separate shipments. No cross-domain merging.

## Key Findings

### 870B1AFF — root cause is pre-initialize child exit, not Tokio

`src/shim/mod.rs::run` evaluates `canonicalize_workspace`,
`ensure_daemon_running`, and `ipc_endpoint` before `transport::run_shim` binds
the stdio transport. Any `Err` propagates through `main` and kills the process
while the client is writing `initialize`, producing Windows `os error 232`. The
client's `tokio::process::ChildStdin`/`ChildStdout` naming is a downstream
artifact. Both failure classes were reproduced live with the installed CLI. The
installed binary (`0.2.0+g6268c1ac`) predates the 122-F worktree fix
(`08676d34`), which is the immediate trigger. Latent second hazard:
`src/lib.rs::init_tracing` uses `fmt::layer()`, whose default writer is stdout —
the MCP framing channel.

### 568B257C — TOCTOU across ~20 path-based resolutions

`resolve_git_metadata` re-walks the full path from the filesystem root roughly
twenty times. `read_metadata_file` is the sharpest instance: `symlink_metadata`
then `read_to_string` on the same path. `canonical_path` follows symlinks while
`require_plain_directory` does not, so validation and use use different
semantics at different instants. Correction made during review: Rust's Windows
`is_symlink()` *does* cover `MOUNT_POINT` (junctions); the real gap is reparse
tags outside `SYMLINK`/`MOUNT_POINT`. The file already applies the broader
`FILE_ATTRIBUTE_REPARSE_POINT` test to `.workspace-id` but not to the Git
metadata chain.

### C2413934 — the gate covers 6 of 208 targets

`.cargo/config.toml` defines `dev-test` as `--lib` plus six HCL targets.
`Cargo.toml` declares 208 `[[test]]` targets, so 202 non-HCL contract and
integration targets never run under the constitutionally mandated gate. The
narrowing arrived with the 117-S HCL stream (`d6db8423`, `2b677646`). Direction
is a measurable oracle (required / selected / omitted, pass when `omitted == 0`)
over a declared manifest, not blind broadening.

### DE460A88 — the oracle reads from its own subject

`tests/contract/tools_catalog_test.rs` imports `engram::shim::tools_catalog` and
calls `all_tools()` in every assertion. Only names are independently declared,
and only as presence/absence. Descriptions and schemas have no oracle, and
observation is against in-process structs rather than the serialized `tools/list`
an agent receives.

## Gates

All four plans passed `plan-review`. Hardening applied to plans 1–3; correctly
declined for plan 4 (no hardening signal). One P0 (568B257C, handle-derived
metadata) and nine P1 findings were raised and all resolved before their gate
decisions. Review-fix cycles: 1, 2, 1, 1 — all within the limit of 3.

Adversarial multi-model review is mandated for 125.004-T, 125.005-T, and
125.006-T.

## Tool Gaps Recorded

1. `backlogit shipment create` / `shipment add` / `update` expose no
   custom-field flag. Arbitrary `custom_fields` require the documented direct
   manifest edit followed by rehydration.
2. `backlogit shipment get` projects only the known `items` key and silently
   drops other `custom_fields`. `backlogit get {id}` is the authoritative
   surface for operator batch fields.
3. `backlogit sync` has no parse-failure tolerance flag and hard-fails on the
   19 pre-existing malformed artifacts, leaving the SQLite cache stale.

## Next Steps for Ship

Claim `120-S` first, then `121-S`, `122-S`, `123-S` strictly in
`operator_order`, honoring `operator_predecessors`. Every unit is TDD
harness-before-code: RED tasks must complete before their paired GREEN tasks.
Do not claim any shipment out of order.

## Not Done (deliberately)

No shipment claimed. No production code written. No build, test, or lint run.
No PR. No merge. The other 16 stash entries were not touched.
