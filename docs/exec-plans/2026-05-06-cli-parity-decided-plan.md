---
title: "CLI Parity Decided Plan"
description: "Decided implementation for full CLI subcommand surface mirroring all 18 MCP tools — shipped as 042-F / PR #85 / merge commit 53b432d"
status: shipped
feature: "042-F"
shipment: "026-S"
merge_commit: "53b432d"
source_plan: "docs/archive/plans/2026-05-06-cli-parity-plan.md"
closure: "docs/closure/2026-05-07-042-F-cli-parity-closure.md"
---

## Shipped Architecture

CLI subcommands communicate with the daemon via IPC (same shim transport). Output is
JSON-RPC 2.0 on non-TTY, human-readable text on TTY. `engram manifest` reads the
compile-time catalog directly — no daemon required.

### Module Layout

```
src/cli/
  mod.rs             — module root (pub)
  flags.rs           — GlobalFlags (workspace, id, json, format, quiet); all pub
  output.rs          — OutputFormatter (mode, quiet); from_flags(json, format, quiet)
  runner.rs          — run_tool(): IPC dispatch, canonicalize, exit codes
  commands/
    mod.rs
    manifest.rs      — run_manifest(): daemon-free, reads tools_catalog
    lifecycle.rs     — bind, daemon-status, workspace-status, flush
    indexing.rs      — sync, index
    search.rs        — search, query-memory, symbols, map-code, impact, query-graph
    report.rs        — stats, health, report (sub: health, symbols, graph)
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success — result present |
| 1 | Tool error — IpcResponse contains error |
| 2 | CLI/connection failure — no IpcResponse reached |

### Output Contract

- Non-TTY / `--json`: JSON-RPC 2.0 envelope `{"jsonrpc":"2.0","id":null,"result":...}`
- TTY: human-readable text
- `--format=[json\|text]`: explicit override (validated by clap; rejects other values)
- `--quiet`: suppresses stdout on success; errors always emit
- `--id <value>`: echoes caller-supplied request ID in the `id` field

### Key Decisions

1. **`pub` (not `pub(crate)`)** for all CLI functions — binary crate (`src/bin/engram.rs`) is
   separate from the library crate and cannot access `pub(crate)` items.
2. **`std::io::IsTerminal`** for TTY detection — `libc::isatty()` requires unsafe; stable Rust
   1.70+ `IsTerminal` works under `#![forbid(unsafe_code)]`.
3. **`shim::run(workspace_override: Option<&str>)`** instead of `std::env::set_var` —
   `set_var` is `unsafe fn` in Rust 2024 edition.
4. **Removed `workspace` from `Daemon` enum variant** — eliminates duplicate with `GlobalFlags`.
   `Daemon` reads `flags.workspace.clone().ok_or_else(...)`.
5. **`env!("CARGO_BIN_EXE_engram")`** in integration tests — Cargo sets this automatically for
   `[[bin]]` targets; more reliable than `current_exe()` path arithmetic.
6. **`value_parser = ["json", "text"]`** on `--format` — invalid values fail at clap parse time.
7. **`#[arg(long = "type")]`** not `#[arg(name = "type")]` — `name=` sets arg ID; `long=` sets
   the flag name.

### Compound Learnings Captured

- `docs/compound/best-practices/clap-long-vs-name-attribute-2026-05-07.md`
- `docs/compound/best-practices/rust-2024-set-var-unsafe-2026-05-07.md`
- `docs/compound/workflow-issues/ci-all-targets-stricter-than-local-2026-05-07.md`

### Follow-Up Stash

| Stash ID | Summary |
|----------|---------|
| B59D87CA | Integrate engram index/sync into start.ps1 for Copilot pre-loading |
| D5F04760 | Implement query-graph: replace stub with real graph query execution |
| 1620BAA6 | Add --quiet flag e2e test coverage in cli_e2e_test.rs |
| D391F5AF | Source stash entry — manual retirement needed |
