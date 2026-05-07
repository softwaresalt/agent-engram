# Session Memory: 042-F CLI Parity — Post-Merge Closure

**Date**: 2026-05-07  
**Feature**: 042-F — CLI Parity for MCP Tool Operations  
**Shipment**: 026-S  
**Phase**: Post-merge closure (Step 6 complete)  
**Merge commit**: `53b432d` on `main`  
**Closure branch**: `post-merge/042-F-cli-parity`

---

## Items Completed

- [x] 042.001-T — CLI module scaffold, GlobalFlags, OutputFormatter
- [x] 042.002-T — IPC runner and exit code mapping
- [x] 042.003-T — Manifest subcommand (daemon-free)
- [x] 042.004-T — Lifecycle subcommands (bind, daemon-status, workspace-status, flush)
- [x] 042.005-T — Indexing subcommands (sync, index)
- [x] 042.006-T — Search/query subcommands (6 commands)
- [x] 042.007-T — Report/diagnostics subcommands (6 commands)
- [x] 042.008-T — Binary integration + parser tests
- [x] 042.009-T — End-to-end CLI integration tests
- [x] 042.010-T — Agent fallback documentation

## Files Modified (feature PR #85)

- `src/bin/engram.rs` — 17 new Command variants, GlobalFlags, ReportCommand
- `src/cli/mod.rs`, `src/cli/flags.rs`, `src/cli/output.rs`, `src/cli/runner.rs`
- `src/cli/commands/{mod,manifest,lifecycle,indexing,search,report}.rs`
- `src/shim/mod.rs` — `run(workspace_override: Option<&str>)` signature
- `tests/unit/cli_parser_test.rs`, `tests/integration/cli_e2e_test.rs`
- `docs/architecture.md` — CLI Architecture section
- `.github/instructions/agent-engram.instructions.md` — Agent fallback protocol

## Post-Merge Closure (this branch)

- [x] Archive commit: `104332b` — 026-S shipment + 042-F items moved to `.backlogit/archive/`
- [x] Closure artifact: `docs/closure/2026-05-07-042-F-cli-parity-closure.md`
- [ ] Compound learnings (in progress)
- [ ] compact-context (in progress)

## Key Decisions Made

1. **`pub` instead of `pub(crate)`** for CLI functions — binary and lib are separate crates
2. **`std::io::IsTerminal`** for TTY detection — avoids `libc::isatty()` and unsafe code
3. **`shim::run(workspace_override)`** instead of `std::env::set_var` — `set_var` is unsafe in Rust 2024
4. **`value_parser = ["json", "text"]`** on `--format` — clap-enforced at parse time
5. **Removed `workspace` from `Daemon` variant** — eliminates duplicate global-flag conflict
6. **`env!("CARGO_BIN_EXE_engram")`** in integration tests — reliable binary path

## Technical Gotchas Captured

- `#[arg(long, name = "type")]` in clap v4 sets arg ID, not long flag name → use `#[arg(long = "type")]`
- `std::env::set_var` is unsafe in Rust 2024; prefer function signature for workspace override
- CI uses `--all-targets --features cozo-backend,embeddings` (stricter than local); always test with this
- `dedup()` without prior `sort_unstable()` only removes consecutive duplicates
- Floating doc comments in test modules fail clippy under `--all-targets`

## PR Review Rounds

- Round 1: 7 comments → commit `4dca15a`
- CI fix: 3 clippy errors → commit `d41966d`
- Round 2: 8 comments → commit `5b7d832`

## Follow-Up Items (stashed)

1. `start.ps1` integration — add `engram index` before Copilot launch
2. `query-graph` implementation (stub currently)
3. Stash D391F5AF manual retirement
4. `--quiet` e2e test coverage

## Next Steps

- Create closure PR for `post-merge/042-F-cli-parity` branch
- After closure PR merge, session is complete for 026-S
